use x86_64::instructions::segmentation::{Segment, CS};
use x86_64::instructions::tables::load_tss;
use x86_64::structures::gdt::{Descriptor, GlobalDescriptorTable};
use x86_64::structures::tss::TaskStateSegment;
use x86_64::VirtAddr;

static mut GDT: GlobalDescriptorTable = GlobalDescriptorTable::new();
static mut TSS: TaskStateSegment = TaskStateSegment::new();

// TODO: move this
const PAGE_SIZE: usize = 4096;
const KERNEL_STACK_SIZE: usize = PAGE_SIZE * 5;

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
