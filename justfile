# List available recipes
default:
    @just --list

# Run all checks (fmt, clippy, test)
check:
    cargo fmt --check
    cargo clippy -- -D warnings
    cargo test

# Run tests
test *args:
    cargo test {{args}}

# Build release binary
build:
    cargo build --release

# Run formatting
fmt:
    cargo fmt

# Start docs dev server
docs:
    cd docs && pnpm dev

# Build docs site
docs-build:
    cd docs && pnpm build

# Install diecut locally
install:
    cargo install --path crates/diecut-cli
