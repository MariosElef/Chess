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
