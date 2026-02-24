# Environment variables
export MTI_FUN_OS_KERNEL_PROFILE := release # or dev
export MTI_FUN_OS_KERNEL_TARGET := x86_64-unknown-none
export MTI_FUN_OS_INIT_PROFILE := release
export MTI_FUN_OS_INIT_TARGET := x86_64-mti_fun_os-init
export MTI_FUN_OS_SERVERS_PROFILE := release
export MTI_FUN_OS_SERVERS_TARGET := x86_64-mti_fun_os
export BUILD_ARGS := -Zjson-target-spec

.PHONY: all run format build image-build init-build process-server-build time-server-build vfs-server-build memfs-server-build display-server-build clean screenshot

all: run

run: build
	cargo run $(BUILD_ARGS) --profile $(MTI_FUN_OS_KERNEL_PROFILE)

format:
	cargo fmt -- --emit=files

build: format image-build

# also build kernel
image-build: init-build
	cargo build $(BUILD_ARGS) --profile $(MTI_FUN_OS_KERNEL_PROFILE)

init-build: process-server-build time-server-build vfs-server-build memfs-server-build display-server-build
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

clean:
	cargo clean
	cd init && cargo clean
	cd servers/process-server && cargo clean
	cd servers/vfs-server && cargo clean
	cd servers/memfs-server && cargo clean
	cd servers/display-server && cargo clean

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
