.PHONY: help check test lint fmt run-lab01 run-lab02 run-lab03 install-sys-deps

CARGO := $(shell which cargo 2>/dev/null || echo "$(HOME)/.cargo/bin/cargo")

help: ## Show available make targets
	@grep -E '^[a-zA-Z0-9_-]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-18s\033[0m %s\n", $$1, $$2}'

check: ## Fast compile check across all workspace sub-projects (like `go vet`)
	$(CARGO) check --workspace --all-targets

test: ## Run all unit & integration tests across the workspace (like `go test ./...`)
	$(CARGO) test --workspace

lint: ## Run Clippy static analyzer across the workspace (like `golangci-lint`)
	$(CARGO) clippy --workspace --all-targets -- -D warnings

fmt: ## Format all Rust code using rustfmt (like `gofmt -w .`)
	$(CARGO) fmt --all

run-lab01: ## Run Lab 01: Base Rust fundamentals for Load Balancing
	$(CARGO) run -p lab-01-rust-basics

run-lab02: ## Run Lab 02: Cloudflare Pingora ProxyHttp blueprint
	$(CARGO) run -p lab-02-pingora-skeleton

run-lab03: ## Run Lab 03: Playground
	$(CARGO) run -p lab-03-playground

install-sys-deps: ## Install Linux C/C++ packages needed to compile Pingora (BoringSSL/OpenSSL)
	sudo apt-get update && sudo apt-get install -y cmake clang libclang-dev pkg-config libssl-dev
