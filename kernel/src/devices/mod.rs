pub mod cpu;
pub mod local_apic;
pub mod pic8259;
pub mod pit;

pub fn init() {
    pic8259::init();
    pic8259::disable();

    local_apic::init();
    local_apic::configure_timer();
}
