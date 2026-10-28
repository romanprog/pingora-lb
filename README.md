# `pingora-lb` — Educational Rust & Cloudflare Pingora Workspace

A multi-crate **Cargo Workspace** designed for a Go Developer / Cloud Architect learning Rust and building a high-performance HTTP load balancer with [Cloudflare Pingora](https://github.com/cloudflare/pingora).

---

## 1. Repository Structure (Cargo Workspace)

In Go, multi-module repositories use `go.work` or a single `go.mod` with `pkg/` and `cmd/`.
In Rust, the idiomatic equivalent is a **Cargo Workspace** defined in the root [`Cargo.toml`](Cargo.toml). All sub-projects share a single `Cargo.lock` and a single `target/` build cache directory:

```text
pingora-lb/
├── Cargo.toml                  # Workspace root (shared resolver, dependencies, lints)
├── rust-toolchain.toml         # Pins stable Rust + rustfmt, clippy, rust-src, rust-analyzer
├── Makefile                    # Developer workflow shortcuts (check, test, lint, fmt, run-*)
├── crates/                     # Shared library crates (like Go `pkg/` or `internal/`)
│   └── lb-core/                # Core load balancer domain types, errors, traits & RoundRobin
│       ├── Cargo.toml
│       └── src/lib.rs
└── labs/                       # Separated executable sub-projects (like Go `cmd/`)
    ├── 01-rust-basics/         # Lab 01: Base Rust walkthrough (Ownership, Enums, Traits, Arc)
    │   ├── Cargo.toml
    │   └── src/main.rs
    └── 02-pingora-skeleton/    # Lab 02: Pingora ProxyHttp & LoadBalancer lifecycle blueprint
        ├── Cargo.toml
        └── src/main.rs
```

---

## 2. Go vs. Rust Toolchain Cheat Sheet

| Concept / Task | Go (`go`) | Rust (`cargo` / `rustup`) |
| :--- | :--- | :--- |
| Toolchain manager | `go env` / `gvm` | `rustup show`, `rustup update` |
| Module / Package manifest | `go.mod` / `go.sum` | `Cargo.toml` / `Cargo.lock` |
| Multi-project workspace | `go.work` | Root `Cargo.toml` with `[workspace]` |
| Fast type/compile check | `go build ./...` | `cargo check --workspace` |
| Run a specific binary | `go run ./cmd/app` | `cargo run -p lab-01-rust-basics` |
| Run all tests | `go test ./...` | `cargo test --workspace` |
| Code formatter | `gofmt` | `cargo fmt` (`rustfmt`) |
| Static linter | `go vet` / `golangci-lint` | `cargo clippy` |
| Language server (IDE) | `gopls` | `rust-analyzer` |
| Add a dependency | `go get pkg@ver` | `cargo add pkg` |

---

## 3. Machine Configuration Status

### Installed & Ready (`~/.cargo/bin`)
- **Compiler & Package Manager**: `rustc 1.99.1` & `cargo 1.99.1` (`stable-x86_64-unknown-linux-gnu`)
- **Linter & Formatter**: `clippy` and `rustfmt`
- **IDE Support**: `rust-analyzer` and `rust-src` (standard library source for jump-to-definition)

> **Shell Note**: `~/.bashrc` and `~/.profile` already source `"$HOME/.cargo/env"`. If a newly opened non-login shell doesn't find `cargo`, run `source "$HOME/.cargo/env"`.

### System Packages Needed Before Compiling Real `pingora`
Because Cloudflare's `pingora` links against C/C++ TLS libraries (OpenSSL or BoringSSL via `bindgen`), it requires `cmake`, `clang`, `libclang-dev`, `pkg-config`, and `libssl-dev` (similar to `CGO_ENABLED=1` in Go). Run this once in your terminal:

```bash
make install-sys-deps
# or manually:
sudo apt-get update && sudo apt-get install -y cmake clang libclang-dev pkg-config libssl-dev
```

---

## 4. Quick Start

```bash
# 1. Run all unit tests across the workspace
make test

# 2. Run Lab 01 (Base Rust educational walkthrough)
make run-lab01

# 3. Run Lab 02 (Pingora ProxyHttp lifecycle blueprint)
make run-lab02

# 4. Run Clippy linter
make lint
```
