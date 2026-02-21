use bit_field::BitField;
use spin::Mutex;
use x86_64::instructions::port::Port;

/// Command sent to begin PIC initialization.
const CMD_INIT: u8 = 0x11;

/// Command sent to acknowledge an interrupt.
const CMD_END_OF_INTERRUPT: u8 = 0x20;

// The mode in which we want to run our PICs.
const MODE_8086: u8 = 0x01;

const PIC_INTERRUPTS_SIZE: usize = 8;
const PIC_MASTER_OFFSET: usize = 32;
const PIC_SLAVE_OFFSET: usize = PIC_MASTER_OFFSET + PIC_INTERRUPTS_SIZE;

pub const IRQ0: usize = PIC_MASTER_OFFSET;

// from https://github.com/rust-osdev/pic8259

/// An individual PIC chip.  This is not exported, because we always access
/// it through `Pics` below.
struct Pic {
    /// The base offset to which our interrupts are mapped.
    offset: usize,

    /// The processor I/O port on which we send commands.
    command: Port<u8>,

    /// The processor I/O port on which we send and receive data.
    data: Port<u8>,
}

impl Pic {
    /// Are we in charge of handling the specified interrupt?
    /// (Each PIC handles 8 interrupts.)
    fn handles_interrupt(&self, interrupt_id: usize) -> bool {
        self.offset <= interrupt_id && interrupt_id < self.offset + PIC_INTERRUPTS_SIZE
    }

    /// Notify us that an interrupt has been handled and that we're ready
    /// for more.
    unsafe fn end_of_interrupt(&mut self) {
        unsafe { self.command.write(CMD_END_OF_INTERRUPT) };
    }

    /// Reads the interrupt mask of this PIC.
    unsafe fn read_mask(&mut self) -> u8 {
        unsafe { self.data.read() }
    }

    /// Writes the interrupt mask of this PIC.
    unsafe fn write_mask(&mut self, mask: u8) {
        unsafe { self.data.write(mask) }
    }

    unsafe fn set_irq_masked(&mut self, interrupt_id: usize, masked: bool) {
        unsafe {
            assert!(self.handles_interrupt(interrupt_id));
            let mut mask = self.read_mask();
            mask.set_bit(interrupt_id - self.offset, masked);
            self.write_mask(mask);
        }
    }

    unsafe fn is_irq_masked(&mut self, interrupt_id: usize) -> bool {
        unsafe {
            assert!(self.handles_interrupt(interrupt_id));
            let mask = self.read_mask();
            mask.get_bit(interrupt_id - self.offset)
        }
    }
}

/// A pair of chained PIC controllers.  This is the standard setup on x86.
struct ChainedPics {
    master: Pic,
    slave: Pic,
}

impl ChainedPics {
    /// Create a new interface for the standard PIC1 and PIC2 controllers,
    /// specifying the desired interrupt offsets.
    pub const fn new() -> Self {
        Self {
            master: Pic {
                offset: PIC_MASTER_OFFSET,
                command: Port::new(0x20),
                data: Port::new(0x21),
            },
            slave: Pic {
                offset: PIC_SLAVE_OFFSET,
                command: Port::new(0xA0),
                data: Port::new(0xA1),
            },
        }
    }

    /// Initialize both our PICs.  We initialize them together, at the same
    /// time, because it's traditional to do so, and because I/O operations
    /// might not be instantaneous on older processors.
    pub unsafe fn initialize(&mut self) {
        unsafe {
            // We need to add a delay between writes to our PICs, especially on
            // older motherboards.  But we don't necessarily have any kind of
            // timers yet, because most of them require interrupts.  Various
            // older versions of Linux and other PC operating systems have
            // worked around this by writing garbage data to port 0x80, which
            // allegedly takes long enough to make everything work on most
            // hardware.  Here, `wait` is a closure.
            let mut wait_port: Port<u8> = Port::new(0x80);
            let mut wait = || wait_port.write(0);

            // Save our original interrupt masks, because I'm too lazy to
            // figure out reasonable values.  We'll restore these when we're
            // done.
            let saved_masks = (self.master.read_mask(), self.slave.read_mask());

            // Tell each PIC that we're going to send it a three-byte
            // initialization sequence on its data port.
            self.master.command.write(CMD_INIT);
            wait();
            self.slave.command.write(CMD_INIT);
            wait();

            // Byte 1: Set up our base offsets.
            self.master.data.write(self.master.offset as u8);
            wait();
            self.slave.data.write(self.slave.offset as u8);
            wait();

            // Byte 2: Configure chaining between PIC1 and PIC2.
            self.master.data.write(4);
            wait();
            self.slave.data.write(2);
            wait();

            // Byte 3: Set our mode.
            self.master.data.write(MODE_8086);
            wait();
            self.slave.data.write(MODE_8086);
            wait();

            // Restore our saved masks.
            self.master.write_mask(saved_masks.0);
            self.slave.write_mask(saved_masks.1);
        }
    }

    pub unsafe fn set_irq_masked(&mut self, interrupt_id: usize, masked: bool) {
        unsafe {
            if self.master.handles_interrupt(interrupt_id) {
                self.master.set_irq_masked(interrupt_id, masked);
            } else if self.slave.handles_interrupt(interrupt_id) {
                self.slave.set_irq_masked(interrupt_id, masked);
            } else {
                panic!("invalid interrupt");
            }
        }
    }

    pub unsafe fn is_irq_masked(&mut self, interrupt_id: usize) -> bool {
        unsafe {
            if self.master.handles_interrupt(interrupt_id) {
                self.master.is_irq_masked(interrupt_id)
            } else if self.slave.handles_interrupt(interrupt_id) {
                self.slave.is_irq_masked(interrupt_id)
            } else {
                panic!("invalid interrupt");
            }
        }
    }

    /// Disables both PICs by masking all interrupts.
    pub unsafe fn disable(&mut self) {
        unsafe {
            self.master.write_mask(u8::MAX);
            self.slave.write_mask(u8::MAX);
        }
    }

    /// Do we handle this interrupt?
    pub fn handles_interrupt(&self, interrupt_id: usize) -> bool {
        self.master.handles_interrupt(interrupt_id) || self.slave.handles_interrupt(interrupt_id)
    }

    /// Figure out which (if any) PICs in our chain need to know about this
    /// interrupt.  This is tricky, because all interrupts from `slave`
    /// get chained through `master`.
    pub unsafe fn notify_end_of_interrupt(&mut self, interrupt_id: usize) {
        unsafe {
            if self.master.handles_interrupt(interrupt_id) {
                self.master.end_of_interrupt();
            } else if self.slave.handles_interrupt(interrupt_id) {
                self.slave.end_of_interrupt();
                self.master.end_of_interrupt();
            }
        }
    }
}

static PICS: Mutex<ChainedPics> = Mutex::new(ChainedPics::new());

pub fn init() {
    let mut pics = PICS.lock();
    unsafe { pics.initialize() };
}

pub fn set_irq_masked(interrupt_id: usize, masked: bool) {
    let mut pics = PICS.lock();
    unsafe {
        pics.set_irq_masked(interrupt_id, masked);
    }
}

pub fn is_irq_masked(interrupt_id: usize) -> bool {
    let mut pics = PICS.lock();
    unsafe { pics.is_irq_masked(interrupt_id) }
}

pub fn disable() {
    let mut pics = PICS.lock();
    unsafe { pics.disable() };
}

pub fn notify_end_of_interrupt(interrupt_id: usize) {
    let mut pics = PICS.lock();
    unsafe { pics.notify_end_of_interrupt(interrupt_id) };
}
