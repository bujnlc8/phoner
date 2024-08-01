test:
	cargo test -- --nocapture


PLATFORMS:=x86_64-apple-darwin x86_64-unknown-linux-gnu x86_64-unknown-linux-musl aarch64-apple-darwin
VERSION:=0.1.1
CLI:=phoner
HOST_TRIPLE := $(shell rustc -vV | grep 'host:' | awk '{print $$2}')

all: $(PLATFORMS)

x86_64-apple-darwin:
	cargo build --release --locked

x86_64-unknown-linux-musl:
	TARGET_CC=x86_64-linux-musl-cc cargo build --release --target x86_64-unknown-linux-musl --locked

x86_64-unknown-linux-gnu:
	TARGET_CC=x86_64-linux-gnu-cc cargo build --release --target x86_64-unknown-linux-gnu --locked

aarch64-apple-darwin:
	cargo build --release --target aarch64-apple-darwin --locked

artifacts:all
	@mkdir -p artifacts/$(VERSION)
	@for platform in $(PLATFORMS); do \
		if [ "$$platform" == "$(HOST_TRIPLE)" ]; then \
			chmod +x target/release/$(CLI); \
			mkdir /tmp/$(CLI); \
			cp target/release/$(CLI) /tmp/$(CLI); \
			tar zcvf $(CLI)_$$platform.tar.gz  -C /tmp $(CLI); \
			rm -rf /tmp/$(CLI); \
		else \
			chmod +x target/$$platform/release/$(CLI); \
			mkdir /tmp/$(CLI); \
			cp target/$$platform/release/$(CLI) /tmp/$(CLI); \
			tar zcvf $(CLI)_$$platform.tar.gz -C /tmp $(CLI); \
			rm -rf /tmp/$(CLI); \
		fi; \
		mv $(CLI)_$$platform.tar.gz artifacts/$(VERSION); \
		md5 -q artifacts/$(VERSION)/$(CLI)_$$platform.tar.gz > artifacts/$(VERSION)/$(CLI)_$$platform.tar.gz.md5; \
	done
	@echo "generate artifacts done !!!"

.PHONY: artifacts
