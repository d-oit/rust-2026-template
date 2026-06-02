.PHONY: all docs docs-check ci fmt clippy test build install-doc-tools

all: ci

ci: fmt clippy test build

fmt:
	cargo fmt --all -- --check

clippy:
	cargo clippy --workspace -- -D warnings

test:
	cargo test --workspace

build:
	cargo build --workspace

install-doc-tools: ## Install pinned versions of doc generation tools (run once)
	cargo install cargo-sync-readme --version 1.1.0
	cargo install cargo-doc2readme --version 0.4.0

docs: ## Auto-generate docs/patterns/ from rustdoc comments (run install-doc-tools first)
	mkdir -p docs/patterns
	cargo sync-readme
	cargo doc2readme -p example-storage-pattern --out docs/patterns/trait-only-storage.md
	cargo doc2readme -p example-registry-pattern --out docs/patterns/registry-dispatch.md
	@# Add AUTO-GENERATED header if not already present
	@for f in docs/patterns/*.md; do \
		if ! grep -q "AUTO-GENERATED" "$$f"; then \
			sed -i '1i <!-- AUTO-GENERATED — edit src/lib.rs in the crate, not this file -->\n' "$$f"; \
		fi \
	done

docs-check: docs ## Fail if generated docs differ from committed versions
	cargo sync-readme --check
	git diff --exit-code docs/patterns/
