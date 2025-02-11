.PHONY: test

install:
	cargo install --path .

test:
	cd ../test && \
	fnpm $(cmd)
