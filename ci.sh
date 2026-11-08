#!/bin/sh
set -eu

cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo deny check

# Layers: SQL stays in repositories, outbound clients stay behind services, the domain stays pure.
if grep -rnE 'sqlx::' backend/src/routes backend/src/services; then exit 1; fi
if grep -rn 'crate::clients::' backend/src/routes; then exit 1; fi
if grep -rnE 'gpui|axum|sqlx' crates/chess-domain/src; then exit 1; fi

# Desktop layers: views → components → services → clients → models → enums. Each rule proves
# itself first, then the code is scanned.
(cd desktop && ast-grep test --skip-snapshot-tests && ast-grep scan)

# Every source file is declared by a module, so none is silently left out of the build.
lint/orphans.sh desktop/src main.rs
lint/orphans.sh crates/chess-domain/src lib.rs
