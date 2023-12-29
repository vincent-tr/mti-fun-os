# mti-fun-os

## Setup

```shell
cargo install cargo-make
pacman -S extra/qemu-base
```

## Build image

```shell
cargo make build
```

## Run kernel in QEmu

### Shell 1

```shell
cargo make run
# or cargo make
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

## Next tasks / Notes

- TODO: when last thread of a process stops, close all its handles
- iomem mapping -> strong uncacheable
- IPC
- Exceptions in userland

kernel register message to port (port_handle, kernel_message_type)
kernel unregister

context switch: 
- in syscall, the interrupted thread will go to sleep.
  on wake up, we must set its syscall result before going back to userland
  then the syscall wake up one thread, and we must not overwrite its state with syscall result


TODO WaitQueue

pub enum KernelMessage {
    ProcessCreated(u64),
    ProcesssDeleted(u64),
    ThreadCreated(u64),
    ThreadTerminated(u64),
    ThreadError((u64, ThreadError)),
    MemoryObjectCreated(u64),
    MemoryObjectDeleted(u64),
}
