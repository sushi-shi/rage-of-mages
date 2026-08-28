#!/usr/bin/env python3
"""Materialize `_originals/` from an external private resources checkout.

The resource location is NEVER baked into this repository (playbook R1): it is
passed as an explicit argument, with the env var named in `game.toml`
(`resources_env`) as a fallback. This copies the surviving distribution files
into git-ignored `_originals/` and then runs `verify.py`.

Usage:
    fetch.py <path-to-resources-checkout-or-its-originals-dir>
    <RESOURCES_ENV>=<path> fetch.py
"""
from __future__ import annotations

import os
import shutil
import sys
from pathlib import Path

from corpus_common import load_game, repo_root

ROOT = repo_root(Path(__file__).parent)
ORIGINALS = ROOT / "_originals"


def resolve_source(argv: list[str], env_name: str) -> Path:
    raw = argv[0] if argv else os.environ.get(env_name, "")
    if not raw:
        sys.exit(f"fetch: no source given. Pass a resources checkout path as an "
                 f"argument or set ${env_name}.")
    src = Path(raw).expanduser().resolve()
    if (src / "originals").is_dir():
        src = src / "originals"
    if not src.is_dir():
        sys.exit(f"fetch: source is not a directory: {src}")
    return src


def main(argv: list[str]) -> int:
    game = load_game(ROOT)
    env_name = game.get("resources_env", "J2ME_RESOURCES")
    src = resolve_source(argv, env_name)
    if ORIGINALS.is_symlink():
        sys.exit("fetch: _originals is a symlink; refusing (playbook R2). "
                 "Remove it and let fetch create a real directory.")
    ORIGINALS.mkdir(exist_ok=True)
    copied = 0
    for item in sorted(src.iterdir()):
        if not item.is_file():
            continue
        dest = ORIGINALS / item.name
        if dest.exists() and dest.read_bytes() == item.read_bytes():
            continue
        shutil.copy2(item, dest)
        copied += 1
    print(f"fetch: materialized {ORIGINALS} from {src} ({copied} file(s) copied).")

    sys.path.insert(0, str(Path(__file__).parent))
    import verify  # noqa: E402
    return verify.main([])


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
