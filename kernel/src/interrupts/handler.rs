// Inspired from:
// - https://gitlab.redox-os.org/redox-os/kernel/-/blob/master/src/arch/x86_64/interrupt/handler.rs
// - https://gitlab.redox-os.org/redox-os/kernel/-/blob/master/src/arch/x86_64/interrupt/syscall.rs

use core::cell::RefCell;
use core::{fmt, mem::size_of};

use log::debug;
use x86_64::{registers::model_specific::KernelGsBase, structures::idt::InterruptStackFrameValue};

use crate::memory::{StaticKernelStack, VirtAddr};

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct InterruptStack {
    pub preserved: PreservedRegisters,
    pub scratch: ScratchRegisters,
    pub error_code: usize,
    pub iret: InterruptStackFrameValue,
}

impl InterruptStack {
    /// Get the InterruptStack object on current kernel stack
    ///
    /// # Safety
    /// - No access are checked
    /// - Returned data is only valid during the current interrupt handler/syscall handler
    pub unsafe fn current() -> &'static mut Self {
        // InterruptStack is on top of kernel stack
        let stack_addr = Self::interrupt_stack_top() - (size_of::<InterruptStack>() as u64);
        let stack_ptr: *mut InterruptStack = stack_addr.as_mut_ptr();
        &mut *stack_ptr
    }

    /// Get the current interrupt kernel stack top
    pub fn interrupt_stack_top() -> VirtAddr {
        let top = KERNEL_STACK.stack_top();

        // Interrupt stacks must be 16 bytes aligned, or the processor will change rsp to align on enter
        assert!(top.is_aligned(16u16));

        top
    }
}

#[derive(Clone, Copy)]
#[repr(C)]
pub struct ScratchRegisters {
    pub r11: usize,
    pub r10: usize,
    pub r9: usize,
    pub r8: usize,
    pub rsi: usize,
    pub rdi: usize,
    pub rdx: usize,
    pub rcx: usize,
    pub rax: usize,
}

impl fmt::Debug for ScratchRegisters {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ScratchRegisters")
            .field("rax", &format_args!("{0:?} ({:#016x})", self.rax))
            .field("rcx", &format_args!("{0:?} ({:#016x})", self.rcx))
            .field("rdx", &format_args!("{0:?} ({:#016x})", self.rdx))
            .field("rdi", &format_args!("{0:?} ({:#016x})", self.rdi))
            .field("rsi", &format_args!("{0:?} ({:#016x})", self.rsi))
            .field("r8", &format_args!("{0:?} ({:#016x})", self.r8))
            .field("r9", &format_args!("{0:?} ({:#016x})", self.r9))
            .field("r10", &format_args!("{0:?} ({:#016x})", self.r10))
            .field("r11", &format_args!("{0:?} ({:#016x})", self.r11))
            .finish()
    }
}

#[macro_export]
macro_rules! push_scratch {
    () => {
        "
        // Push scratch registers
        push rax
        push rcx
        push rdx
        push rdi
        push rsi
        push r8
        push r9
        push r10
        push r11
    "
    };
}
#[macro_export]
macro_rules! pop_scratch {
    () => {
        "
        // Pop scratch registers
        pop r11
        pop r10
        pop r9
        pop r8
        pop rsi
        pop rdi
        pop rdx
        pop rcx
        pop rax
    "
    };
}

#[derive(Clone, Copy)]
#[repr(C)]
pub struct PreservedRegisters {
    pub r15: usize,
    pub r14: usize,
    pub r13: usize,
    pub r12: usize,
    pub rbp: usize,
    pub rbx: usize,
}

impl fmt::Debug for PreservedRegisters {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PreservedRegisters")
            .field("rbx", &format_args!("{0:?} ({:#016x})", self.rbx))
            .field("rbp", &format_args!("{0:?} ({:#016x})", self.rbp))
            .field("r12", &format_args!("{0:?} ({:#016x})", self.r12))
            .field("r13", &format_args!("{0:?} ({:#016x})", self.r13))
            .field("r14", &format_args!("{0:?} ({:#016x})", self.r14))
            .field("r15", &format_args!("{0:?} ({:#016x})", self.r15))
            .finish()
    }
}

#[macro_export]
macro_rules! push_preserved {
    () => {
        "
        // Push preserved registers
        push rbx
        push rbp
        push r12
        push r13
        push r14
        push r15
    "
    };
}
#[macro_export]
macro_rules! pop_preserved {
    () => {
        "
        // Pop preserved registers
        pop r15
        pop r14
        pop r13
        pop r12
        pop rbp
        pop rbx
    "
    };
}

/// Implement a native error handler, to handle an interrupt without an error code
#[macro_export]
macro_rules! native_handler {
    ($handler:expr) => {
        {
            #[naked]
            #[allow(undefined_naked_function_abi)]
            unsafe fn handler() {
                unsafe {
                    core::arch::naked_asm!(concat!(
                        "push 0;",                    // Fake error code

                        "cld;",                       // Clear direction flag, required by ABI when running any Rust code in the kernel.

                        push_scratch!(),
                        push_preserved!(),

                        // Call inner funtion
                        "call {interrupt_handler};",

                        pop_preserved!(),
                        pop_scratch!(),

                        "add rsp,8;",               // Error code
                        "iretq;",                   // Back to userland
                    ),

                    interrupt_handler = sym wrapper);
                }
            }

            unsafe extern "C" fn wrapper() {
                let _userland_timer = crate::user::thread::UserlandTimerInterruptScope::new();

                let stack = InterruptStack::current();

                $handler(stack);
            }

            VirtAddr::new(handler as u64)
        }
    }
}

/// Implement a native error handler, to handle an interrupt with an error code
#[macro_export]
macro_rules! native_error_handler {
    ($handler:expr) => {
        {
            #[naked]
            #[allow(undefined_naked_function_abi)]
            unsafe fn handler() {
                unsafe {
                    core::arch::naked_asm!(concat!(
                        "cld;",                       // Clear direction flag, required by ABI when running any Rust code in the kernel.

                        push_scratch!(),
                        push_preserved!(),

                        // Call inner funtion
                        "call {interrupt_handler};",

                        pop_preserved!(),
                        pop_scratch!(),

                        "add rsp,8;",               // Error code
                        "iretq;",                   // Back to userland
                    ),

                    interrupt_handler = sym wrapper);
                }
            }

            unsafe extern "C" fn wrapper() {
                let _userland_timer = crate::user::thread::UserlandTimerInterruptScope::new();

                let stack = InterruptStack::current();

                $handler(stack);
            }

            VirtAddr::new(handler as u64)
        }
    }
}

// Used in asm code at syscall handling to access data without registers
#[repr(align(4096))]
pub struct ProcessorControlRegion {
    pub userland_stack_ptr_tmp: usize,
    pub kernal_stack_ptr: usize,
}

impl ProcessorControlRegion {
    pub const fn new() -> Self {
        Self {
            userland_stack_ptr_tmp: 0,
            kernal_stack_ptr: 0,
        }
    }
}

/// This forces allocation in kernel binary in RW data section
struct StaticProcessorControlRegion(RefCell<ProcessorControlRegion>);

unsafe impl Sync for StaticProcessorControlRegion {}
unsafe impl Send for StaticProcessorControlRegion {}

impl StaticProcessorControlRegion {
    pub const fn new() -> Self {
        Self(RefCell::new(ProcessorControlRegion::new()))
    }
}

// Kernel stack used when entering kernel from userland (syscall, exception, irq)
// remove pub
static KERNEL_STACK: StaticKernelStack = StaticKernelStack::new();

// Structure will be setup so that it's easily addressable durign syscalls
static PROCESSOR_CONTROL_REGION: StaticProcessorControlRegion = StaticProcessorControlRegion::new();

// Setup KernelGsBase so that we can use ProcessorControlRegion using swapgs
pub fn init_process_control_region() {
    let processor_control_region = PROCESSOR_CONTROL_REGION.0.as_ptr();

    KernelGsBase::write(VirtAddr::from_ptr(processor_control_region));

    PROCESSOR_CONTROL_REGION.0.borrow_mut().kernal_stack_ptr =
        KERNEL_STACK.stack_top().as_u64() as usize;

    debug!(
        "Processor control region: {:?}",
        VirtAddr::from_ptr(processor_control_region)
    );
}
