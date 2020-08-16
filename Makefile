source_files := $(wildcard src/*.rs)

all: nio_crypto/nio_crypto.so

PHONY: test format

test: nio_crypto/nio_crypto.so
	python3 -m pytest

format:
	rustfmt src/*.rs

nio_crypto/nio_crypto.so: target/debug/nio_crypto.so
	cp target/debug/libnio_crypto.so nio_crypto/nio_crypto.so

target/debug/nio_crypto.so: $(source_files)
	cargo build
