#!/usr/bin/env python3
"""Decompile a build's classes into git-ignored `_reference/decompile/`.

Adapted from the gothic-mobile sibling (tools/java/decompile.py). Generated
decompiler output is EVIDENCE, never hand-edited source — it lives under the
ignored `_reference/` tree and is NEVER committed (rulebook R1: derived from
copyrighted bytecode).

Two independent decompilers are run — jadx (primary, readable) and cfr
(cross-check) — so a suspicious jadx rendering can be checked against cfr and,
ultimately, `javap -c -p` bytecode.

The build defaults to `builds.toml`'s `baseline`; pass an id (or a bare jar path)
to override. The payload is located from its `containers` (`_originals/<file>`);
the immutable `_originals/` is never modified.

Output tree:
    _reference/decompile/<resolution-or-id>/jadx/
    _reference/decompile/<resolution-or-id>/cfr/

Usage:
    decompile.py                       # baseline from builds.toml
    decompile.py <build-id>
    decompile.py --jar _originals/x.jar --out-name 176x220
"""
from __future__ import annotations

import argparse
import shutil
import subprocess
import sys
from pathlib import Path

try:
    import tomllib
except ModuleNotFoundError:  # pragma: no cover
    import tomli as tomllib  # type: ignore

REPO = Path(__file__).resolve().parents[2]
BUILDS = REPO / "java" / "reconstruction" / "builds.toml"
ORIGINALS = REPO / "_originals"
OUT_ROOT = REPO / "_reference" / "decompile"


def load() -> dict:
    with BUILDS.open("rb") as fh:
        return tomllib.load(fh)


def find_entry(manifest: dict, build_id: str) -> dict:
    for entry in manifest.get("payload", []) + manifest.get("archived", []):
        if entry["id"] == build_id:
            return entry
    sys.exit(f"decompile: build id not found in builds.toml: {build_id}")


def jar_for(entry: dict) -> Path:
    for c in entry.get("containers", []):
        c = c.strip()
        if c.startswith("_originals/"):
            p = REPO / c
            if p.is_file():
                return p
    sys.exit(f"decompile: could not locate jar for {entry['id']}")


def decompile(jar: Path, out_dir: Path) -> None:
    if out_dir.exists():
        shutil.rmtree(out_dir)
    (out_dir / "jadx").mkdir(parents=True)
    (out_dir / "cfr").mkdir(parents=True)
    print(f"decompile: {jar.name} ({jar.stat().st_size} bytes) -> {out_dir}")

    jadx = shutil.which("jadx")
    if jadx:
        subprocess.run(
            [jadx, "--no-res", "--no-imports", "-d", str(out_dir / "jadx"), str(jar)],
            check=False,
        )
        n = len(list((out_dir / "jadx").rglob("*.java")))
        print(f"  jadx: {n} .java files -> {out_dir / 'jadx'}")
    else:
        print("  jadx: not on PATH (run inside `nix develop`)")

    cfr = shutil.which("cfr")
    if cfr:
        res = subprocess.run(
            [cfr, str(jar), "--outputdir", str(out_dir / "cfr")],
            check=False, capture_output=True, text=True,
        )
        n = len(list((out_dir / "cfr").rglob("*.java")))
        print(f"  cfr: {n} .java files -> {out_dir / 'cfr'}")
        if res.returncode != 0 and n == 0:
            print(res.stderr[-500:])
    else:
        print("  cfr: not on PATH (run inside `nix develop`)")


def main(argv: list[str]) -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("build_id", nargs="?", help="builds.toml payload/archived id")
    ap.add_argument("--jar", help="decompile a bare jar path instead of a build id")
    ap.add_argument("--out-name", help="output subdir name (default: resolution or id)")
    args = ap.parse_args(argv)

    if args.jar:
        jar = Path(args.jar)
        if not jar.is_absolute():
            jar = REPO / jar
        name = args.out_name or jar.stem
        decompile(jar, OUT_ROOT / name)
        return 0

    manifest = load()
    build_id = args.build_id or manifest.get("baseline")
    if not build_id:
        sys.exit("decompile: no build id and no baseline in builds.toml")
    entry = find_entry(manifest, build_id)
    jar = jar_for(entry)
    name = args.out_name or entry.get("resolution") or build_id
    decompile(jar, OUT_ROOT / name)
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
