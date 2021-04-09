build: envoy-singleton-service.wasm

envoy-singleton-service.wasm:
	cargo build --target wasm32-unknown-unknown --release
	cp target/wasm32-unknown-unknown/release/singleton_service.wasm ../envoy/singleton_service.wasm

.PHONY: clean
clean:
	rm envoy-singleton-service || true