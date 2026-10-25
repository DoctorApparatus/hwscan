set dotenv-load := true

default:
    @just --list

# Type-check, lint, and test.
check:
    cargo fmt --check
    cargo clippy --all-targets -- -D warnings
    cargo test

# Print the scan as JSON; sends nothing.
scan:
    cargo run -q -- scan

# Print the report JSON Schema (the controller contract).
schema:
    cargo run -q -- schema

# Scan and POST to HWSCAN_CONTROLLER_URL (set it in .env).
report:
    cargo run -q -- report

# Static musl binary: copy-and-run on any Linux box.
release-static target="x86_64-unknown-linux-musl":
    rustup target add {{target}}
    cargo build --release --target {{target}}
    @ls -lh target/{{target}}/release/hwscan
