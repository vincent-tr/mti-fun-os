use crate::interrupts::InterruptStack;
use crate::memory::StaticKernelStack;
use lazy_static::lazy_static;
use x86_64::structures::gdt::{Descriptor, GlobalDescriptorTable, SegmentSelector};
use x86_64::structures::tss::TaskStateSegment;

pub const FATAL_FAULT_IST_INDEX: u16 = 0;
pub const INTERRUPT_IST_INDEX: u16 = 1;

pub const KERNEL_CODE_SELECTOR_INDEX: u16 = 1;
pub const KERNEL_DATA_SELECTOR_INDEX: u16 = 2;
pub const USER_DATA_SELECTOR_INDEX: u16 = 3; // Note: to configure STAR syscall register properly
pub const USER_CODE_SELECTOR_INDEX: u16 = 4;

pub const KERNEL_CODE_SELECTOR: SegmentSelector =
    SegmentSelector::new(KERNEL_CODE_SELECTOR_INDEX, x86_64::PrivilegeLevel::Ring0);
pub const KERNEL_DATA_SELECTOR: SegmentSelector =
    SegmentSelector::new(KERNEL_DATA_SELECTOR_INDEX, x86_64::PrivilegeLevel::Ring0);
pub const USER_DATA_SELECTOR: SegmentSelector =
    SegmentSelector::new(USER_DATA_SELECTOR_INDEX, x86_64::PrivilegeLevel::Ring3);
pub const USER_CODE_SELECTOR: SegmentSelector =
    SegmentSelector::new(USER_CODE_SELECTOR_INDEX, x86_64::PrivilegeLevel::Ring3);

static FATAL_FAULT_STACK: StaticKernelStack = StaticKernelStack::new();

lazy_static! {
    static ref TSS: TaskStateSegment = {
        let mut tss = TaskStateSegment::new();

        tss.interrupt_stack_table[FATAL_FAULT_IST_INDEX as usize] = {
            // Special stack for double fault, so that we cannot stack overflow to print to correct info
            FATAL_FAULT_STACK.stack_top()
        };

        tss.interrupt_stack_table[INTERRUPT_IST_INDEX as usize] = {
            // Stack for normal interrupt handling
            InterruptStack::interrupt_stack_top()
        };

        tss
    };
}

lazy_static! {
    static ref GDT: (GlobalDescriptorTable, Selectors) = {
        let mut gdt = GlobalDescriptorTable::new();
        let selectors = Selectors {
            kernel_code_selector: gdt.append(Descriptor::kernel_code_segment()),
            kernel_data_selector: gdt.append(Descriptor::kernel_data_segment()),
            user_data_selector: gdt.append(Descriptor::user_data_segment()),
            user_code_selector: gdt.append(Descriptor::user_code_segment()),
            tss_selector: gdt.append(Descriptor::tss_segment(&TSS)),
        };

        assert!(selectors.kernel_code_selector.index() == KERNEL_CODE_SELECTOR_INDEX);
        assert!(selectors.kernel_data_selector.index() == KERNEL_DATA_SELECTOR_INDEX);
        assert!(selectors.user_code_selector.index() == USER_CODE_SELECTOR_INDEX);
        assert!(selectors.user_data_selector.index() == USER_DATA_SELECTOR_INDEX);

        (gdt, selectors)
    };
}

struct Selectors {
    kernel_code_selector: SegmentSelector,
    kernel_data_selector: SegmentSelector,
    user_code_selector: SegmentSelector,
    user_data_selector: SegmentSelector,
    tss_selector: SegmentSelector,
}

pub fn init() {
    use x86_64::instructions::segmentation::{Segment, CS};
    use x86_64::instructions::tables::load_tss;

    GDT.0.load();
    unsafe {
        CS::set_reg(GDT.1.kernel_code_selector);
        load_tss(GDT.1.tss_selector);
    }
}
