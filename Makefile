# Environment variables
# Kernel profile: release or dev
export MTI_FUN_OS_KERNEL_PROFILE := release
export MTI_FUN_OS_KERNEL_TARGET := x86_64-unknown-none
export MTI_FUN_OS_INIT_PROFILE := release
export MTI_FUN_OS_INIT_TARGET := x86_64-mti_fun_os-init
export MTI_FUN_OS_SERVERS_PROFILE := release
export MTI_FUN_OS_SERVERS_TARGET := x86_64-mti_fun_os
export BUILD_ARGS := -Zjson-target-spec

.PHONY: all run net-install net-uninstall debug logs gdb format build build-relwithdebinfo image-build init-build process-server-build time-server-build vfs-server-build memfs-server-build display-server-build archivefs-server-build pci-server-build edu-server-build e1000e-server-build net-server-build boot.cpio clean screenshot

all: run

net-install:
	@if [ "$$(id -u)" != "0" ]; then \
		echo "Error: net-install requires root (sudo)"; \
		exit 1; \
	fi
	@if ! ip link show tap0 &>/dev/null; then \
		echo "Creating TAP device tap0 for user $(SUDO_USER)..."; \
		ip tuntap add dev tap0 mode tap user $(SUDO_USER); \
		echo "✓ TAP device created and owned by $(SUDO_USER)"; \
	else \
		echo "✓ TAP device tap0 already exists"; \
	fi
	@if ! ip link show br0 &>/dev/null; then \
		echo "Error: Bridge br0 not found"; \
		exit 1; \
	fi
	@echo "Attaching tap0 to br0..."
	ip link set tap0 up
	ip link set tap0 master br0
	@echo "✓ tap0 attached to br0"

net-uninstall:
	@if [ "$$(id -u)" != "0" ]; then \
		echo "Error: net-uninstall requires root (sudo)"; \
		exit 1; \
	fi
	ip link set tap0 down 2>/dev/null || true
	ip tuntap del dev tap0 mode tap 2>/dev/null || true
	@echo "✓ TAP device removed"

run: build
	cargo run $(BUILD_ARGS) --profile $(MTI_FUN_OS_KERNEL_PROFILE)

build-relwithdebinfo: export MTI_FUN_OS_KERNEL_PROFILE := relwithdebinfo
build-relwithdebinfo: export MTI_FUN_OS_INIT_PROFILE := relwithdebinfo
build-relwithdebinfo: export MTI_FUN_OS_SERVERS_PROFILE := relwithdebinfo
build-relwithdebinfo: build

debug: build-relwithdebinfo
	MTI_FUN_OS_DEBUG=1 cargo run $(BUILD_ARGS) --profile relwithdebinfo

logs:
	tail -f serial.log

gdb: build-relwithdebinfo
	@echo "Connecting GDB to QEMU on localhost:1234..."
	@echo "Use 'c' to continue execution, 'Ctrl-C' then 'quit' to exit"
	@KERNEL=$$(ls target/x86_64-unknown-none/relwithdebinfo/deps/artifact/kernel-*/bin/kernel-* 2>/dev/null | grep -v '\.d$$' | head -n1); \
	gdb \
		-ex "set confirm off" \
		-ex "target remote localhost:1234" \
		-ex "add-symbol-file $$KERNEL -o 0xffff800000000000"

format:
	cargo fmt -- --emit=files

build: format image-build

# also build kernel
image-build: boot.cpio
	cargo build $(BUILD_ARGS) --profile $(MTI_FUN_OS_KERNEL_PROFILE)

boot.cpio: init-build process-server-build time-server-build vfs-server-build memfs-server-build display-server-build archivefs-server-build pci-server-build edu-server-build e1000e-server-build net-server-build
	@echo "Creating boot.cpio archive..."
	@TMPDIR=$$(mktemp -d); \
	mkdir -p $$TMPDIR/servers/core; \
	mkdir -p $$TMPDIR/servers/fs; \
	mkdir -p $$TMPDIR/servers/drivers/bus; \
	mkdir -p $$TMPDIR/servers/drivers/test/edu; \
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
	cp target/$(MTI_FUN_OS_SERVERS_TARGET)/$(MTI_FUN_OS_SERVERS_PROFILE)/pci-server $$TMPDIR/servers/drivers/bus/pci-server; \
	cp target/$(MTI_FUN_OS_SERVERS_TARGET)/$(MTI_FUN_OS_SERVERS_PROFILE)/edu-server $$TMPDIR/servers/drivers/test/edu/edu-server; \
	cp target/$(MTI_FUN_OS_SERVERS_TARGET)/$(MTI_FUN_OS_SERVERS_PROFILE)/e1000e-server $$TMPDIR/servers/drivers/net/e1000e-server; \
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
	cd servers/drivers/bus/pci-server && cargo build $(BUILD_ARGS) --profile $(MTI_FUN_OS_SERVERS_PROFILE)

edu-server-build:
	cd servers/drivers/test/edu/edu-server && cargo build $(BUILD_ARGS) --profile $(MTI_FUN_OS_SERVERS_PROFILE)

e1000e-server-build:
	cd servers/drivers/net/e1000e-server && cargo build $(BUILD_ARGS) --profile $(MTI_FUN_OS_SERVERS_PROFILE)

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
	cd servers/drivers/bus/pci-server && cargo clean
	cd servers/drivers/test/edu/edu-server && cargo clean
	cd servers/drivers/net/e1000e-server && cargo clean
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
