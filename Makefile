.phony: format lint

format:
	@rustup run nightly cargo fmt

lint:
	@rustup run nightly cargo clippy --features="binary"
