#!/usr/bin/env python3
"""Load the baseline `.class` payloads, keyed by the content hash builds.toml pins.

`java/reconstruction/builds.toml` is the tracked provenance authority (R2) and
git-ignored `_originals/` holds the immutable jar bytes. This joins the two: it
resolves the `baseline` build's jar, verifies the bytes hash to the recorded
sha256 (identity is the hash, never the filename — R2/R10), and hands back the
parsed classes.

Corpus-dependent, so it fails LOUD when `_originals/` is missing (GATES.md rule
3: a skip must never read as a pass) — never returns an empty corpus.
"""

from __future__ import annotations

import hashlib
import io
import sys
import zipfile
from pathlib import Path

try:
    import tomllib
except ModuleNotFoundError:  # pragma: no cover - Python < 3.11
    import tomli as tomllib  # type: ignore

sys.path.insert(0, str(Path(__file__).resolve().parent))
import classfile  # noqa: E402

REPO = Path(__file__).resolve().parents[2]
BUILDS_TOML = REPO / "java" / "reconstruction" / "builds.toml"
ORIGINALS = REPO / "_originals"


class CorpusError(RuntimeError):
    pass


def _sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def load_manifest() -> dict:
    with BUILDS_TOML.open("rb") as handle:
        return tomllib.load(handle)


def baseline_entry() -> dict:
    manifest = load_manifest()
    want = manifest.get("baseline")
    if not want:
        raise CorpusError("builds.toml has no `baseline` key")
    for entry in manifest.get("payload", []):
        if entry.get("id") == want:
            return entry
    raise CorpusError(f"baseline {want!r} not found among [[payload]] entries")


def baseline_payload() -> bytes:
    """The baseline jar bytes, resolved by content hash from a `containers` ref."""
    entry = baseline_entry()
    want = entry["sha256"]
    for ref in entry.get("containers", []):
        ref = ref.strip()
        prefix = "_originals/"
        if not ref.startswith(prefix):
            continue
        path = ORIGINALS / ref[len(prefix):]
        if path.is_file():
            data = path.read_bytes()
            if _sha256(data) == want:
                return data
    raise CorpusError(
        f"baseline payload {entry['id']} (sha {want[:12]}) not found in "
        f"{ORIGINALS}; materialize it with `just bootstrap <rage-of-mages-resources>`"
    )


def jar_class_bytes(payload: bytes) -> dict[str, bytes]:
    """(member_path -> bytes) for every `.class` member, sorted by member name."""
    with zipfile.ZipFile(io.BytesIO(payload)) as jar:
        names = sorted(n for n in jar.namelist() if n.endswith(".class"))
        return {name: jar.read(name) for name in names}


def baseline_classes(payload: bytes | None = None) -> dict[str, classfile.ClassInfo]:
    """Parse every baseline class, keyed by its internal name (e.g. `m`,
    `Container/GameMIDlet`). A pure function of the verified jar bytes."""
    if payload is None:
        payload = baseline_payload()
    classes: dict[str, classfile.ClassInfo] = {}
    for member_path, data in jar_class_bytes(payload).items():
        info = classfile.parse_class(member_path, data)
        classes[info.internal_name] = info
    if not classes:
        raise CorpusError("baseline jar carries no classes (corpus broken)")
    return classes
