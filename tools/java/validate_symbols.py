#!/usr/bin/env python3
"""Integrity gate for the reviewed naming ledger `java/reconstruction/symbols.toml`.

R10 forbids inventing identity from an obfuscated letter, and R14 forbids stale
docs / dangling references. This checks that every named member in the ledger is
a REAL `(obf, descriptor)` pair in the baseline bytecode (so a typo'd descriptor
or a member that drifted between builds cannot masquerade as reviewed naming),
that every named class exists, that semantic names are non-empty and unique
within a class+kind, that each entry carries evidence, and that confidences are
well-formed.

The ledger does NOT name all 37 baseline classes — it seeds the established
roles — so this gate does not demand a name for every class; it demands that
every name present RESOLVES and that the ledger is not vacuously small.

The bytecode is the authority (rulebook R2): membership is checked against the
classes parsed by `tools/java/classfile.py`, straight from the content-hash-
verified baseline jar, never a decompiler.

Modes:
  * (default / ``--check``) validate; non-zero exit on any problem.
  * ``--self-test``          inject a bogus entry and confirm it is caught (R3).
"""

from __future__ import annotations

import sys
import tomllib
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

import baseline  # noqa: E402

REPO = Path(__file__).resolve().parents[2]
SYMBOLS = REPO / "java" / "reconstruction" / "symbols.toml"
CONFIDENCES = {"high", "medium", "low"}

# Non-vacuity floors (GATES.md rule 2 / rule 3): an empty or stripped-down ledger
# must FAIL, not pass. Well below the seeded ~10 classes / ~78 members so the
# floor never fights a routine edit but a gutted ledger trips it.
MIN_CLASSES = 8
MIN_MEMBERS = 40


def baseline_members() -> dict[str, dict[str, set[tuple[str, str]]]]:
    """{internal_name: {"field": {(name,desc)...}, "method": {(name,desc)...}}}."""
    classes = baseline.baseline_classes()
    out: dict[str, dict[str, set[tuple[str, str]]]] = {}
    for name, info in classes.items():
        out[name] = {
            "field": {(f.name, f.descriptor) for f in info.fields},
            "method": {(m.name, m.descriptor) for m in info.methods},
        }
    return out


def load_ledger(text: str | None = None) -> dict:
    if text is None:
        return tomllib.loads(SYMBOLS.read_text(encoding="utf-8"))
    return tomllib.loads(text)


def validate(ledger: dict, real: dict) -> list[str]:
    problems: list[str] = []
    classes = ledger.get("classes", {})
    if not classes:
        problems.append("ledger names no classes (empty symbols.toml)")
    for cls, cinfo in classes.items():
        obf = cinfo.get("obf")
        if obf != cls:
            problems.append(f"class table {cls!r} has obf={obf!r} (must match the key)")
        if obf not in real:
            problems.append(f"ledger names unknown class {obf!r} (not in baseline)")
            continue
        if not cinfo.get("semantic"):
            problems.append(f"class {cls} has no semantic name")
        if not cinfo.get("role"):
            problems.append(f"class {cls} has no role")
        if not cinfo.get("evidence"):
            problems.append(f"class {cls} has no evidence")
        if cinfo.get("confidence") not in CONFIDENCES:
            problems.append(f"class {cls} confidence not in {CONFIDENCES}")
        for kind in ("field", "method"):
            seen: dict[str, int] = {}
            for entry in cinfo.get(kind, []):
                mobf = entry.get("obf")
                desc = entry.get("descriptor")
                sem = entry.get("semantic")
                conf = entry.get("confidence")
                where = f"{cls}.{kind} {mobf}:{desc} ({sem})"
                if not sem:
                    problems.append(f"{where}: empty semantic name")
                if not desc:
                    problems.append(f"{where}: missing descriptor")
                if conf not in CONFIDENCES:
                    problems.append(
                        f"{where}: confidence {conf!r} not in {CONFIDENCES}"
                    )
                if not entry.get("evidence"):
                    problems.append(f"{where}: missing evidence")
                if (mobf, desc) not in real[obf][kind]:
                    problems.append(
                        f"{where}: no such {kind} in baseline {obf} bytecode"
                    )
                if sem:
                    seen[sem] = seen.get(sem, 0) + 1
            dups = sorted(n for n, c in seen.items() if c > 1)
            if dups:
                problems.append(
                    f"class {cls}: duplicate {kind} semantic names {dups}"
                )
    return problems


def _counts(ledger: dict) -> tuple[int, int]:
    classes = ledger.get("classes", {})
    members = sum(
        len(c.get("field", [])) + len(c.get("method", []))
        for c in classes.values()
    )
    return len(classes), members


def check() -> int:
    real = baseline_members()
    ledger = load_ledger()
    problems = validate(ledger, real)
    n_classes, n_members = _counts(ledger)
    # Non-vacuity: a gutted ledger must fail even if every remaining name resolves.
    if n_classes < MIN_CLASSES:
        problems.append(
            f"ledger names only {n_classes} classes (< {MIN_CLASSES}); vacuous"
        )
    if n_members < MIN_MEMBERS:
        problems.append(
            f"ledger names only {n_members} members (< {MIN_MEMBERS}); vacuous"
        )
    if problems:
        print("symbols-check FAIL:", file=sys.stderr)
        for p in problems:
            print(f"  - {p}", file=sys.stderr)
        return 1
    print(
        f"symbols-check OK: all {n_members} named members across {n_classes} "
        f"classes resolve to real (obf, descriptor) pairs in the baseline bytecode."
    )
    return 0


def self_test() -> int:
    """R3: a ledger with one member whose descriptor does not exist must fail."""
    real = baseline_members()
    good = load_ledger()
    if validate(good, real):
        print(
            "self-test FAIL: the committed ledger already has problems",
            file=sys.stderr,
        )
        return 1
    # Inject a member that cannot exist (bogus descriptor) into a real class and
    # confirm it is caught.
    bogus = load_ledger()
    bogus["classes"]["m"].setdefault("method", []).append(
        {
            "obf": "zzz",
            "descriptor": "(Lnope;)V",
            "semantic": "does_not_exist",
            "evidence": "injected",
            "confidence": "low",
        }
    )
    problems = validate(bogus, real)
    if not any("no such method" in p for p in problems):
        print("self-test FAIL: a bogus member was not flagged", file=sys.stderr)
        return 1
    print(
        "symbols --self-test OK: an injected non-existent member is caught "
        "(gate can go red)."
    )
    return 0


def main(argv: list[str]) -> int:
    if not argv or argv[0] == "--check":
        return check()
    if argv[0] == "--self-test":
        return self_test()
    print(__doc__)
    return 2


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
