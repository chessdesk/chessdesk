set windows-shell := ["cmd", "/c"]

run *args:
    cargo run -- {{ args }}

release:
    cargo build --release

# Fully optimized, as the release workflow builds it. Takes a while.
dist:
    cargo build --profile dist

fmt:
    cargo fmt --all

lint:
    cargo clippy --workspace --all-targets -- -D warnings

test:
    cargo test --workspace

# Licences, advisories and sources of every dependency, as set in deny.toml.
deny:
    cargo deny check

check: fmt lint test deny

# Installs the git hooks of prek.toml, once per clone.
hooks:
    prek install
