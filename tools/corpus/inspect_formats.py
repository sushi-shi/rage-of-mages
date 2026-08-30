#!/usr/bin/env python3
"""Evidence generator for the Rage of Mages custom resource formats.

Reconstructs, straight from the surviving jar bytes, the framing facts that
`docs/FORMATS.md` states — so the spec is reproducible and any drift is caught:

  * .map    header (u16 LE width,height) + width*height tile cells, EOF-exact,
            cell bytes in the closed range 128..173 (the f251o remap domain).
  * .utf    a flat stream of Java readUTF records (u16 BE len + modified-UTF-8);
            record COUNT is a cross-language invariant (RU baseline vs EN).
  * .pak    [i32 BE count][count*i32 BE lengths] dir + XOR-0x53 entries -> PNG,
            EOF-exact (res0..4). scenes.pak is a DIFFERENT big-endian nested-int
            structure (not XOR-packed) despite the extension.
  * .int    90 * i16 BE, value = round(10000 * sin/cos(deg)), deg 0..89.

Reads the jars directly (immutable `_originals/`); writes nothing. Run inside the
nix dev shell:  python3 tools/corpus/inspect_formats.py
"""
from __future__ import annotations

import struct
import sys
import zipfile
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
ORIG = REPO / "_originals"
BASE = ORIG / "allods_176x220.jar"          # behavior authority (RU)
EN = ORIG / "Rage-of-Mages_J2ME_EN_v11.jar"  # English text source

UTF_FILES = ["1", "2", "common", "dialogs0", "dialogs1",
             "messages", "quests", "rumours", "skills"]
PAKS = [f"res{i}" for i in range(5)]
XOR_KEY = 0x53  # 'S' — the res0..4.pak entry cipher (class j)


def entry(jar: Path, name: str) -> bytes:
    with zipfile.ZipFile(jar) as z:
        return z.read(name)


def check_maps(jar: Path) -> bool:
    ok = True
    with zipfile.ZipFile(jar) as z:
        maps = sorted(n for n in z.namelist() if n.endswith(".map"))
    print("== .map (u16 LE w,h + w*h cells; cells in 128..173) ==")
    for n in maps:
        d = entry(jar, n)
        w = d[0] | (d[1] << 8)
        h = d[2] | (d[3] << 8)
        body = d[4:]
        eof = len(d) == 4 + w * h
        rng = (min(body), max(body)) if body else (0, 0)
        good = eof and 128 <= rng[0] and rng[1] <= 173
        ok &= good
        print(f"  {n:14s} {w:3d}x{h:<3d} body={len(body):5d} "
              f"eof={'OK' if eof else 'BAD'} cells={rng[0]}..{rng[1]} "
              f"{'' if good else '  <-- UNEXPECTED'}")
    return ok


def utf_records(blob: bytes):
    off, recs = 0, []
    while off + 2 <= len(blob):
        ln = struct.unpack_from(">H", blob, off)[0]
        recs.append(blob[off + 2:off + 2 + ln])
        off += 2 + ln
    return recs, off


def check_utf() -> bool:
    ok = True
    print("== .utf (readUTF stream; RU/EN record count invariant) ==")
    for u in UTF_FILES:
        rb = entry(BASE, f"res/{u}.utf")
        eb = entry(EN, f"res/{u}.utf")
        rr, ro = utf_records(rb)
        er, eo = utf_records(eb)
        same = len(rr) == len(er)
        clean = ro == len(rb) and eo == len(eb)
        ok &= same and clean
        print(f"  {u:10s} RU={len(rb):5d}B/{len(rr):3d}rec  EN={len(eb):5d}B/{len(er):3d}rec  "
              f"count={'MATCH' if same else 'DIFFER'} eof={'exact' if clean else 'RAGGED'}")
    return ok


def check_paks(jar: Path) -> bool:
    ok = True
    print(f"== .pak (i32 BE dir + XOR 0x53 -> PNG) [{jar.name}] ==")
    for p in PAKS:
        d = entry(jar, f"res/{p}.pak")
        n = struct.unpack_from(">i", d, 0)[0]
        lens = [struct.unpack_from(">i", d, 4 + 4 * i)[0] for i in range(n)]
        hdr = 4 + 4 * n
        eof = hdr + sum(lens) == len(d)
        e0 = bytes(b ^ XOR_KEY for b in d[hdr:hdr + lens[0]])
        png = e0[1:4] == b"PNG"
        ok &= eof and png
        print(f"  {p:6s} entries={n:3d} eof={'OK' if eof else 'BAD'} "
              f"entry0^0x53={'PNG' if png else e0[:4].hex()}")
    # scenes.pak is a different structure (big-endian nested ints, not XOR)
    s = entry(jar, "res/scenes.pak")
    head = struct.unpack_from(">6i", s, 0)
    print(f"  scenes {len(s)}B  first 6 i32 BE={head}  (nested-int, NOT XOR pak)")
    return ok


def check_sincos() -> bool:
    print("== sincos (.int: 90 * i16 BE, scale 10000, deg 0..89) ==")
    ok = True
    for name, fn in (("sin", math_sin), ("cos", math_cos)):
        d = entry(BASE, f"sincos/{name}.int")
        vals = list(struct.unpack(">90h", d))
        worst = max(abs(vals[i] - round(10000 * fn(i))) for i in range(90))
        ok &= len(d) == 180 and worst <= 1
        print(f"  {name}.int {len(d)}B n={len(vals)} v[0]={vals[0]} v[45]={vals[45]} "
              f"v[89]={vals[89]} max|err|vs round(1e4*{name}(deg))={worst}")
    return ok


def math_sin(deg):
    import math
    return math.sin(math.radians(deg))


def math_cos(deg):
    import math
    return math.cos(math.radians(deg))


def main() -> int:
    ok = True
    ok &= check_maps(BASE)
    print()
    ok &= check_utf()
    print()
    ok &= check_paks(BASE)
    print()
    ok &= check_sincos()
    print()
    print("ALL FRAMING CHECKS PASS" if ok else "SOME CHECKS FAILED")
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
