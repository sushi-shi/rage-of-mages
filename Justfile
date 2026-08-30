set shell := ["bash", "-eu", "-o", "pipefail", "-c"]

default:
    @just --list

# --- Fresh clone -------------------------------------------------------------

# Fresh clone to verified: materialize the corpus, then reconcile it. The
# resource location is passed explicitly; it is never baked into the repo (R1).
# Phase 1 adds `classify` + `catalog` here as they land (R13 clean-slate).
bootstrap resources:
    nix run .#fetch-resources -- {{quote(resources)}}
    just originals-verify

# Verify the materialized _originals against builds.toml's sha256/bytes table.
originals-verify:
    python3 tools/originals/verify.py

# Prove the originals-verify gate can fail (playbook R3). Must exit 0.
originals-verify-canfail:
    python3 tools/originals/verify.py --self-test

# Regenerate builds.toml provenance from a resources dir (mechanical facts only;
# the judgment calls stay flagged for Phase 1 — see the file header).
gen-builds resources match:
    python3 tools/originals/gen_builds.py \
        --resources {{quote(resources)}} --match {{quote(match)}} \
        --slug "$(python3 -c 'import tomllib;print(tomllib.load(open("game.toml","rb"))["slug"])')" \
        --title "$(python3 -c 'import tomllib;print(tomllib.load(open("game.toml","rb"))["title"])')" \
        --out java/reconstruction/builds.toml

# --- Phase-2 correctness gates (decompile + reviewed naming) -----------------

# R8 numeric-shape authority: regenerate _reference/numeric-shapes.json (the
# ordered JVM numeric opcodes each baseline method executes) and prove it is
# byte-identical to a fresh regen. The file is git-ignored (derived from
# copyrighted bytecode); a transliterator consults it before porting a method.
numeric-shape:
    python3 tools/java/validate_numeric_shape.py
    python3 tools/java/validate_numeric_shape.py --check

# Prove the numeric-shape gate can fail (playbook R3): perturb one recorded
# opcode in memory and confirm the byte-identical check goes RED. Must exit 0.
numeric-shape-canfail:
    python3 tools/java/validate_numeric_shape.py --self-test

# R10 symbols ledger: assert every named (obf, descriptor) in symbols.toml is a
# REAL member of the baseline bytecode (the ledger cannot drift from reality).
symbols:
    python3 tools/java/validate_symbols.py --check

# Prove the symbols gate can fail (playbook R3): inject a non-existent member in
# memory and confirm it is rejected. Must exit 0.
symbols-canfail:
    python3 tools/java/validate_symbols.py --self-test

# --- Test batteries ----------------------------------------------------------

# Formats corpus + oracles gate: the bounded parsers against every real blob in
# the baseline + EN jars, each guarded by an independent oracle (png-crate
# decode, ideal trig, cross-language record counts, re-derived scenes head).
# Can-fail proof: the in-test negative controls reject one-unit-perturbed blobs.
formats-corpus:
    cargo test -p rage-formats

# Transliteration first-frame gate (R3): boot the strict transliteration
# against the baseline jar and require a REAL rendered frame — the Nival logo,
# non-uniform FrameStats + a pixel-exact rage-formats oracle on the logo box.
# Can-fail proof: in-test blank/uniform/t=0 frames fail the same predicate;
# a missing _originals corpus fails loudly (never skips).
first-frame:
    cargo test -p rage-game-xlat --test first_frame

test:
    if [ -d tools/tests ]; then python3 -m unittest discover -s tools/tests; fi
    if [ -f Cargo.toml ]; then cargo test --workspace; fi

# Every gate the project has today. Grows as phases land; every gate cited here
# must exist and be proven able to fail (playbook R3, R14).
check:
    just originals-verify
    just originals-verify-canfail
    just numeric-shape
    just numeric-shape-canfail
    just symbols
    just symbols-canfail
    if [ -d tools/tests ]; then python3 -m unittest discover -s tools/tests; fi
    if [ -f Cargo.toml ]; then cargo fmt --all --check; fi
    if [ -f Cargo.toml ]; then cargo clippy --workspace --all-targets -- -D warnings; fi
    if [ -f Cargo.toml ]; then cargo test --workspace; fi
    if [ -f Cargo.toml ]; then just formats-corpus; fi
