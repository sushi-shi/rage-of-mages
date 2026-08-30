#!/usr/bin/env python3
"""Evidence generator for docs/SEMANTICS.md (game-logic semantics).

Re-derives, straight from the baseline jar (`_originals/allods_176x220.jar`),
the corpus- and bytecode-level facts that `docs/SEMANTICS.md` states — so the
spec is reproducible and any drift is caught:

  * sprite slots (§1.1): res0..4.pak entry counts are 54/41/53/57/75 — i.e. the
    280-slot global table with hardcoded boundaries g.f114w = {53,94,147,204,279};
    the campaign tile sprites (slots 26..63) decrypt to PNGs of the exact
    dimensions the tile table lists.
  * the f251o tile remap (§1.2): parsed out of m.<init>'s bytecode (javap) and
    compared against the documented 46-entry table.
  * save-element sizes (§4): ab.c() == 47, q.a() == 228, n.m45a() base == 46 —
    read from the bytecode constants, and the ab/n fixed layouts re-summed from
    the documented field widths.
  * .map coverage (§1.5): every cell's raw code is 0..45, and the remapped tile
    ids used by the shipped maps are exactly {26..36, 44..61}.
  * scenes.pak (§3): parses EOF-exact as 12 scenes x 3 groups; every group-2
    trigger row is 5 ints; every group-3 spawn row is 9 or 10 ints with
    record id in 0..49, creature type in 0..17, hostile flag in {0,1}; prints
    the per-level summary table of §3.4.

Reads the jar directly (immutable `_originals/`); writes only a temp dir it
cleans up. Requires `javap` on PATH — run inside the nix dev shell:

    python3 tools/corpus/semantics_evidence.py
    python3 tools/corpus/semantics_evidence.py --self-test   # prove it can fail
"""
from __future__ import annotations

import re
import shutil
import struct
import subprocess
import sys
import tempfile
import zipfile
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
BASE = REPO / "_originals" / "allods_176x220.jar"
XOR_KEY = 0x53

# ---------------------------------------------------------------- documented facts
PAK_ENTRY_COUNTS = [54, 41, 53, 57, 75]          # -> f114w {53,94,147,204,279}
F251O = [26, 44, 46, 33, 47, 48, 27, 28, 29, 30, 31, 36, 37, 38, 39, 40, 41,
         42, 43, 36, 37, 38, 39, 40, 41, 42, 43, 49, 50, 51, 60, 61, 52, 53,
         55, 32, 56, 45, 36, 58, 54, 59, 26, 57, 35, 34]
TILE_DIMS = {  # slot -> (w, h) of the decrypted PNG (SEMANTICS.md §1.5)
    26: (100, 116), 27: (48, 48), 28: (16, 48), 29: (16, 48), 30: (48, 16),
    31: (48, 16), 32: (64, 16), 33: (32, 16), 34: (32, 16), 35: (90, 68),
    36: (32, 32), 37: (32, 32), 38: (32, 32), 39: (32, 32), 40: (32, 32),
    41: (32, 32), 42: (32, 32), 43: (32, 32), 44: (47, 32), 45: (59, 60),
    46: (42, 35), 47: (48, 80), 48: (48, 80), 49: (37, 41), 50: (43, 47),
    51: (12, 25), 52: (16, 16), 53: (16, 16), 54: (73, 83), 55: (90, 82),
    56: (57, 69), 57: (76, 94), 58: (74, 130), 59: (124, 109), 60: (92, 64),
    61: (92, 74), 62: (61, 65), 63: (6, 5),
}
MAP_TILE_IDS = set(range(26, 37)) | set(range(44, 62))   # §1.5 corpus note
# ab wire (§4.1): kind 1 + id 4 + sockets 1 + attached 1 + 10*4 gem refs = 47
AB_FIELDS = [1, 4, 1, 1, 40]
# n fixed wire (§4.3): the 19 fields before the name blob = 46
N_FIELDS = [1, 1, 4, 1, 4, 4, 1, 1, 4, 4, 1, 1, 4, 4, 4, 4, 1, 1, 1]
Q_BYTES = 57 * 4                                          # §4.2

CREATURES = ["swordsman", "archer lady", "healer", "wizard", "troll", "hermit",
             "orc", "dragon", "type-8", "Goblin", "mage", "skeleton", "bat",
             "skeleton(13)", "evil squirrel", "turtle", "guard",
             "poisoned troll"]


def fail(msg: str) -> None:
    print(f"  FAIL: {msg}")
    fail.count += 1


fail.count = 0  # type: ignore[attr-defined]


def read_entry(name: str) -> bytes:
    with zipfile.ZipFile(BASE) as z:
        return z.read(name)


# ---------------------------------------------------------------- §1.1 sprite slots
def load_pak_entries() -> list[bytes]:
    entries: list[bytes] = []
    print("== sprite slots: res0..4.pak entry counts -> 280-slot table ==")
    for i, expect in enumerate(PAK_ENTRY_COUNTS):
        d = read_entry(f"res/res{i}.pak")
        (n,) = struct.unpack(">i", d[:4])
        lens = struct.unpack(f">{n}i", d[4 : 4 + 4 * n])
        off = 4 + 4 * n
        first = len(entries)
        for ln in lens:
            entries.append(bytes(b ^ XOR_KEY for b in d[off : off + ln]))
            off += ln
        ok = n == expect and off == len(d)
        print(f"  res{i}.pak entries={n} slots {first}..{first+n-1} "
              f"eof={'OK' if off == len(d) else 'BAD'}")
        if not ok:
            fail(f"res{i}.pak: expected {expect} entries, got {n}")
    if len(entries) != 280:
        fail(f"total slots {len(entries)} != 280")
    return entries


def check_tile_dims(entries: list[bytes]) -> None:
    print("== tile sprites 26..63: decrypted PNG dimensions ==")
    for slot, (w, h) in sorted(TILE_DIMS.items()):
        p = entries[slot]
        if p[:8] != b"\x89PNG\r\n\x1a\n":
            fail(f"slot {slot}: not a PNG after XOR")
            continue
        gw, gh = struct.unpack(">II", p[16:24])
        if (gw, gh) != (w, h):
            fail(f"slot {slot}: {gw}x{gh}, documented {w}x{h}")
    print(f"  {len(TILE_DIMS)} slots checked")


# ---------------------------------------------------------------- §1.2 f251o (javap)
def javap(tmp: Path, cls: str) -> str:
    with zipfile.ZipFile(BASE) as z:
        z.extract(f"{cls}.class", tmp)
    return subprocess.run(
        ["javap", "-c", "-p", f"{cls}.class"], cwd=tmp,
        capture_output=True, text=True, check=True).stdout


def parse_byte_array_literals(disasm: str, length: int) -> list[list[int]]:
    """Every `<length> newarray byte` literal filled by dup/index/value/bastore."""
    # Walk the instruction stream textually: find "newarray byte" preceded by the
    # length push, then consume (index, value) pairs before each bastore.
    out: list[list[int]] = []
    lines = disasm.splitlines()
    i = 0
    while i < len(lines):
        if "newarray" in lines[i] and "byte" in lines[i]:
            prev = "\n".join(lines[max(0, i - 2): i])
            m = re.search(r"(?:bipush\s+(\d+)|iconst_(\d))\s*$", prev.strip())
            if m and int(m.group(1) or m.group(2)) == length:
                arr = [0] * length
                j = i + 1
                pending: list[int] = []
                while j < len(lines):
                    ln = lines[j]
                    pm = re.search(r"(?:bipush\s+(-?\d+)|sipush\s+(-?\d+)|iconst_(m1|[0-5]))\s*(?://.*)?$", ln)
                    if pm:
                        pending.append(
                            int(pm.group(1) or pm.group(2)) if (pm.group(1) or pm.group(2))
                            else (-1 if pm.group(3) == "m1" else int(pm.group(3))))
                    elif "bastore" in ln:
                        if len(pending) >= 2:
                            idx, v = pending[-2], pending[-1]
                            if 0 <= idx < length:
                                arr[idx] = v
                        pending = []
                    elif "dup" in ln:
                        pass
                    else:
                        break  # literal ended
                    j += 1
                out.append(arr)
                i = j
                continue
        i += 1
    return out


def check_bytecode(tmp: Path, f251o_expect: list[int]) -> None:
    print("== bytecode constants (javap; tie-breaking authority) ==")
    m_dis = javap(tmp, "m")
    cands = parse_byte_array_literals(m_dis, 46)
    if not any(c == f251o_expect for c in cands):
        fail(f"f251o: no 46-entry byte-array literal in m.class matches the "
             f"documented table (found {len(cands)} candidates)")
    else:
        print("  m.f251o 46-entry remap literal matches")

    ab_dis = javap(tmp, "ab")
    m = re.search(r"public static final int c\(\);\s*\n\s*Code:\s*\n\s*0: bipush\s+(\d+)", ab_dis)
    if not m or int(m.group(1)) != sum(AB_FIELDS):
        fail(f"ab.c(): bytecode constant != documented {sum(AB_FIELDS)}")
    else:
        print(f"  ab.c() == {sum(AB_FIELDS)} (kind1+id4+sockets1+attached1+10x4)")

    q_dis = javap(tmp, "q")
    m = re.search(r"public static final int a\(\);\s*\n\s*Code:\s*\n\s*0: sipush\s+(\d+)", q_dis)
    if not m or int(m.group(1)) != Q_BYTES:
        fail(f"q.a(): bytecode constant != documented {Q_BYTES}")
    else:
        print(f"  q.a() == {Q_BYTES} (57 x 4-byte offset-LE ints)")

    n_dis = javap(tmp, "n")
    m = re.search(r"public final int a\(\);\s*\n\s*Code:\s*\n\s*0: bipush\s+(\d+)", n_dis)
    if not m or int(m.group(1)) != sum(N_FIELDS):
        fail(f"n.m45a(): fixed-part constant != documented {sum(N_FIELDS)}")
    else:
        print(f"  n record fixed part == {sum(N_FIELDS)} (sum of the 19 field widths)")


# ---------------------------------------------------------------- §1.5 map coverage
def check_maps() -> None:
    print("== .map coverage: raw codes 0..45; remapped ids in the tile table ==")
    with zipfile.ZipFile(BASE) as z:
        maps = sorted(n for n in z.namelist() if n.endswith(".map"))
    used: set[int] = set()
    for name in maps:
        d = read_entry(name)
        for b in d[4:]:
            raw = b - 128
            if not 0 <= raw <= 45:
                fail(f"{name}: raw code {raw} outside 0..45")
                break
            used.add(F251O[raw])
    extra = used - MAP_TILE_IDS
    if extra:
        fail(f"maps use undocumented tile ids {sorted(extra)}")
    print(f"  {len(maps)} maps; tile ids used: {sorted(used)}")


# ---------------------------------------------------------------- §3 scenes.pak
def check_scenes() -> None:
    print("== scenes.pak: 12 scenes x 3 groups; row-shape + per-level summary ==")
    d = read_entry("res/scenes.pak")
    off = 0

    def ri() -> int:
        nonlocal off
        v = struct.unpack_from(">i", d, off)[0]
        off += 4
        return v

    for lvl in range(12):
        groups: list[list[list[int]]] = []
        for _ in range(3):
            rows = ri()
            g = []
            for _ in range(rows):
                ln = ri()
                g.append([ri() for _ in range(ln)])
            groups.append(g)
        scripts, trigs, spawns = groups
        for r in trigs:
            if len(r) != 5:
                fail(f"lvl {lvl}: trigger row of length {len(r)} (expected 5)")
        types: dict[str, int] = {}
        for r in spawns:
            if len(r) not in (9, 10):
                fail(f"lvl {lvl}: spawn row of length {len(r)} (expected 9|10)")
                continue
            if not 0 <= r[0] <= 49:
                fail(f"lvl {lvl}: spawn record id {r[0]} outside f180d[50]")
            if not 0 <= r[3] <= 17:
                fail(f"lvl {lvl}: creature type {r[3]} outside 0..17")
            if r[5] not in (0, 1):
                fail(f"lvl {lvl}: hostile flag {r[5]} not boolean")
            name = CREATURES[r[3]] if 0 <= r[3] <= 17 else str(r[3])
            types[name] = types.get(name, 0) + 1
        summary = ", ".join(f"{v}x{k}" if v > 1 else k
                            for k, v in sorted(types.items(), key=lambda kv: -kv[1]))
        print(f"  lvl {lvl:2d}: scripts={len(scripts):2d} triggers={len(trigs):2d} "
              f"spawns={len(spawns):2d}  [{summary}]")
    if off != len(d):
        fail(f"scenes.pak: {len(d) - off} bytes left after 12 scenes (not EOF-exact)")


# ---------------------------------------------------------------- driver
def run() -> int:
    if not BASE.is_file():
        print(f"FAIL: baseline jar missing: {BASE}\n"
              "      (originals not fetched — a skip must never read as a pass)")
        return 1
    if shutil.which("javap") is None:
        print("FAIL: javap not on PATH — run inside the nix dev shell")
        return 1
    entries = load_pak_entries()
    check_tile_dims(entries)
    tmp = Path(tempfile.mkdtemp(prefix="semantics_evidence_"))
    try:
        check_bytecode(tmp, F251O)
    finally:
        shutil.rmtree(tmp, ignore_errors=True)
    check_maps()
    check_scenes()
    if fail.count:  # type: ignore[attr-defined]
        print(f"\n{fail.count} FAILURE(S) — docs/SEMANTICS.md and the jar disagree")
        return 1
    print("\nall checks green — docs/SEMANTICS.md matches the jar")
    return 0


def self_test() -> int:
    """Prove the gate bites: a corrupted expectation must turn the run red."""
    global F251O
    good = run()
    if good != 0:
        print("self-test: baseline run is already red — fix that first")
        return 1
    print("\n-- self-test: corrupting one f251o entry; expecting failure --")
    fail.count = 0  # type: ignore[attr-defined]
    F251O = F251O.copy()
    F251O[0] = 99
    tmp = Path(tempfile.mkdtemp(prefix="semantics_evidence_st_"))
    try:
        check_bytecode(tmp, F251O)
    finally:
        shutil.rmtree(tmp, ignore_errors=True)
    if fail.count == 0:  # type: ignore[attr-defined]
        print("self-test FAILED: corrupted table not detected")
        return 1
    print("self-test OK: corruption detected (gate can go red)")
    return 0


if __name__ == "__main__":
    sys.exit(self_test() if "--self-test" in sys.argv[1:] else run())
