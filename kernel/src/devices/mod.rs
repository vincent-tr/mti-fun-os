// Not used for now
//mod local_apic;
mod cpu;
mod pic8259;
mod pit;

//pub use local_apic::*;
pub use cpu::CPUID;
pub use pic8259::{notify_end_of_interrupt, IRQ0};

pub fn init() {
    pic8259::init();
    pit::init();
}
