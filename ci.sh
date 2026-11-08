#!/bin/sh
set -eu

cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
# The backend's database tests make their own throwaway databases on this server.
export DATABASE_URL="${DATABASE_URL:-postgres://chess:change-me@localhost:5433/chess}"
cargo test --workspace --locked
cargo deny check

# The domain stays pure.
if grep -rnE 'gpui|axum|sqlx' crates/chess-domain/src; then exit 1; fi

# Backend layers: routes → services → repositories → models, sqlx only in repositories.
(cd backend && ast-grep test --skip-snapshot-tests && ast-grep scan)

# Desktop layers: views → components → services → clients → models → enums. Each rule proves
# itself first, then the code is scanned.
(cd desktop && ast-grep test --skip-snapshot-tests && ast-grep scan)

# Every source file is declared by a module, so none is silently left out of the build.
lint/orphans.sh desktop/src main.rs
lint/orphans.sh crates/chess-domain/src lib.rs
lint/orphans.sh backend/src lib.rs
