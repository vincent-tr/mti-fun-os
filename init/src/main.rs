#![no_std]
#![no_main]
#![feature(naked_functions)]
#![feature(used_with_arg)]

mod syscalls;

use core::{arch::asm, fmt, mem, panic::PanicInfo};

use syscalls::{syscall1, syscall3, SyscallNumber};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Handle(u64);

impl Handle {
    pub const fn invalid() -> Self {
        Handle(0)
    }
}

#[repr(usize)]
enum Level {
    Error = 1,
    Warn,
    Info,
    Debug,
    Trace,
}

// https://stackoverflow.com/questions/50200268/how-can-i-use-the-format-macro-in-a-no-std-environment
pub mod write_to {
    use core::cmp::min;
    use core::fmt;

    pub struct WriteTo<'a> {
        buffer: &'a mut [u8],
        // on write error (i.e. not enough space in buffer) this grows beyond
        // `buffer.len()`.
        used: usize,
    }

    impl<'a> WriteTo<'a> {
        pub fn new(buffer: &'a mut [u8]) -> Self {
            WriteTo { buffer, used: 0 }
        }

        pub fn as_str(self) -> Option<&'a str> {
            if self.used <= self.buffer.len() {
                // only successful concats of str - must be a valid str.
                use core::str::from_utf8_unchecked;
                Some(unsafe { from_utf8_unchecked(&self.buffer[..self.used]) })
            } else {
                None
            }
        }
    }

    impl<'a> fmt::Write for WriteTo<'a> {
        fn write_str(&mut self, s: &str) -> fmt::Result {
            if self.used > self.buffer.len() {
                return Err(fmt::Error);
            }
            let remaining_buf = &mut self.buffer[self.used..];
            let raw_s = s.as_bytes();
            let write_num = min(raw_s.len(), remaining_buf.len());
            remaining_buf[..write_num].copy_from_slice(&raw_s[..write_num]);
            self.used += raw_s.len();
            if write_num < raw_s.len() {
                Err(fmt::Error)
            } else {
                Ok(())
            }
        }
    }

    pub fn show<'a>(buffer: &'a mut [u8], args: fmt::Arguments) -> Result<&'a str, fmt::Error> {
        let mut w = WriteTo::new(buffer);
        fmt::write(&mut w, args)?;
        w.as_str().ok_or(fmt::Error)
    }
}

fn log(level: Level, args: fmt::Arguments) {
    let mut buf: [u8; 1024] = [0u8; 1024];

    let message: &str = write_to::show(&mut buf, args).unwrap();

    unsafe {
        syscall3(
            SyscallNumber::Log,
            level as usize,
            message.as_ptr() as usize,
            message.len(),
        )
    };
}

/// # Safety
///
/// Borrowing rules unchecked. Do right before syscalls only.
unsafe fn out_ptr<T>(value: &mut T) -> usize {
    let ptr: *mut T = value;
    mem::transmute(ptr)
}

mod offsets {
    use core::ops::Range;

    extern "C" {
        // text (R-X)
        static __text_start: u8;
        static __text_end: u8;
        // rodata (R--)
        static __rodata_start: u8;
        static __rodata_end: u8;
        // data (RW-)
        static __data_start: u8;
        static __data_end: u8;
        static __bss_start: u8;
        static __bss_end: u8;

        static __end: u8;

        // stack in RW data
        static __init_stack_start: u8;
        pub static __init_stack_end: u8;
    }

    pub fn text() -> Range<usize> {
        unsafe {
            let start = &__text_start as *const u8 as usize;
            let end = &__text_end as *const u8 as usize;
            start..end
        }
    }

    pub fn rodata() -> Range<usize> {
        unsafe {
            let start = &__rodata_start as *const u8 as usize;
            let end = &__rodata_end as *const u8 as usize;
            start..end
        }
    }

    pub fn data() -> Range<usize> {
        unsafe {
            let start = &__data_start as *const u8 as usize;
            let end = &__data_end as *const u8 as usize;
            start..end
        }
    }

    pub fn stack_top() -> usize {
        unsafe { &__init_stack_end as *const u8 as usize }
    }
}


#[naked]
#[no_mangle]
pub unsafe extern "C" fn user_start() {
    core::arch::asm!(
        "
        lea rsp, {stack}
        mov rbp, rsp

        call {main}
        # `start` must never return.
        ud2
        ",
        stack = sym offsets::__init_stack_end,
        main = sym main,
        options(noreturn),
    );
}


// Force at least one data, so that it is laid out after bss in linker script
// This force bss allocation in binary file
#[used(linker)]
static mut FORCE_DATA_SECTION: u8 = 0x42;

extern "C" fn main() -> ! {
    // TODO: protection

    log(
        Level::Info,
        format_args!(
            "text: {:016X} -> {:016X} (size={})",
            offsets::text().start,
            offsets::text().end,
            offsets::text().end - offsets::text().start
        ),
    );
    log(
        Level::Info,
        format_args!(
            "rodata: {:016X} -> {:016X} (size={})",
            offsets::rodata().start,
            offsets::rodata().end,
            offsets::rodata().end - offsets::rodata().start
        ),
    );
    log(
        Level::Info,
        format_args!(
            "data: {:016X} -> {:016X} (size={})",
            offsets::data().start,
            offsets::data().end,
            offsets::data().end - offsets::data().start
        ),
    );
    log(
        Level::Info,
        format_args!("stack_top: {:016X}", offsets::stack_top()),
    );

    log(Level::Info, format_args!("test"));

    unsafe {
        let mut handle = Handle::invalid();
        syscall1(SyscallNumber::ProcessOpenSelf, out_ptr(&mut handle));

        log(Level::Info, format_args!("handle value={handle:?}"));

        syscall1(SyscallNumber::Close, handle.0 as usize);
    }

    loop {}
}

#[inline]
fn debugbreak() {
    unsafe {
        asm!("int3", options(nomem, nostack));
    }
}

#[inline]
fn page_fault() {
    let ptr = 0x42 as *mut u8;
    unsafe { *ptr = 42 };
}

#[allow(unconditional_panic)]
#[inline]
fn div0() {
    // div / 0
    let _ = 42 / 0;
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    //error!("PANIC: {info}");
    log(
        Level::Error,
        format_args!(
            "PANIC: {info}"
        ),
    );

    loop {}
}
