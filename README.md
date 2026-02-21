# mti-fun-os

## Setup

```shell
pacman -S extra/qemu-base # archlinux
sudo apt install qemu-system-x86 # ubuntu
rustup component add rust-src --toolchain nightly-x86_64-unknown-linux-gnu
```

## Build image

```shell
make build
```

## Run kernel in QEmu

### Shell 1

```shell
make run
# or make
```

### Shell 2

```shell
tail -f serial.log
```

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
- https://gitlab.redox-os.org/redox-os/kernel/-/blob/master/src/arch/x86_64/interrupt/syscall.rs

### Context switch

- https://wiki.osdev.org/Kernel_Multitasking

### PIT

- https://wiki.osdev.org/Programmable_Interval_Timer

### APIC/Timer

- https://wiki.osdev.org/APIC
- https://wiki.osdev.org/APIC_Timer
- https://gitlab.redox-os.org/redox-os/kernel/-/blob/master/src/arch/x86_64/device/local_apic.rs
- https://github.com/rust-osdev/apic

### The holy bible

- ( [downloaded](docs/64-ia-32-architectures-software-developer-vol-3a-part-1-manual.pdf) ) https://www.intel.com/content/dam/www/public/us/en/documents/manuals/64-ia-32-architectures-software-developer-vol-3a-part-1-manual.pdf

### Futex

- https://github.com/eliben/code-for-blog/blob/master/2018/futex-basics/mutex-using-futex.cpp
- https://man7.org/linux/man-pages/man2/futex.2.html

## Next tasks / Notes

### kernel

- multi-core
- iomem to userland
- irq to userland

### runtime

- add guards hits to "page fault of interest" (+ auto grow of stack)
- object-oriented TLS

### servers

- process-server:
  - dynamic linking:
    - https://en.wikipedia.org/wiki/Executable_and_Linkable_Format
    - https://wiki.osdev.org/Dynamic_Linker
    - https://refspecs.linuxbase.org/elf/gabi4+/ch5.dynamic.html
    - https://www.openbsd.org/papers/nycbsdcon08-pie/
    - https://stackoverflow.com/questions/50303305/elf-file-type-et-exec-and-et-dyn
    - ( [downloaded](docs/ELF_Format.pdf) ) http://www.skyfree.org/linux/references/ELF_Format.pdf
    - https://gitlab.redox-os.org/redox-os/relibc/-/tree/master/src/ld_so
    -> build PE + dllimport/dllexport ?
    -> rajouter de la metadata static + faire un post-build pour avoir un format de binaire mti-os ?
    - -C prefer dynamic
- vfs
  - archive fs
- RTC/time server (kernel: ioport)
- screen/graphics (kernel: iomem)
- find something simple to have kernel irq and kernel DMA
- net

