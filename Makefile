# Environment variables
export MTI_FUN_OS_KERNEL_PROFILE := release # or dev
export MTI_FUN_OS_KERNEL_TARGET := x86_64-unknown-none
export MTI_FUN_OS_INIT_PROFILE := release
export MTI_FUN_OS_INIT_TARGET := x86_64-mti_fun_os-init
export MTI_FUN_OS_SERVERS_PROFILE := release
export MTI_FUN_OS_SERVERS_TARGET := x86_64-mti_fun_os

.PHONY: all run format build image-build init-build vfs-server-build process-server-build clean

all: run

run: build
	cargo run --profile $(MTI_FUN_OS_KERNEL_PROFILE)

format:
	cargo fmt -- --emit=files

build: format image-build

# also build kernel
image-build: init-build
	cargo build --profile $(MTI_FUN_OS_KERNEL_PROFILE)

init-build: vfs-server-build process-server-build
	cd init && cargo build --profile $(MTI_FUN_OS_INIT_PROFILE)

vfs-server-build:
	cd servers/vfs-server && cargo build --profile $(MTI_FUN_OS_SERVERS_PROFILE)

process-server-build:
	cd servers/process-server && cargo build --profile $(MTI_FUN_OS_SERVERS_PROFILE)

clean:
	cargo clean
	cd init && cargo clean
	cd servers/vfs-server && cargo clean
	cd servers/process-server && cargo clean
