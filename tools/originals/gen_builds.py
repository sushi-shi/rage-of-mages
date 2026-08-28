#!/usr/bin/env python3
"""Generate a `builds.toml` provenance authority from a resources directory.

Scans the surviving distribution files for ONE game (filtered by a filename
substring), deduplicates every JAR payload BY SHA256 across the top-level jars
AND the jars nested inside the zips, and emits one `[[payload]]` row per unique
payload — with sha256, exact byte length, every `collected_as` alias, its
`containers` (top-level and/or `inside <zip>`), device/resolution + MIDlet
metadata, and a 2D/3D probe. Zip wrappers become `[[container]]` rows; JADs
become `kind = "jad"` payload rows.

This fills in the MECHANICAL facts only (hashes, sizes, containers, manifest
strings, filename-derived device). The JUDGMENT calls — official vs fan repack
(`[[archived]]`), the true shipped language (from decoded content, not the
filename — playbook R10), and the baseline/naming-reference choice — are left
flagged for a human to reconcile in Phase 1. Nothing here is committed as a
semantic fact without evidence.

READ-ONLY over the resources directory. Never modifies an original.
"""
from __future__ import annotations

import argparse
import hashlib
import io
import re
import sys
import zipfile
from pathlib import Path
from collections import defaultdict

M3G_RE = re.compile(rb"javax/microedition/m3g|Graphics3D|VertexBuffer")
RES_RE = re.compile(r"(\d{2,4})\s*[xX]\s*(\d{3,4})")
REPACK_RE = re.compile(
    r"\bby\s+\w|andrew-lviv|konon|allmobi|tegos|playfon|sket4er|mathla|lavita",
    re.IGNORECASE,
)
MANIFEST_KEYS = ("MIDlet-Name", "MIDlet-Version", "MIDlet-Vendor",
                 "MicroEdition-Configuration", "MicroEdition-Profile")


def sha256(b: bytes) -> str:
    return hashlib.sha256(b).hexdigest()


def parse_manifest(text: str) -> dict:
    out = {}
    for line in text.splitlines():
        if ":" in line:
            k, v = line.split(":", 1)
            k, v = k.strip(), v.strip()
            if k in MANIFEST_KEYS or k in ("MIDlet-Jar-Size",):
                out[k] = v
    return out


def probe_jar(blob: bytes) -> dict:
    d = {"bytes": len(blob), "sha256": sha256(blob), "classes": None,
         "is_3d": None, "manifest": {}, "formats": {}}
    try:
        zf = zipfile.ZipFile(io.BytesIO(blob))
    except Exception as e:
        d["error"] = f"badzip: {e}"
        return d
    names = zf.namelist()
    classes = [n for n in names if n.endswith(".class")]
    d["classes"] = len(classes)
    m3g = 0
    for c in classes:
        try:
            if M3G_RE.search(zf.read(c)):
                m3g += 1
        except Exception:
            pass
    d["is_3d"] = m3g > 0
    exts = defaultdict(int)
    for n in names:
        if n.endswith("/"):
            continue
        base = n.rsplit("/", 1)[-1]
        ext = base.rsplit(".", 1)[-1].lower() if "." in base else "noext"
        exts[ext] += 1
    d["formats"] = dict(sorted(exts.items(), key=lambda kv: -kv[1]))
    for n in names:
        if n.upper().endswith("MANIFEST.MF"):
            try:
                d["manifest"] = parse_manifest(zf.read(n).decode("utf-8", "replace"))
            except Exception:
                pass
            break
    return d


def declared_language(name: str) -> str:
    m = re.search(r"_(EN|RU|ZH|DE|PL|FR|ES|IT)_", name)
    if m:
        return m.group(1).lower()
    for tok in ("_EN", "_RU", "_ZH", "-EN", "-RU"):
        if tok in name.upper():
            return tok[-2:].lower()
    return "unknown"


def resolution(name: str) -> str | None:
    m = RES_RE.search(name)
    return f"{m.group(1)}x{m.group(2)}" if m else None


def device(name: str) -> str | None:
    # A trailing "(Device Name)" or "-Device-Name" hint from the filename.
    m = re.search(r"\(([^()]*(?:Nokia|Samsung|Sony|SonyEricsson|LG|Motorola|"
                  r"Lenovo|Alcatel)[^()]*)\)", name)
    if m:
        return m.group(1).strip()
    m = re.search(r"-(Nokia|Samsung|Sony-?Ericsson|Lenovo|Motorola|LG)-([A-Za-z0-9]+)",
                  name)
    return f"{m.group(1)} {m.group(2)}" if m else None


def toml_str(s) -> str:
    return '"' + str(s).replace("\\", "\\\\").replace('"', '\\"') + '"'


def toml_list(xs) -> str:
    return "[" + ", ".join(toml_str(x) for x in xs) + "]"


def slugify_id(name: str, lang: str, res: str | None, seq: int) -> str:
    base = re.sub(r"[^a-z0-9]+", "_", Path(name).stem.lower()).strip("_")
    parts = [base]
    if res:
        parts.append(res)
    return "_".join(parts)[:60] + f"_{seq:02d}"


def main(argv: list[str]) -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--resources", required=True, help="dir of surviving builds")
    ap.add_argument("--match", required=True, help="filename substring filter")
    ap.add_argument("--slug", required=True)
    ap.add_argument("--title", required=True)
    ap.add_argument("--out", required=True, help="builds.toml path to write")
    args = ap.parse_args(argv)

    src = Path(args.resources).expanduser().resolve()
    if (src / "originals").is_dir():
        src = src / "originals"
    # --match may be a comma-separated list; a file matches if ANY pattern is a
    # substring of its name (so one merged game can span several filename stems,
    # e.g. the Allods/Rage-of-Mages region variants of one Nival codebase).
    patterns = [m.strip().lower() for m in args.match.split(",") if m.strip()]
    files = sorted(f for f in src.iterdir()
                   if f.is_file() and any(m in f.name.lower() for m in patterns))
    if not files:
        sys.exit(f"gen_builds: no files in {src} match {patterns!r}")

    # Unique jar payloads keyed by sha: aggregate names + container refs.
    payloads: dict[str, dict] = {}
    jads: dict[str, dict] = {}
    containers: list[dict] = []

    def add_jar(blob: bytes, collected: str, container_ref: str):
        info = probe_jar(blob)
        sha = info["sha256"]
        p = payloads.setdefault(sha, {**info, "collected_as": [], "containers": []})
        if collected not in p["collected_as"]:
            p["collected_as"].append(collected)
        if container_ref not in p["containers"]:
            p["containers"].append(container_ref)

    for f in files:
        blob = f.read_bytes()
        low = f.name.lower()
        if low.endswith(".jar"):
            add_jar(blob, f.name, f"_originals/{f.name}")
        elif low.endswith(".jad"):
            sha = sha256(blob)
            jd = jads.setdefault(sha, {"sha256": sha, "bytes": len(blob),
                                       "collected_as": [],
                                       "containers": [f"_originals/{f.name}"],
                                       "manifest": parse_manifest(
                                           blob.decode("utf-8", "replace"))})
            if f.name not in jd["collected_as"]:
                jd["collected_as"].append(f.name)
        elif low.endswith(".zip"):
            holds = []
            try:
                zf = zipfile.ZipFile(io.BytesIO(blob))
                for n in zf.namelist():
                    if n.endswith("/"):
                        continue
                    mb = zf.read(n)
                    holds.append(sha256(mb))
                    if n.lower().endswith(".jar"):
                        add_jar(mb, f"{f.name}::{n}", f"inside _originals/{f.name}")
                    elif n.lower().endswith(".jad"):
                        s = sha256(mb)
                        jd = jads.setdefault(s, {
                            "sha256": s, "bytes": len(mb), "collected_as": [],
                            "containers": [f"inside _originals/{f.name}"],
                            "manifest": parse_manifest(mb.decode("utf-8", "replace"))})
                        nm = f"{f.name}::{n}"
                        if nm not in jd["collected_as"]:
                            jd["collected_as"].append(nm)
            except zipfile.BadZipFile:
                pass
            containers.append({"name": f.name, "sha256": sha256(blob),
                               "bytes": len(blob), "holds": holds})

    # Emit deterministic TOML.
    lines: list[str] = []
    lines.append(f"# Provenance authority for the {args.title} (J2ME) corpus.")
    lines.append("#")
    lines.append("# AUTO-GENERATED by tools/originals/gen_builds.py — the MECHANICAL")
    lines.append("# facts (sha256, bytes, containers, manifest strings, filename-derived")
    lines.append("# device/resolution) are trustworthy. The flagged JUDGMENT calls are NOT")
    lines.append("# yet made: reconcile in Phase 1 and edit this file by hand thereafter.")
    lines.append("#   - official vs fan repack: move fan builds to [[archived]] "
                 "(see `repack_tag`).")
    lines.append("#   - true shipped language: decode the string tables; the filename "
                 "and MIDlet-Name lie (playbook R10). `declared_language` is a HINT only.")
    lines.append("#   - baseline / naming_reference: choose from Phase 1 fingerprints.")
    lines.append("#")
    lines.append("# `just originals-verify` reconciles every materialized payload "
                 "against the")
    lines.append("# sha256 + bytes table below; `--self-test` proves the gate can fail.")
    lines.append("")
    n3d = sum(1 for p in payloads.values() if p.get("is_3d"))
    fork = "2d" if n3d == 0 else ("3d" if n3d == len(payloads) else "mixed")
    lines.append(f"title = {toml_str(args.title)}")
    lines.append(f"slug = {toml_str(args.slug)}")
    lines.append(f"fork = {toml_str(fork)}   # 2d = Graphics2D (gothic path); "
                 f"3d = M3G (stalker path)")
    lines.append('baseline = ""          # TODO Phase 1: pick the quality baseline id')
    lines.append('naming_reference = ""  # TODO Phase 1: pick the semantic-naming id')
    lines.append(f"# unique jar payloads: {len(payloads)}; jads: {len(jads)}; "
                 f"zip containers: {len(containers)}; 3D payloads: {n3d}")
    lines.append("")

    def sortkey(item):
        sha, p = item
        return (declared_language(p["collected_as"][0]),
                resolution(p["collected_as"][0]) or "", p["bytes"], sha)

    seq = 0
    for sha, p in sorted(payloads.items(), key=sortkey):
        seq += 1
        name0 = p["collected_as"][0]
        lang = declared_language(name0)
        res = resolution(name0)
        dev = device(name0)
        mf = p.get("manifest", {})
        repack = ""
        hay = name0 + " " + mf.get("MIDlet-Name", "")
        m = REPACK_RE.search(hay)
        if m:
            repack = m.group(0)
        lines.append("[[payload]]")
        lines.append(f"id = {toml_str(slugify_id(name0, lang, res, seq))}")
        lines.append(f"sha256 = {toml_str(sha)}")
        lines.append(f"bytes = {p['bytes']}")
        lines.append('kind = "jar"')
        lines.append(f"is_3d = {'true' if p.get('is_3d') else 'false'}")
        lines.append(f"class_count = {p.get('classes', 0)}")
        if mf.get("MIDlet-Name"):
            lines.append(f"midlet_name = {toml_str(mf['MIDlet-Name'])}")
        if mf.get("MIDlet-Version"):
            lines.append(f"midlet_version = {toml_str(mf['MIDlet-Version'])}")
        if mf.get("MIDlet-Vendor"):
            lines.append(f"vendor = {toml_str(mf['MIDlet-Vendor'])}")
        if mf.get("MicroEdition-Configuration"):
            lines.append(f"cldc = {toml_str(mf['MicroEdition-Configuration'])}")
        if res:
            lines.append(f"resolution = {toml_str(res)}")
        if dev:
            lines.append(f"device = {toml_str(dev)}")
        lines.append(f"declared_language = {toml_str(lang)}   "
                     f"# HINT from filename/manifest; verify from content (R10)")
        lines.append("official = true   # TODO Phase 1: set false + move to "
                     "[[archived]] if a fan repack")
        if repack:
            lines.append(f"repack_tag = {toml_str(repack)}   "
                         f"# fan-repack signature detected — likely [[archived]]")
        lines.append(f"collected_as = {toml_list(p['collected_as'])}")
        lines.append(f"containers = {toml_list(p['containers'])}")
        if len(p["collected_as"]) > 1:
            lines.append('notes = "Meaningful aliases: same payload under '
                         'multiple collected names (kept, playbook R2)."')
        lines.append("")

    for sha, jd in sorted(jads.items(), key=lambda kv: kv[1]["collected_as"][0]):
        mf = jd.get("manifest", {})
        lines.append("[[payload]]")
        lines.append(f"id = {toml_str(re.sub(r'[^a-z0-9]+','_',Path(jd['collected_as'][0]).stem.lower()).strip('_')[:56] + '_jad')}")
        lines.append(f"sha256 = {toml_str(sha)}")
        lines.append(f"bytes = {jd['bytes']}")
        lines.append('kind = "jad"')
        if mf.get("MIDlet-Version"):
            lines.append(f"midlet_version = {toml_str(mf['MIDlet-Version'])}")
        lines.append(f"collected_as = {toml_list(jd['collected_as'])}")
        lines.append(f"containers = {toml_list(jd['containers'])}")
        lines.append('notes = "JAD descriptor (text metadata; carries no bytecode)."')
        lines.append("")

    for c in sorted(containers, key=lambda c: c["name"]):
        lines.append("[[container]]")
        lines.append(f"id = {toml_str(re.sub(r'[^a-z0-9]+','_',Path(c['name']).stem.lower()).strip('_')[:56])}")
        lines.append(f"sha256 = {toml_str(c['sha256'])}")
        lines.append(f"bytes = {c['bytes']}")
        lines.append(f"collected_as = {toml_list([c['name']])}")
        lines.append(f"holds = {toml_list(c['holds'])}")
        lines.append('notes = "Immutable zip distribution wrapper (preserved for provenance)."')
        lines.append("")

    out = Path(args.out)
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text("\n".join(lines) + "\n")
    print(f"gen_builds: wrote {out} — {len(payloads)} payloads, {len(jads)} jads, "
          f"{len(containers)} containers (fork={fork}).")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
