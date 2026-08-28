#!/usr/bin/env python3
"""Shared helpers for the resource-provenance tools (verify / fetch / generate).

Game-neutral: reads the per-game config from `game.toml` at the repository root
so the same tooling serves every scaffolded J2ME port. No resource location is
ever baked into the repo (playbook R1) — `fetch` takes it as an explicit
argument, with the env var named in `game.toml` as a fallback.
"""
from __future__ import annotations

import hashlib
from pathlib import Path

try:
    import tomllib
except ModuleNotFoundError:  # pragma: no cover - Python < 3.11
    import tomli as tomllib  # type: ignore


def repo_root(start: Path) -> Path:
    """Walk up from `start` until a directory containing game.toml is found."""
    p = start.resolve()
    for cand in [p, *p.parents]:
        if (cand / "game.toml").is_file():
            return cand
    # Fallback: the tool lives at tools/originals/<file>, so root is parents[2].
    return start.resolve().parents[2]


def load_game(root: Path) -> dict:
    with (root / "game.toml").open("rb") as fh:
        return tomllib.load(fh)


def load_builds(root: Path) -> dict:
    with (root / "java" / "reconstruction" / "builds.toml").open("rb") as fh:
        return tomllib.load(fh)


def sha256_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()
