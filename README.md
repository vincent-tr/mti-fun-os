# mti-fun-os

## Build kernel

```shell
cargo build
```

## Run kernel in QEmu

```shell
pacman -S extra/qemu-base
cargo run
```
to exit: `Ctrl+A X`

## Readings

- https://wiki.osdev.org/Rust
- https://os.phil-opp.com/
- https://crates.io/crates/bootloader
- https://github.com/redox-os/kernel
- https://doc.redox-os.org/book/
- https://web.archive.org/web/20140803112320/http://i30www.ira.uka.de/~neider/edu/mkc/mkc.html
- https://fuchsia.dev/fuchsia-src/reference/kernel_objects/objects