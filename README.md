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

- http://sos.enix.org/fr/SOSDownload

### Rust

- https://wiki.osdev.org/Rust
- https://os.phil-opp.com/
- https://crates.io/crates/bootloader
- https://github.com/redox-os/kernel
- https://doc.redox-os.org/book/

### Microkernel design

- https://web.archive.org/web/20140803112320/http://i30www.ira.uka.de/~neider/edu/mkc/mkc.html
- https://fuchsia.dev/fuchsia-src/reference/kernel_objects/objects

### Pagination

- http://sos.enix.org/wiki-fr/upload/SOSDownload/sos-texte-art4.pdf
- https://wiki.osdev.org/Paging
- https://os.phil-opp.com/paging-introduction/

### Embed init

- https://doc.rust-lang.org/std/macro.include_bytes.html

### Syscalls

- https://wiki.osdev.org/Global_Descriptor_Table
- https://wiki.osdev.org/GDT_Tutorial
- https://wiki.osdev.org/System_Calls
- https://wiki.osdev.org/Getting_to_Ring_3
- https://wiki.osdev.org/SYSENTER
- https://wiki.osdev.org/Segmentation
- https://github.com/jasonwhite/syscalls
- https://nfil.dev/kernel/rust/coding/rust-kernel-to-userspace-and-back/#syscall-and-sysret
- https://nfil.dev/kernel/rust/coding/rust-kernel-task-scheduler/#building-a-task-struct

### Context switch

- https://wiki.osdev.org/Kernel_Multitasking
