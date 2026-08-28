# Rage of Mages — J2ME recovery & native Rust port

A game-preservation project: recover Rage of Mages from its surviving J2ME builds and
reimplement it as a maintainable native Rust game for Linux and the browser,
following the shared method in `../PLAYBOOK.md` (the j2me home).

This repository is **resource-free** and dedicated to the public domain (CC0): it
contains only recovered-by-hand reconstruction and our own code. The original
game binaries are never committed — they live in a private resources location and
are materialized locally by an explicit fetch.

## Getting started

```sh
# materialize + verify the corpus from the private resources location
python3 tools/originals/fetch.py <resources-checkout-or-its-originals-dir>
#   or set the env var named in game.toml (resources_env)

# reconcile the materialized corpus against builds.toml, and prove the gate bites
python3 tools/originals/verify.py
python3 tools/originals/verify.py --self-test
```

With `nix` + `just`:

```sh
nix develop
just bootstrap <resources>
just check
```

## Status

Phase 0 (resource-free foundation) scaffolded. See `docs/STATUS.md` and the
provenance authority `java/reconstruction/builds.toml`. Phase 1 onward follows
`../PLAYBOOK.md`.
