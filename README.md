# mti-fun-os

## Overview

**mti-fun-os** is a microkernel-based operating system written in Rust, designed for x86_64 architecture. The project explores modern OS design principles by implementing a minimal kernel that provides only essential services (memory management, scheduling, IPC), while drivers and services run in isolated userspace processes.

Built from scratch as an educational and experimental platform, mti-fun-os demonstrates how Rust's safety guarantees can be leveraged for systems programming without sacrificing performance. The kernel uses QEMU for development and testing, featuring a bootloader-based boot process and a custom runtime library (libruntime) that provides userspace functionality for building applications and services.

The architecture emphasizes capability-based security, with kernel objects accessed through handles and a message-passing IPC system inspired by microkernel designs like Fuchsia and seL4. Device drivers communicate with hardware through a structured PCI interface, supporting device enumeration, configuration space access, and capability-based feature discovery.

### Key Features

- **Microkernel Architecture**: Minimal kernel with services running in userspace
- **Rust Implementation**: Memory-safe systems programming with modern tooling
- **IPC System**: Inter-process communication with handles and message passing
- **PCI Driver Framework**: Device discovery and management with capability support
- **Advanced Memory Management**: Paging, virtual memory, and memory object handling
- **Syscall Interface**: Ring 0/3 transitions with SYSENTER/SYSEXIT support
- **Task Scheduling**: Preemptive multitasking with context switching
- **APIC/Timer Support**: Local APIC timer-based scheduling

## Setup

```shell
# qemu
pacman -S extra/qemu-base # archlinux
sudo apt install qemu-system-x86 # ubuntu
# toolchain
rustup component add rust-src --toolchain nightly-x86_64-unknown-linux-gnu
# cpio image tools
pacman -S cpio
# screenshot tools
pacman -S socat magick # archlinux
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

### Net - e100e driver

- https://wiki.osdev.org/Intel_Ethernet_i217
- https://wiki.osdev.org/Intel_8254x
- https://www.intel.com/content/dam/doc/manual/pci-pci-x-family-gbe-controllers-software-dev-manual.pdf

## Next tasks / Notes

### kernel

- multi-core
- IRQ: userland IRQ supports only MSI
- DMA: no support for low-address memory

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
    -> add static metadata + have post-build step to get mti-fun-os binary format?
    - -C prefer dynamic
- pci-server:
  - PCI bridge suport
  - PCIe
  - MSI-X
- screen/graphics
- net

