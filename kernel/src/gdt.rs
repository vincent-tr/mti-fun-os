use x86_64::{
    instructions::{
        segmentation::{Segment, CS},
        tables::load_tss,
    },
    structures::{
        gdt::{Descriptor, GlobalDescriptorTable},
        tss::TaskStateSegment,
    },
    VirtAddr,
};

use crate::memory::KERNEL_STACK_SIZE;
use crate::memory::KERNEL_STACK_SIZE;

// static GDT = crat

static mut GDT: GlobalDescriptorTable = GlobalDescriptorTable::new();
static mut TSS: TaskStateSegment = TaskStateSegment::new();

pub const DOUBLE_FAULT_IST_INDEX: u16 = 0;

pub fn init() {
    unsafe {
        TSS.interrupt_stack_table[DOUBLE_FAULT_IST_INDEX as usize] = {
            static mut STACK: [u8; KERNEL_STACK_SIZE] = [0; KERNEL_STACK_SIZE];

            let stack_start = VirtAddr::from_ptr(&STACK);
            let stack_end = stack_start + KERNEL_STACK_SIZE;
            stack_end
        };

        let code_selector = GDT.add_entry(Descriptor::kernel_code_segment());
        let tss_selector = GDT.add_entry(Descriptor::tss_segment(&TSS));
        GDT.load();

        CS::set_reg(code_selector);
        load_tss(tss_selector);
    }
}
