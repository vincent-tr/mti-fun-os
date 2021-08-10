use core::panic::PanicInfo;

use kernel::{hlt_loop, print, println};

pub trait Testable {
    fn run(&self) -> ();
}

impl<T> Testable for T
where
    T: Fn(),
{
    fn run(&self) {
        print!("{}...\t", core::any::type_name::<T>());
        self();
        println!("[ok]");
    }
}

#[allow(dead_code)]
pub fn test_runner(tests: &[&dyn Testable]) {
    println!("Running {} tests", tests.len());

    for test in tests {
        test.run();
    }

    exit_qemu(QemuExitCode::Success);
}

#[panic_handler]
fn test_panic_handler(info: &PanicInfo) -> ! {
    println!("[failed]\n");
    println!("Error: {}\n", info);

    exit_qemu(QemuExitCode::Failed);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum QemuExitCode {
    Success = 0x10,
    Failed = 0x11,
}

pub fn exit_qemu(exit_code: QemuExitCode) -> ! {
    use x86_64::instructions::port::Port;

    unsafe {
        let mut port = Port::new(0xf4);
        port.write(exit_code as u32);
    }

    hlt_loop();
}
