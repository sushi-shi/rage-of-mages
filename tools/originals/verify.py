#!/usr/bin/env python3
"""Reconcile the materialized `_originals/` against `builds.toml`.

`builds.toml` (playbook R2) is the single tracked owner of provenance; the binary
payloads are never committed. This gate proves that whatever sits in `_originals/`
is byte-for-byte the corpus we recorded — no corruption, no unknown file, nothing
missing. It verifies:

  * every top-level file in `_originals/` hashes to a known payload/container;
  * every nested payload and every JAD extracts byte-exactly from its container;
  * every top-level payload and every container is present.

Any mismatch exits non-zero (so it works as a CI gate). Run with `--self-test`
to prove the gate can fail (playbook R3): it corrupts one payload byte in memory
and confirms detection — robust to any corpus shape (top-level-only or nested).

Game-neutral: reads `game.toml`; works for every scaffolded port.
"""
from __future__ import annotations

import io
import sys
import zipfile
from pathlib import Path

from corpus_common import load_builds, repo_root, sha256_bytes

ROOT = repo_root(Path(__file__).parent)
ORIGINALS = ROOT / "_originals"


def top_level_name(container_ref: str) -> str | None:
    ref = container_ref.strip()
    prefix = "_originals/"
    return ref[len(prefix):] if ref.startswith(prefix) else None


def container_file(container_ref: str) -> str | None:
    ref = container_ref.strip()
    marker = "inside _originals/"
    return ref[len(marker):] if ref.startswith(marker) else None


def verify(manifest: dict, *, corrupt: str | None = None) -> list[str]:
    """Return a list of problems (empty == corpus verified).

    `corrupt` is a sha256 whose first byte is flipped wherever those bytes are
    hashed, used by --self-test to prove the gate detects tampering.
    """
    problems: list[str] = []
    payloads = manifest.get("payload", []) + manifest.get("archived", [])
    containers = manifest.get("container", [])

    known: dict[str, tuple[int, str]] = {}
    for entry in payloads + containers:
        known[entry["sha256"]] = (entry.get("bytes", -1), entry["id"])

    def digest(data: bytes, sha_expected: str) -> tuple[str, int]:
        if corrupt is not None and sha_expected == corrupt and data:
            data = bytes([data[0] ^ 0xFF]) + data[1:]
        return sha256_bytes(data), len(data)

    if not ORIGINALS.is_dir():
        return [f"_originals/ is not a directory: {ORIGINALS} "
                f"(materialize it with `python3 tools/originals/fetch.py <source>`)"]
    if ORIGINALS.is_symlink():
        problems.append("_originals/ is a symlink; it must be a real directory "
                        "(playbook R2 — symlink hazard).")

    # 1) every physically-present file must be a known payload/container.
    present: dict[str, Path] = {}
    for path in sorted(ORIGINALS.iterdir()):
        if not path.is_file():
            continue
        clean_sha = sha256_bytes(path.read_bytes())
        # Simulate a one-byte corruption of the target for --self-test.
        sha, nbytes = digest(path.read_bytes(), clean_sha) if corrupt == clean_sha \
            else (clean_sha, path.stat().st_size)
        if sha not in known:
            problems.append(f"unknown/corrupt file in _originals (not in "
                            f"builds.toml): {path.name} sha256={sha[:12]}")
            continue
        exp_bytes, ident = known[sha]
        if exp_bytes != -1 and exp_bytes != nbytes:
            problems.append(f"{path.name}: bytes {nbytes} != builds.toml "
                            f"{exp_bytes} ({ident})")
        present[sha] = path

    # 2) coverage: every top-level payload/container must be present.
    for entry in payloads:
        tops = [t for t in (top_level_name(c) for c in entry.get("containers", [])) if t]
        if not tops:
            continue  # nested-only payload, checked in step 3
        if entry["sha256"] not in present:
            problems.append(f"missing top-level payload {entry['id']} "
                            f"(sha {entry['sha256'][:12]})")
    for entry in containers:
        if entry["sha256"] not in present:
            problems.append(f"missing container {entry['id']} "
                            f"(sha {entry['sha256'][:12]})")

    # 3) nested payloads + JADs: extract from their container and verify.
    container_by_name: dict[str, bytes] = {}
    for entry in containers:
        p = present.get(entry["sha256"])
        if p is not None:
            container_by_name[Path(entry["collected_as"][0]).name] = p.read_bytes()

    for entry in payloads:
        nested = [c for c in (container_file(x) for x in entry.get("containers", [])) if c]
        if not nested or entry["sha256"] in present:
            continue  # not nested, or also present top-level (already checked)
        found = False
        for zip_name in nested:
            blob = container_by_name.get(zip_name)
            if blob is None:
                continue
            try:
                with zipfile.ZipFile(io.BytesIO(blob)) as zf:
                    for name in zf.namelist():
                        sha, nbytes = digest(zf.read(name), entry["sha256"])
                        if sha == entry["sha256"]:
                            found = True
                            exp = entry.get("bytes", -1)
                            if exp != -1 and exp != nbytes:
                                problems.append(f"{entry['id']}: nested bytes "
                                                f"{nbytes} != builds.toml {exp}")
                            break
            except zipfile.BadZipFile:
                problems.append(f"{zip_name}: not a valid zip")
            if found:
                break
        if not found:
            problems.append(f"nested payload {entry['id']} not found in its "
                            f"container(s) {nested} (or hash mismatch)")

    return problems


def _pick_selftest_target(manifest: dict) -> str:
    """Prefer a nested-only payload (exercises the extract path); else baseline;
    else the first payload/container — so --self-test works for any corpus shape."""
    payloads = manifest.get("payload", [])
    for e in payloads:
        conts = e.get("containers", [])
        if conts and all(c.strip().startswith("inside ") for c in conts):
            return e["sha256"]
    baseline = manifest.get("baseline")
    if baseline:
        for e in payloads:
            if e["id"] == baseline:
                return e["sha256"]
    pool = payloads + manifest.get("archived", []) + manifest.get("container", [])
    if not pool:
        raise SystemExit("self-test: builds.toml has no payloads to corrupt.")
    return pool[0]["sha256"]


def main(argv: list[str]) -> int:
    manifest = load_builds(ROOT)

    if "--self-test" in argv:
        target = _pick_selftest_target(manifest)
        clean = verify(manifest)
        dirty = verify(manifest, corrupt=target)
        if clean:
            print("SELF-TEST FAILED: clean corpus already reports problems:")
            for pr in clean:
                print("  -", pr)
            return 3
        if not dirty:
            print("SELF-TEST FAILED: corrupting a payload did NOT trip the gate "
                  "(vacuous gate).")
            return 3
        print(f"self-test OK: clean corpus verifies; a one-byte corruption of "
              f"{target[:12]} is detected ({len(dirty)} problem(s)).")
        return 0

    problems = verify(manifest)
    if problems:
        print(f"originals-verify: {len(problems)} problem(s):", file=sys.stderr)
        for pr in problems:
            print("  -", pr, file=sys.stderr)
        return 1
    n_p = len(manifest.get("payload", []))
    n_a = len(manifest.get("archived", []))
    n_c = len(manifest.get("container", []))
    print(f"originals-verify OK: {n_p} payloads + {n_a} archived + {n_c} "
          f"containers reconciled against builds.toml.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
