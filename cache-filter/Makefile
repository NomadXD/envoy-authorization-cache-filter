build: envoy-cache-filter.wasm

envoy-cache-filter.wasm:
	cargo build --target wasm32-unknown-unknown --release
	cp target/wasm32-unknown-unknown/release/cache_filter.wasm ../envoy/cache_filter.wasm

.PHONY: clean
clean:
	rm envoy-cache-filter.wasm || true