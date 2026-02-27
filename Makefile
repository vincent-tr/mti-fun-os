# Environment variables
# Kernel profile: release or dev
export MTI_FUN_OS_KERNEL_PROFILE := release
export MTI_FUN_OS_KERNEL_TARGET := x86_64-unknown-none
export MTI_FUN_OS_INIT_PROFILE := release
export MTI_FUN_OS_INIT_TARGET := x86_64-mti_fun_os-init
export MTI_FUN_OS_SERVERS_PROFILE := release
export MTI_FUN_OS_SERVERS_TARGET := x86_64-mti_fun_os
export BUILD_ARGS := -Zjson-target-spec

.PHONY: all run format build image-build init-build process-server-build time-server-build vfs-server-build memfs-server-build display-server-build archivefs-server-build pci-server-build boot.cpio clean screenshot

all: run

run: build
	cargo run $(BUILD_ARGS) --profile $(MTI_FUN_OS_KERNEL_PROFILE)

format:
	cargo fmt -- --emit=files

build: format image-build

# also build kernel
image-build: boot.cpio
	cargo build $(BUILD_ARGS) --profile $(MTI_FUN_OS_KERNEL_PROFILE)

boot.cpio: init-build process-server-build time-server-build vfs-server-build memfs-server-build display-server-build archivefs-server-build pci-server-build
	@echo "Creating boot.cpio archive..."
	@TMPDIR=$$(mktemp -d); \
	mkdir -p $$TMPDIR/servers; \
	mkdir -p target/$(MTI_FUN_OS_KERNEL_PROFILE); \
	cp target/$(MTI_FUN_OS_INIT_TARGET)/$(MTI_FUN_OS_INIT_PROFILE)/init $$TMPDIR/init; \
	cp target/$(MTI_FUN_OS_SERVERS_TARGET)/$(MTI_FUN_OS_SERVERS_PROFILE)/process-server $$TMPDIR/servers/process-server; \
	cp target/$(MTI_FUN_OS_SERVERS_TARGET)/$(MTI_FUN_OS_SERVERS_PROFILE)/time-server $$TMPDIR/servers/time-server; \
	cp target/$(MTI_FUN_OS_SERVERS_TARGET)/$(MTI_FUN_OS_SERVERS_PROFILE)/vfs-server $$TMPDIR/servers/vfs-server; \
	cp target/$(MTI_FUN_OS_SERVERS_TARGET)/$(MTI_FUN_OS_SERVERS_PROFILE)/memfs-server $$TMPDIR/servers/memfs-server; \
	cp target/$(MTI_FUN_OS_SERVERS_TARGET)/$(MTI_FUN_OS_SERVERS_PROFILE)/display-server $$TMPDIR/servers/display-server; \
	cp target/$(MTI_FUN_OS_SERVERS_TARGET)/$(MTI_FUN_OS_SERVERS_PROFILE)/archivefs-server $$TMPDIR/servers/archivefs-server; \
	cp target/$(MTI_FUN_OS_SERVERS_TARGET)/$(MTI_FUN_OS_SERVERS_PROFILE)/pci-server $$TMPDIR/servers/pci-server; \
	cd $$TMPDIR && find . -depth -print | cpio -o -H newc > $(CURDIR)/target/$(MTI_FUN_OS_KERNEL_PROFILE)/boot.cpio; \
	rm -rf $$TMPDIR
	@echo "Boot archive created: target/$(MTI_FUN_OS_KERNEL_PROFILE)/boot.cpio"

init-build:
	cd init && cargo build $(BUILD_ARGS) --profile $(MTI_FUN_OS_INIT_PROFILE)

process-server-build:
	cd servers/process-server && cargo build $(BUILD_ARGS) --profile $(MTI_FUN_OS_SERVERS_PROFILE)

time-server-build:
	cd servers/time-server && cargo build $(BUILD_ARGS) --profile $(MTI_FUN_OS_SERVERS_PROFILE)

vfs-server-build:
	cd servers/vfs-server && cargo build $(BUILD_ARGS) --profile $(MTI_FUN_OS_SERVERS_PROFILE)

memfs-server-build:
	cd servers/memfs-server && cargo build $(BUILD_ARGS) --profile $(MTI_FUN_OS_SERVERS_PROFILE)

display-server-build:
	cd servers/display-server && cargo build $(BUILD_ARGS) --profile $(MTI_FUN_OS_SERVERS_PROFILE)

archivefs-server-build:
	cd servers/archivefs-server && cargo build $(BUILD_ARGS) --profile $(MTI_FUN_OS_SERVERS_PROFILE)

pci-server-build:
	cd servers/pci-server && cargo build $(BUILD_ARGS) --profile $(MTI_FUN_OS_SERVERS_PROFILE)

clean:
	cargo clean
	cd init && cargo clean
	cd servers/process-server && cargo clean
	cd servers/vfs-server && cargo clean
	cd servers/memfs-server && cargo clean
	cd servers/display-server && cargo clean
	cd servers/archivefs-server && cargo clean
	cd servers/pci-server && cargo clean
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
