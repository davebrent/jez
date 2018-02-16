.phony: lint

lint:
	@rustup run nightly cargo clippy --features="binary"
