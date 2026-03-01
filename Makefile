# Environment variables
# Kernel profile: release or dev
export MTI_FUN_OS_KERNEL_PROFILE := release
export MTI_FUN_OS_KERNEL_TARGET := x86_64-unknown-none
export MTI_FUN_OS_INIT_PROFILE := release
export MTI_FUN_OS_INIT_TARGET := x86_64-mti_fun_os-init
export MTI_FUN_OS_SERVERS_PROFILE := release
export MTI_FUN_OS_SERVERS_TARGET := x86_64-mti_fun_os
export BUILD_ARGS := -Zjson-target-spec

.PHONY: all run format build image-build init-build process-server-build time-server-build vfs-server-build memfs-server-build display-server-build archivefs-server-build pci-server-build e1000-server-build net-server-build boot.cpio clean screenshot

all: run

run: build
	cargo run $(BUILD_ARGS) --profile $(MTI_FUN_OS_KERNEL_PROFILE)

format:
	cargo fmt -- --emit=files

build: format image-build

# also build kernel
image-build: boot.cpio
	cargo build $(BUILD_ARGS) --profile $(MTI_FUN_OS_KERNEL_PROFILE)

boot.cpio: init-build process-server-build time-server-build vfs-server-build memfs-server-build display-server-build archivefs-server-build pci-server-build e1000-server-build net-server-build
	@echo "Creating boot.cpio archive..."
	@TMPDIR=$$(mktemp -d); \
	mkdir -p $$TMPDIR/servers/core; \
	mkdir -p $$TMPDIR/servers/fs; \
	mkdir -p $$TMPDIR/servers/bus; \
	mkdir -p $$TMPDIR/servers/drivers/net; \
	mkdir -p $$TMPDIR/servers/net; \
	mkdir -p target/$(MTI_FUN_OS_KERNEL_PROFILE); \
	cp target/$(MTI_FUN_OS_INIT_TARGET)/$(MTI_FUN_OS_INIT_PROFILE)/init $$TMPDIR/init; \
	cp target/$(MTI_FUN_OS_SERVERS_TARGET)/$(MTI_FUN_OS_SERVERS_PROFILE)/process-server $$TMPDIR/servers/core/process-server; \
	cp target/$(MTI_FUN_OS_SERVERS_TARGET)/$(MTI_FUN_OS_SERVERS_PROFILE)/time-server $$TMPDIR/servers/core/time-server; \
	cp target/$(MTI_FUN_OS_SERVERS_TARGET)/$(MTI_FUN_OS_SERVERS_PROFILE)/vfs-server $$TMPDIR/servers/core/vfs-server; \
	cp target/$(MTI_FUN_OS_SERVERS_TARGET)/$(MTI_FUN_OS_SERVERS_PROFILE)/display-server $$TMPDIR/servers/core/display-server; \
	cp target/$(MTI_FUN_OS_SERVERS_TARGET)/$(MTI_FUN_OS_SERVERS_PROFILE)/memfs-server $$TMPDIR/servers/fs/memfs-server; \
	cp target/$(MTI_FUN_OS_SERVERS_TARGET)/$(MTI_FUN_OS_SERVERS_PROFILE)/archivefs-server $$TMPDIR/servers/fs/archivefs-server; \
	cp target/$(MTI_FUN_OS_SERVERS_TARGET)/$(MTI_FUN_OS_SERVERS_PROFILE)/pci-server $$TMPDIR/servers/bus/pci-server; \
	cp target/$(MTI_FUN_OS_SERVERS_TARGET)/$(MTI_FUN_OS_SERVERS_PROFILE)/e1000-server $$TMPDIR/servers/drivers/net/e1000-server; \
	cp target/$(MTI_FUN_OS_SERVERS_TARGET)/$(MTI_FUN_OS_SERVERS_PROFILE)/net-server $$TMPDIR/servers/net/net-server; \
	cd $$TMPDIR && find . -depth -print | cpio -o -H newc > $(CURDIR)/target/$(MTI_FUN_OS_KERNEL_PROFILE)/boot.cpio; \
	rm -rf $$TMPDIR
	@echo "Boot archive created: target/$(MTI_FUN_OS_KERNEL_PROFILE)/boot.cpio"

init-build:
	cd init && cargo build $(BUILD_ARGS) --profile $(MTI_FUN_OS_INIT_PROFILE)

process-server-build:
	cd servers/core/process-server && cargo build $(BUILD_ARGS) --profile $(MTI_FUN_OS_SERVERS_PROFILE)

time-server-build:
	cd servers/core/time-server && cargo build $(BUILD_ARGS) --profile $(MTI_FUN_OS_SERVERS_PROFILE)

vfs-server-build:
	cd servers/core/vfs-server && cargo build $(BUILD_ARGS) --profile $(MTI_FUN_OS_SERVERS_PROFILE)

memfs-server-build:
	cd servers/fs/memfs-server && cargo build $(BUILD_ARGS) --profile $(MTI_FUN_OS_SERVERS_PROFILE)

display-server-build:
	cd servers/core/display-server && cargo build $(BUILD_ARGS) --profile $(MTI_FUN_OS_SERVERS_PROFILE)

archivefs-server-build:
	cd servers/fs/archivefs-server && cargo build $(BUILD_ARGS) --profile $(MTI_FUN_OS_SERVERS_PROFILE)

pci-server-build:
	cd servers/bus/pci-server && cargo build $(BUILD_ARGS) --profile $(MTI_FUN_OS_SERVERS_PROFILE)

e1000-server-build:
	cd servers/drivers/net/e1000-server && cargo build $(BUILD_ARGS) --profile $(MTI_FUN_OS_SERVERS_PROFILE)

net-server-build:
	cd servers/net/net-server && cargo build $(BUILD_ARGS) --profile $(MTI_FUN_OS_SERVERS_PROFILE)

clean:
	cargo clean
	cd init && cargo clean
	cd servers/core/process-server && cargo clean
	cd servers/core/vfs-server && cargo clean
	cd servers/core/time-server && cargo clean
	cd servers/core/display-server && cargo clean
	cd servers/fs/memfs-server && cargo clean
	cd servers/fs/archivefs-server && cargo clean
	cd servers/bus/pci-server && cargo clean
	cd servers/drivers/net/e1000-server && cargo clean
	cd servers/net/net-server && cargo clean
	rm -f target/*/boot.cpio

screenshot:
	@command -v socat >/dev/null 2>&1 || (echo "Error: socat is not installed. Install with: sudo apt install socat" && exit 1)
	@command -v magick >/dev/null 2>&1 || (echo "Error: ImageMagick is not installed. Install with: sudo apt install imagemagick" && exit 1)
	@echo "Taking screenshot..."
	@(echo '{"execute":"qmp_capabilities"}'; echo '{"execute":"screendump", "arguments":{"filename":"/tmp/screenshot.ppm"}}') | socat - UNIX-CONNECT:/tmp/qmp-socket >/dev/null 2>&1 || (echo "Error: QEMU not running or QMP socket not available." && exit 1)
	@sleep 0.2
	@test -f /tmp/screenshot.ppm || (echo "Error: Screenshot file was not created by QEMU" && exit 1)
	@magick /tmp/screenshot.ppm /tmp/screenshot.png 2>/dev/null
	@echo "Screenshot saved to /tmp/screenshot.png"
	@code /tmp/screenshot.png
