#![no_std]
#![no_main]

extern crate rlibc;

mod syscalls;

use core::{panic::PanicInfo, arch::asm, fmt, mem};

use syscalls::{SyscallNumber, syscall3, syscall1};

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

    let message: &str = write_to::show(
        &mut buf,
        args,
    ).unwrap();

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

#[no_mangle]
pub extern "C" fn _start() -> ! {
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
    loop {}
    //error!("PANIC: {info}");
}
