#!/usr/bin/env python3
"""The R8 numeric-shape authority for the baseline's transliteration surface.

Java promotes `byte`/`short` to `int` before arithmetic and narrows the `int`
result on a cast back; a decompiler routinely hides that. A decompiled `a / b`
fed to a float can be *either* an integer `idiv` widened afterwards (`idiv` then
`i2f`) *or* a per-operand widened float divide (`i2f i2f fdiv`) — the Java text is
identical, the arithmetic is not. Rulebook R8 says the bytecode is the authority
and a decompiled numeric expression is never trusted.

This tool extracts, straight from the `.class` bytes (never a decompiler), the
ORDERED sequence of numeric opcodes each method executes — arithmetic, shifts,
bitwise ops, `iinc`, every `x2y` conversion, and the `lcmp`/`fcmp`/`dcmp`
comparisons (which carry the NaN-ordering a transliterator must reproduce). That
sequence is the arithmetic *shape* the transliterated method must reproduce
exactly. A transliterator consults it before porting a method: if the port's
widen/narrow/convert order does not match, the port is wrong even if it compiles
and "looks like" the decompiled Java.

The authority lands in git-ignored `_reference/numeric-shapes.json` (evidence,
regenerable). Modes:

  * (default)     regenerate the authority file.
  * ``--check``   regenerate and compare byte-for-byte against the file on disk;
                  exit non-zero on drift, a missing file, or a nondeterministic
                  render. This is the gate.
  * ``--self-test`` prove the check goes red on a perturbed shape (rulebook R3).
  * ``--show C.n:desc`` print one method's numeric shape (for a transliterator).

Reads the baseline jar directly via `tools/java/baseline.py` (content-hash
verified against builds.toml) and `tools/java/classfile.py` (the from-scratch
class parser). Covers EVERY class in the baseline jar — the whole obfuscated
program is the eventual transliteration surface.
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

import baseline  # noqa: E402  (tools/java/baseline.py)
import classfile  # noqa: E402  (tools/java/classfile.py)

REPO = Path(__file__).resolve().parents[2]
OUT = REPO / "_reference" / "numeric-shapes.json"

# The JVM's numeric-operation opcodes, in opcode order (0x60..0x98). Every one
# either does width-sensitive arithmetic, changes a value's width/type, or
# encodes NaN-ordering — exactly the decisions R8 forbids trusting a decompiler
# for. Anything outside this set (loads, stores, branches, invokes) is not part
# of the arithmetic shape and is deliberately excluded.
NUMERIC_OPCODES = {
    0x60: "iadd", 0x61: "ladd", 0x62: "fadd", 0x63: "dadd",
    0x64: "isub", 0x65: "lsub", 0x66: "fsub", 0x67: "dsub",
    0x68: "imul", 0x69: "lmul", 0x6A: "fmul", 0x6B: "dmul",
    0x6C: "idiv", 0x6D: "ldiv", 0x6E: "fdiv", 0x6F: "ddiv",
    0x70: "irem", 0x71: "lrem", 0x72: "frem", 0x73: "drem",
    0x74: "ineg", 0x75: "lneg", 0x76: "fneg", 0x77: "dneg",
    0x78: "ishl", 0x79: "lshl", 0x7A: "ishr", 0x7B: "lshr",
    0x7C: "iushr", 0x7D: "lushr",
    0x7E: "iand", 0x7F: "land", 0x80: "ior", 0x81: "lor",
    0x82: "ixor", 0x83: "lxor",
    0x84: "iinc",
    0x85: "i2l", 0x86: "i2f", 0x87: "i2d",
    0x88: "l2i", 0x89: "l2f", 0x8A: "l2d",
    0x8B: "f2i", 0x8C: "f2l", 0x8D: "f2d",
    0x8E: "d2i", 0x8F: "d2l", 0x90: "d2f",
    0x91: "i2b", 0x92: "i2c", 0x93: "i2s",
    0x94: "lcmp",
    0x95: "fcmpl", 0x96: "fcmpg", 0x97: "dcmpl", 0x98: "dcmpg",
}

# Documentation block emitted into the file so a reader (or a transliterator)
# knows exactly which opcodes define the shape.
TRACKED = [NUMERIC_OPCODES[k] for k in sorted(NUMERIC_OPCODES)]

# Non-vacuity floors (GATES.md rule 2). The baseline is ~37 classes; the extractor
# must find hundreds of methods and thousands of numeric opcodes across many of
# them. Anything below these means the extractor is broken (a corpus that
# produced zero shapes must FAIL, not pass) — deliberately well under the real
# counts so the floor never fights a routine change.
MIN_METHODS = 400
MIN_TOTAL_OPCODES = 2000
MIN_METHODS_WITH_OPS = 150


class ShapeError(RuntimeError):
    pass


def _numeric_shape(code: bytes) -> list[str]:
    """The ordered numeric-opcode mnemonics of one method's Code attribute."""
    shape: list[str] = []
    for _, opcode, _operand in classfile.instructions(code):
        mnemonic = NUMERIC_OPCODES.get(opcode)
        if mnemonic is not None:
            shape.append(mnemonic)
    return shape


def build_shapes(classes: dict | None = None) -> dict:
    """The authoritative numeric-shape table for every baseline class.

    A pure function of the class bytes — deterministic, so the render is
    byte-identical across runs. Classes are emitted in sorted internal-name
    order and methods in their class-file (ordinal) order for a stable render.
    """
    if classes is None:
        classes = baseline.baseline_classes()

    methods: list[dict] = []
    for class_name in sorted(classes):
        info = classes[class_name]
        for method in info.methods:
            shape = _numeric_shape(method.code) if method.code is not None else []
            methods.append(
                {
                    "class": class_name,
                    "ordinal": method.ordinal,
                    "name": method.name,
                    "descriptor": method.descriptor,
                    "abstract": method.code is None,
                    "numeric_shape": shape,
                    "shape_sha256": classfile.sha256("\n".join(shape)),
                }
            )
    return {
        "build": baseline.baseline_entry()["id"],
        "note": (
            "R8 numeric-shape authority: the ordered JVM numeric opcodes each "
            "transliterated method must reproduce. Generated from class bytes; "
            "never hand-edit (regenerate with "
            "tools/java/validate_numeric_shape.py)."
        ),
        "opcodes_tracked": TRACKED,
        "class_count": len(classes),
        "classes": sorted(classes),
        "method_count": len(methods),
        "methods": methods,
    }


def render(data: dict) -> bytes:
    """Deterministic JSON bytes (stable key order, trailing newline)."""
    text = json.dumps(data, indent=1, ensure_ascii=True)
    return (text + "\n").encode("utf-8")


def _shape_index(data: dict) -> dict[str, list[str]]:
    return {
        f"{m['class']}.{m['name']}:{m['descriptor']}#{m['ordinal']}": m["numeric_shape"]
        for m in data["methods"]
    }


def _assert_non_vacuous(data: dict) -> None:
    """Guard against a vacuous authority (GATES.md rule 2): the extractor must
    actually have found numeric opcodes across many methods, not compare empty
    lists against empty lists. Builds the asserted quantities in-process — never
    parses a tool's stdout (GATES.md rule 3)."""
    total_ops = sum(len(m["numeric_shape"]) for m in data["methods"])
    with_ops = sum(1 for m in data["methods"] if m["numeric_shape"])
    if data["method_count"] < MIN_METHODS:
        raise ShapeError(
            f"only {data['method_count']} methods; expected the baseline's many "
            f"hundreds (>= {MIN_METHODS})"
        )
    if total_ops < MIN_TOTAL_OPCODES or with_ops < MIN_METHODS_WITH_OPS:
        raise ShapeError(
            f"numeric shape looks vacuous: {total_ops} opcodes across "
            f"{with_ops} methods (extractor likely broken)"
        )


def generate() -> int:
    data = build_shapes()
    _assert_non_vacuous(data)
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_bytes(render(data))
    total = sum(len(m["numeric_shape"]) for m in data["methods"])
    with_ops = sum(1 for m in data["methods"] if m["numeric_shape"])
    print(
        f"numeric-shape: wrote {OUT.relative_to(REPO)} — "
        f"{data['class_count']} classes, {data['method_count']} methods, "
        f"{total} numeric opcodes across {with_ops} methods."
    )
    return 0


def check() -> int:
    """Regenerate and compare byte-for-byte to the on-disk authority."""
    first = render(build_shapes())
    second = render(build_shapes())
    if first != second:
        print(
            "numeric-shape --check FAIL: render is nondeterministic",
            file=sys.stderr,
        )
        return 1
    _assert_non_vacuous(json.loads(first))
    if not OUT.is_file():
        print(
            f"numeric-shape --check FAIL: {OUT.relative_to(REPO)} is missing; "
            f"run `just numeric-shape` first",
            file=sys.stderr,
        )
        return 1
    on_disk = OUT.read_bytes()
    if on_disk != first:
        want = _shape_index(json.loads(first))
        have = _shape_index(json.loads(on_disk))
        drifted = [k for k in want if want.get(k) != have.get(k)]
        print(
            "numeric-shape --check FAIL: on-disk authority differs from a fresh "
            f"regeneration ({len(drifted)} method(s) drifted). First few: "
            f"{drifted[:5]}",
            file=sys.stderr,
        )
        return 1
    print(
        f"numeric-shape --check OK: {OUT.relative_to(REPO)} is byte-identical "
        f"to a regen."
    )
    return 0


def self_test() -> int:
    """R3 can-fail proof: perturb one method's numeric shape by exactly one
    opcode and confirm the byte-identical comparison the gate relies on goes red
    and names the perturbed method."""
    data = build_shapes()
    _assert_non_vacuous(data)
    clean = render(data)

    # Find a real method that actually has a numeric shape to perturb.
    victim = next((m for m in data["methods"] if m["numeric_shape"]), None)
    if victim is None:
        print("self-test FAIL: no method carries a numeric shape", file=sys.stderr)
        return 1

    perturbed = json.loads(clean.decode("utf-8"))
    for m in perturbed["methods"]:
        if (m["class"], m["name"], m["descriptor"], m["ordinal"]) == (
            victim["class"],
            victim["name"],
            victim["descriptor"],
            victim["ordinal"],
        ):
            # Swap one integer op for its float sibling: the exact R8 confusion
            # (idiv vs fdiv) a decompiler hides. Fall back to flipping the first
            # opcode to a conversion if the shape has no idiv.
            shape = list(m["numeric_shape"])
            if "idiv" in shape:
                shape[shape.index("idiv")] = "fdiv"
            else:
                shape[0] = "i2f" if shape[0] != "i2f" else "i2b"
            m["numeric_shape"] = shape
            m["shape_sha256"] = classfile.sha256("\n".join(shape))
            break

    dirty = render(perturbed)
    if dirty == clean:
        print(
            "self-test FAIL: perturbing a shape did not change the render",
            file=sys.stderr,
        )
        return 1

    want = _shape_index(json.loads(clean))
    have = _shape_index(json.loads(dirty))
    drifted = [k for k in want if want.get(k) != have.get(k)]
    expected = (
        f"{victim['class']}.{victim['name']}:{victim['descriptor']}"
        f"#{victim['ordinal']}"
    )
    if drifted != [expected]:
        print(
            f"self-test FAIL: expected exactly [{expected}] to drift, got {drifted}",
            file=sys.stderr,
        )
        return 1
    print(
        "numeric-shape --self-test OK: a one-opcode perturbation of "
        f"{expected} is caught by the byte-identical check (gate can go red)."
    )
    return 0


def show(query: str) -> int:
    data = build_shapes()
    index = _shape_index(data)
    if query in index:
        print(f"{query}\n    {index[query]}")
        return 0
    # Be forgiving: allow "C.name" / "C.name:desc" (no ordinal) to list matches.
    matches = [k for k in index if k.startswith(query)]
    if not matches:
        print(f"no method matches {query!r}", file=sys.stderr)
        return 1
    for k in matches:
        print(f"{k}\n    {index[k]}")
    return 0


def main(argv: list[str]) -> int:
    if not argv:
        return generate()
    if argv[0] == "--check":
        return check()
    if argv[0] == "--self-test":
        return self_test()
    if argv[0] == "--show" and len(argv) >= 2:
        return show(argv[1])
    print(__doc__)
    return 2


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
