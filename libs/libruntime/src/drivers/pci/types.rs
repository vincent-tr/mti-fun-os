use core::fmt;

/// PCI device address
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(C)]
pub struct PciAddress {
    /// The bus number of the PCI address (0-255).
    pub bus: u8,

    /// The device number of the PCI address (0-31).
    pub device: u8,

    /// The function number of the PCI address (0-7).
    pub function: u8,
}

impl fmt::Display for PciAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:02x}:{:02x}.{:01x}",
            self.bus, self.device, self.function
        )
    }
}

/// PCI device ID (vendor ID and device ID)
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[repr(C)]
pub struct PciDeviceId {
    /// The vendor ID of the PCI device (16 bits).
    pub vendor: u16,

    /// The device ID of the PCI device (16 bits).
    pub device: u16,
}

impl fmt::Display for PciDeviceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:04x}:{:04x}", self.vendor, self.device)
    }
}

/// PCI class information
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[repr(C)]
pub struct PciClass {
    /// The class code of the PCI device (8 bits).
    pub class: u8,

    /// The subclass code of the PCI device (8 bits).
    pub subclass: u8,

    /// The programming interface code of the PCI device (8 bits).
    pub prog_if: u8,

    /// The revision ID of the PCI device (8 bits).
    pub revision_id: u8,
}

impl PciClass {
    /// Returns the class kind of the PCI device, which provides a more human-readable classification of the device type.
    pub fn kind(&self) -> PciClassKind {
        PciClassKind::from(*self)
    }
}

impl fmt::Display for PciClass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:02x}:{:02x}", self.class, self.subclass)
    }
}

/// PCI class kind (e.g., mass storage, network controller, etc.)
///
/// Note: source here: https://wiki.osdev.org/PCI#Class_Codes
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum PciClassKind {
    Unclassified(UnclassifiedSubclass),
    MassStorage(MassStorageSubclass),
    Network(NetworkSubclass),
    Display(DisplaySubclass),
    Multimedia(MultimediaSubclass),
    Memory(MemorySubclass),
    Bridge(BridgeSubclass),
    SimpleCommunication(SimpleCommunicationSubclass),
    BaseSystemPeripheral(BaseSystemPeripheralSubclass),
    InputDevice(InputDeviceSubclass),
    DockingStation(DockingStationSubclass),
    Processor(ProcessorSubclass),
    SerialBus(SerialBusSubclass),
    Wireless(WirelessSubclass),
    Intelligent(IntelligentSubclass),
    SatelliteCommunication(SatelliteCommunicationSubclass),
    Encryption(EncryptionSubclass),
    SignalProcessing(SignalProcessingSubclass),
    ProcessingAccelerator(u8, u8),
    NonEssentialInstrumentation(u8, u8),
    CoProcessor(u8, u8),
    Unknown(u8, u8, u8),
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum UnclassifiedSubclass {
    NotVgaCompatible(u8),
    VgaCompatible(u8),
    Unknown(u8, u8),
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum MassStorageSubclass {
    ScsiBus(u8),
    Ide(IdeProgIf),
    FloppyDisk(u8),
    IpiBus(u8),
    Raid(u8),
    Ata(AtaProgIf),
    Sata(SataProgIf),
    Sas(SasProgIf),
    Nvm(NvmProgIf),
    Unknown(u8, u8),
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum IdeProgIf {
    IsaCompatibilityOnly,
    PciNativeOnly,
    IsaCompatibilitySwitchable,
    PciNativeSwitchable,
    IsaCompatibilityOnlyBusMastering,
    PciNativeOnlyBusMastering,
    IsaCompatibilitySwitchableBusMastering,
    PciNativeSwitchableBusMastering,
    Unknown(u8),
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum AtaProgIf {
    SingleDma,
    ChainedDma,
    Unknown(u8),
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum SataProgIf {
    VendorSpecific,
    Ahci,
    SerialStorageBus,
    Unknown(u8),
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum SasProgIf {
    Sas,
    SerialStorageBus,
    Unknown(u8),
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum NvmProgIf {
    NvmHci,
    NvmExpress,
    Unknown(u8),
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum NetworkSubclass {
    Ethernet(u8),
    TokenRing(u8),
    Fddi(u8),
    Atm(u8),
    Isdn(u8),
    WorldFip(u8),
    PicmgMultiComputing(u8),
    Infiniband(u8),
    Fabric(u8),
    Unknown(u8, u8),
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum DisplaySubclass {
    Vga(VgaProgIf),
    Xga(u8),
    Controller3D(u8),
    Unknown(u8, u8),
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum VgaProgIf {
    VgaController,
    Compatible8514,
    Unknown(u8),
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum MultimediaSubclass {
    Video(u8),
    Audio(u8),
    ComputerTelephony(u8),
    AudioDevice(u8),
    Unknown(u8, u8),
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum MemorySubclass {
    Ram(u8),
    Flash(u8),
    Unknown(u8, u8),
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum BridgeSubclass {
    Host(u8),
    Isa(u8),
    Eisa(u8),
    Mca(u8),
    PciToPci(PciToPciProgIf),
    Pcmcia(u8),
    NuBus(u8),
    CardBus(u8),
    RaceWay(RaceWayProgIf),
    PciToPciSemiTransparent(SemiTransparentProgIf),
    InfiniBandToPci(u8),
    Unknown(u8, u8),
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum PciToPciProgIf {
    NormalDecode,
    SubtractiveDecode,
    Unknown(u8),
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum RaceWayProgIf {
    Transparent,
    Endpoint,
    Unknown(u8),
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum SemiTransparentProgIf {
    PrimaryTowardsCpu,
    SecondaryTowardsCpu,
    Unknown(u8),
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum SimpleCommunicationSubclass {
    Serial(SerialProgIf),
    Parallel(ParallelProgIf),
    MultiportSerial(u8),
    Modem(ModemProgIf),
    Gpib(u8),
    SmartCard(u8),
    Unknown(u8, u8),
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum SerialProgIf {
    Compatible8250,
    Compatible16450,
    Compatible16550,
    Compatible16650,
    Compatible16750,
    Compatible16850,
    Compatible16950,
    Unknown(u8),
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum ParallelProgIf {
    Standard,
    BiDirectional,
    Ecp,
    Ieee1284Controller,
    Ieee1284Target,
    Unknown(u8),
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum ModemProgIf {
    Generic,
    Hayes16450,
    Hayes16550,
    Hayes16650,
    Hayes16750,
    Unknown(u8),
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum BaseSystemPeripheralSubclass {
    Pic(PicProgIf),
    Dma(DmaProgIf),
    Timer(TimerProgIf),
    Rtc(RtcProgIf),
    PciHotPlug(u8),
    SdHost(u8),
    Iommu(u8),
    Unknown(u8, u8),
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum PicProgIf {
    Generic8259,
    IsaCompatible,
    EisaCompatible,
    IoApic,
    IoXApic,
    Unknown(u8),
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum DmaProgIf {
    Generic8237,
    IsaCompatible,
    EisaCompatible,
    Unknown(u8),
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum TimerProgIf {
    Generic8254,
    IsaCompatible,
    EisaCompatible,
    Hpet,
    Unknown(u8),
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum RtcProgIf {
    Generic,
    IsaCompatible,
    Unknown(u8),
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum InputDeviceSubclass {
    Keyboard(u8),
    Digitizer(u8),
    Mouse(u8),
    Scanner(u8),
    Gameport(GameportProgIf),
    Unknown(u8, u8),
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum GameportProgIf {
    Generic,
    Extended,
    Unknown(u8),
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum DockingStationSubclass {
    Generic,
    Unknown(u8),
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum ProcessorSubclass {
    I386(u8),
    I486(u8),
    Pentium(u8),
    PentiumPro(u8),
    Alpha(u8),
    PowerPc(u8),
    Mips(u8),
    CoProcessor(u8),
    Unknown(u8, u8),
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum SerialBusSubclass {
    FireWire(FireWireProgIf),
    AccessBus(u8),
    Ssa(u8),
    Usb(UsbProgIf),
    FibreChannel(u8),
    SmBus(u8),
    InfiniBand(u8),
    Ipmi(IpmiProgIf),
    Sercos(u8),
    CanBus(u8),
    Unknown(u8, u8),
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum FireWireProgIf {
    Generic,
    Ohci,
    Unknown(u8),
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum UsbProgIf {
    Uhci,
    Ohci,
    Ehci,
    Xhci,
    Unspecified,
    UsbDevice,
    Unknown(u8),
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum IpmiProgIf {
    Smic,
    KeyboardControllerStyle,
    BlockTransfer,
    Unknown(u8),
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum WirelessSubclass {
    Irda(u8),
    ConsumerIr(u8),
    Rf(u8),
    Bluetooth(u8),
    Broadband(u8),
    Ethernet802_1a(u8),
    Ethernet802_1b(u8),
    Unknown(u8, u8),
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum IntelligentSubclass {
    I20(u8),
    Unknown(u8, u8),
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum SatelliteCommunicationSubclass {
    Tv(u8),
    Audio(u8),
    Voice(u8),
    Data(u8),
    Unknown(u8, u8),
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum EncryptionSubclass {
    NetworkAndComputing(u8),
    Entertainment(u8),
    Unknown(u8, u8),
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum SignalProcessingSubclass {
    DpioModules(u8),
    PerformanceCounters(u8),
    CommunicationSynchronizer(u8),
    SignalProcessingManagement(u8),
    Unknown(u8, u8),
}

// ============================================================================
// Display implementations
// ============================================================================

/// Helper struct to count bytes written in Display implementations.
struct ByteCounter {
    count: usize,
}

impl fmt::Write for ByteCounter {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.count += s.len();
        Ok(())
    }
}

/// Extension trait for fmt::Formatter to format a class with its subclass.
trait FormatterExt {
    /// Format a parent class with its subclass.
    /// If the subclass displays as empty string (Unknown), only show the parent class name.
    fn format_with_subtype(&mut self, parent: &str, subtype: impl fmt::Display) -> fmt::Result;
}

impl FormatterExt for fmt::Formatter<'_> {
    fn format_with_subtype(&mut self, parent: &str, subtype: impl fmt::Display) -> fmt::Result {
        use core::fmt::Write;

        let mut counter = ByteCounter { count: 0 };
        write!(counter, "{}", subtype)?;

        if counter.count == 0 {
            write!(self, "{}", parent)
        } else {
            write!(self, "{} - {}", parent, subtype)
        }
    }
}

impl fmt::Display for PciClassKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unclassified(sub) => f.format_with_subtype("Unclassified", sub),
            Self::MassStorage(sub) => f.format_with_subtype("Mass Storage", sub),
            Self::Network(sub) => f.format_with_subtype("Network", sub),
            Self::Display(sub) => f.format_with_subtype("Display", sub),
            Self::Multimedia(sub) => f.format_with_subtype("Multimedia", sub),
            Self::Memory(sub) => f.format_with_subtype("Memory", sub),
            Self::Bridge(sub) => f.format_with_subtype("Bridge", sub),
            Self::SimpleCommunication(sub) => f.format_with_subtype("Simple Communication", sub),
            Self::BaseSystemPeripheral(sub) => f.format_with_subtype("Base System Peripheral", sub),
            Self::InputDevice(sub) => f.format_with_subtype("Input Device", sub),
            Self::DockingStation(sub) => f.format_with_subtype("Docking Station", sub),
            Self::Processor(sub) => f.format_with_subtype("Processor", sub),
            Self::SerialBus(sub) => f.format_with_subtype("Serial Bus", sub),
            Self::Wireless(sub) => f.format_with_subtype("Wireless", sub),
            Self::Intelligent(sub) => f.format_with_subtype("Intelligent", sub),
            Self::SatelliteCommunication(sub) => {
                f.format_with_subtype("Satellite Communication", sub)
            }
            Self::Encryption(sub) => f.format_with_subtype("Encryption", sub),
            Self::SignalProcessing(sub) => f.format_with_subtype("Signal Processing", sub),
            Self::ProcessingAccelerator(..) => f.write_str("Processing Accelerator"),
            Self::NonEssentialInstrumentation(..) => f.write_str("Non-Essential Instrumentation"),
            Self::CoProcessor(..) => f.write_str("Co-Processor"),
            Self::Unknown(..) => f.write_str("Unknown"),
        }
    }
}

impl fmt::Display for UnclassifiedSubclass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotVgaCompatible(_) => f.write_str("Non-VGA-Compatible"),
            Self::VgaCompatible(_) => f.write_str("VGA-Compatible"),
            Self::Unknown(..) => Ok(()),
        }
    }
}

impl fmt::Display for MassStorageSubclass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ScsiBus(_) => f.write_str("SCSI Bus Controller"),
            Self::Ide(prog) => f.format_with_subtype("IDE Controller", prog),
            Self::FloppyDisk(_) => f.write_str("Floppy Disk Controller"),
            Self::IpiBus(_) => f.write_str("IPI Bus Controller"),
            Self::Raid(_) => f.write_str("RAID Controller"),
            Self::Ata(prog) => f.format_with_subtype("ATA Controller", prog),
            Self::Sata(prog) => f.format_with_subtype("Serial ATA", prog),
            Self::Sas(prog) => f.format_with_subtype("Serial Attached SCSI", prog),
            Self::Nvm(prog) => f.format_with_subtype("Non-Volatile Memory", prog),
            Self::Unknown(..) => Ok(()),
        }
    }
}

impl fmt::Display for IdeProgIf {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::IsaCompatibilityOnly => f.write_str("ISA Compatibility mode-only"),
            Self::PciNativeOnly => f.write_str("PCI native mode-only"),
            Self::IsaCompatibilitySwitchable => {
                f.write_str("ISA Compatibility, switchable to PCI native")
            }
            Self::PciNativeSwitchable => f.write_str("PCI native, switchable to ISA compatibility"),
            Self::IsaCompatibilityOnlyBusMastering => {
                f.write_str("ISA Compatibility mode-only, bus mastering")
            }
            Self::PciNativeOnlyBusMastering => f.write_str("PCI native mode-only, bus mastering"),
            Self::IsaCompatibilitySwitchableBusMastering => {
                f.write_str("ISA Compatibility, switchable to PCI native, bus mastering")
            }
            Self::PciNativeSwitchableBusMastering => {
                f.write_str("PCI native, switchable to ISA compatibility, bus mastering")
            }
            Self::Unknown(..) => Ok(()),
        }
    }
}

impl fmt::Display for AtaProgIf {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SingleDma => f.write_str("Single DMA"),
            Self::ChainedDma => f.write_str("Chained DMA"),
            Self::Unknown(..) => Ok(()),
        }
    }
}

impl fmt::Display for SataProgIf {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::VendorSpecific => f.write_str("Vendor Specific Interface"),
            Self::Ahci => f.write_str("AHCI 1.0"),
            Self::SerialStorageBus => f.write_str("Serial Storage Bus"),
            Self::Unknown(..) => Ok(()),
        }
    }
}

impl fmt::Display for SasProgIf {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Sas => f.write_str("SAS"),
            Self::SerialStorageBus => f.write_str("Serial Storage Bus"),
            Self::Unknown(..) => Ok(()),
        }
    }
}

impl fmt::Display for NvmProgIf {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NvmHci => f.write_str("NVMHCI"),
            Self::NvmExpress => f.write_str("NVM Express"),
            Self::Unknown(..) => Ok(()),
        }
    }
}

impl fmt::Display for NetworkSubclass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Ethernet(_) => f.write_str("Ethernet Controller"),
            Self::TokenRing(_) => f.write_str("Token Ring Controller"),
            Self::Fddi(_) => f.write_str("FDDI Controller"),
            Self::Atm(_) => f.write_str("ATM Controller"),
            Self::Isdn(_) => f.write_str("ISDN Controller"),
            Self::WorldFip(_) => f.write_str("WorldFip Controller"),
            Self::PicmgMultiComputing(_) => f.write_str("PICMG 2.14 Multi Computing"),
            Self::Infiniband(_) => f.write_str("Infiniband Controller"),
            Self::Fabric(_) => f.write_str("Fabric Controller"),
            Self::Unknown(..) => Ok(()),
        }
    }
}

impl fmt::Display for DisplaySubclass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Vga(prog) => f.format_with_subtype("VGA Compatible Controller", prog),
            Self::Xga(_) => f.write_str("XGA Controller"),
            Self::Controller3D(_) => f.write_str("3D Controller (Not VGA-Compatible)"),
            Self::Unknown(..) => Ok(()),
        }
    }
}

impl fmt::Display for VgaProgIf {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::VgaController => f.write_str("VGA Controller"),
            Self::Compatible8514 => f.write_str("8514-Compatible"),
            Self::Unknown(..) => Ok(()),
        }
    }
}

impl fmt::Display for MultimediaSubclass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Video(_) => f.write_str("Multimedia Video Controller"),
            Self::Audio(_) => f.write_str("Multimedia Audio Controller"),
            Self::ComputerTelephony(_) => f.write_str("Computer Telephony Device"),
            Self::AudioDevice(_) => f.write_str("Audio Device"),
            Self::Unknown(..) => Ok(()),
        }
    }
}

impl fmt::Display for MemorySubclass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Ram(_) => f.write_str("RAM Controller"),
            Self::Flash(_) => f.write_str("Flash Controller"),
            Self::Unknown(..) => Ok(()),
        }
    }
}

impl fmt::Display for BridgeSubclass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Host(_) => f.write_str("Host Bridge"),
            Self::Isa(_) => f.write_str("ISA Bridge"),
            Self::Eisa(_) => f.write_str("EISA Bridge"),
            Self::Mca(_) => f.write_str("MCA Bridge"),
            Self::PciToPci(prog) => f.format_with_subtype("PCI-to-PCI Bridge", prog),
            Self::Pcmcia(_) => f.write_str("PCMCIA Bridge"),
            Self::NuBus(_) => f.write_str("NuBus Bridge"),
            Self::CardBus(_) => f.write_str("CardBus Bridge"),
            Self::RaceWay(prog) => f.format_with_subtype("RACEway Bridge", prog),
            Self::PciToPciSemiTransparent(prog) => {
                f.format_with_subtype("PCI-to-PCI Semi-Transparent Bridge", prog)
            }
            Self::InfiniBandToPci(_) => f.write_str("InfiniBand-to-PCI Host Bridge"),
            Self::Unknown(..) => Ok(()),
        }
    }
}

impl fmt::Display for PciToPciProgIf {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NormalDecode => f.write_str("Normal Decode"),
            Self::SubtractiveDecode => f.write_str("Subtractive Decode"),
            Self::Unknown(..) => Ok(()),
        }
    }
}

impl fmt::Display for RaceWayProgIf {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Transparent => f.write_str("Transparent Mode"),
            Self::Endpoint => f.write_str("Endpoint Mode"),
            Self::Unknown(..) => Ok(()),
        }
    }
}

impl fmt::Display for SemiTransparentProgIf {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PrimaryTowardsCpu => {
                f.write_str("Semi-Transparent, Primary bus towards host CPU")
            }
            Self::SecondaryTowardsCpu => {
                f.write_str("Semi-Transparent, Secondary bus towards host CPU")
            }
            Self::Unknown(..) => Ok(()),
        }
    }
}

impl fmt::Display for SimpleCommunicationSubclass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Serial(prog) => f.format_with_subtype("Serial Controller", prog),
            Self::Parallel(prog) => f.format_with_subtype("Parallel Controller", prog),
            Self::MultiportSerial(_) => f.write_str("Multiport Serial Controller"),
            Self::Modem(prog) => f.format_with_subtype("Modem", prog),
            Self::Gpib(_) => f.write_str("IEEE 488.1/2 (GPIB) Controller"),
            Self::SmartCard(_) => f.write_str("Smart Card Controller"),
            Self::Unknown(..) => Ok(()),
        }
    }
}

impl fmt::Display for SerialProgIf {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Compatible8250 => f.write_str("8250-Compatible (Generic XT)"),
            Self::Compatible16450 => f.write_str("16450-Compatible"),
            Self::Compatible16550 => f.write_str("16550-Compatible"),
            Self::Compatible16650 => f.write_str("16650-Compatible"),
            Self::Compatible16750 => f.write_str("16750-Compatible"),
            Self::Compatible16850 => f.write_str("16850-Compatible"),
            Self::Compatible16950 => f.write_str("16950-Compatible"),
            Self::Unknown(..) => Ok(()),
        }
    }
}

impl fmt::Display for ParallelProgIf {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Standard => f.write_str("Standard Parallel Port"),
            Self::BiDirectional => f.write_str("Bi-Directional Parallel Port"),
            Self::Ecp => f.write_str("ECP 1.X Compliant Parallel Port"),
            Self::Ieee1284Controller => f.write_str("IEEE 1284 Controller"),
            Self::Ieee1284Target => f.write_str("IEEE 1284 Target Device"),
            Self::Unknown(..) => Ok(()),
        }
    }
}

impl fmt::Display for ModemProgIf {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Generic => f.write_str("Generic Modem"),
            Self::Hayes16450 => f.write_str("Hayes 16450-Compatible"),
            Self::Hayes16550 => f.write_str("Hayes 16550-Compatible"),
            Self::Hayes16650 => f.write_str("Hayes 16650-Compatible"),
            Self::Hayes16750 => f.write_str("Hayes 16750-Compatible"),
            Self::Unknown(..) => Ok(()),
        }
    }
}

impl fmt::Display for BaseSystemPeripheralSubclass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pic(prog) => f.format_with_subtype("PIC", prog),
            Self::Dma(prog) => f.format_with_subtype("DMA Controller", prog),
            Self::Timer(prog) => f.format_with_subtype("Timer", prog),
            Self::Rtc(prog) => f.format_with_subtype("RTC Controller", prog),
            Self::PciHotPlug(_) => f.write_str("PCI Hot-Plug Controller"),
            Self::SdHost(_) => f.write_str("SD Host controller"),
            Self::Iommu(_) => f.write_str("IOMMU"),
            Self::Unknown(..) => Ok(()),
        }
    }
}

impl fmt::Display for PicProgIf {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Generic8259 => f.write_str("Generic 8259-Compatible"),
            Self::IsaCompatible => f.write_str("ISA-Compatible"),
            Self::EisaCompatible => f.write_str("EISA-Compatible"),
            Self::IoApic => f.write_str("I/O APIC"),
            Self::IoXApic => f.write_str("I/O(x) APIC"),
            Self::Unknown(..) => Ok(()),
        }
    }
}

impl fmt::Display for DmaProgIf {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Generic8237 => f.write_str("Generic 8237-Compatible"),
            Self::IsaCompatible => f.write_str("ISA-Compatible"),
            Self::EisaCompatible => f.write_str("EISA-Compatible"),
            Self::Unknown(..) => Ok(()),
        }
    }
}

impl fmt::Display for TimerProgIf {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Generic8254 => f.write_str("Generic 8254-Compatible"),
            Self::IsaCompatible => f.write_str("ISA-Compatible"),
            Self::EisaCompatible => f.write_str("EISA-Compatible"),
            Self::Hpet => f.write_str("HPET"),
            Self::Unknown(..) => Ok(()),
        }
    }
}

impl fmt::Display for RtcProgIf {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Generic => f.write_str("Generic RTC"),
            Self::IsaCompatible => f.write_str("ISA-Compatible"),
            Self::Unknown(..) => Ok(()),
        }
    }
}

impl fmt::Display for InputDeviceSubclass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Keyboard(_) => f.write_str("Keyboard Controller"),
            Self::Digitizer(_) => f.write_str("Digitizer Pen"),
            Self::Mouse(_) => f.write_str("Mouse Controller"),
            Self::Scanner(_) => f.write_str("Scanner Controller"),
            Self::Gameport(prog) => f.format_with_subtype("Gameport Controller", prog),
            Self::Unknown(..) => Ok(()),
        }
    }
}

impl fmt::Display for GameportProgIf {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Generic => f.write_str("Generic"),
            Self::Extended => f.write_str("Extended"),
            Self::Unknown(..) => Ok(()),
        }
    }
}

impl fmt::Display for DockingStationSubclass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Generic => f.write_str("Generic"),
            Self::Unknown(..) => Ok(()),
        }
    }
}

impl fmt::Display for ProcessorSubclass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::I386(_) => f.write_str("386"),
            Self::I486(_) => f.write_str("486"),
            Self::Pentium(_) => f.write_str("Pentium"),
            Self::PentiumPro(_) => f.write_str("Pentium Pro"),
            Self::Alpha(_) => f.write_str("Alpha"),
            Self::PowerPc(_) => f.write_str("PowerPC"),
            Self::Mips(_) => f.write_str("MIPS"),
            Self::CoProcessor(_) => f.write_str("Co-Processor"),
            Self::Unknown(..) => Ok(()),
        }
    }
}

impl fmt::Display for SerialBusSubclass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FireWire(prog) => f.format_with_subtype("FireWire (IEEE 1394)", prog),
            Self::AccessBus(_) => f.write_str("ACCESS Bus"),
            Self::Ssa(_) => f.write_str("SSA"),
            Self::Usb(prog) => f.format_with_subtype("USB", prog),
            Self::FibreChannel(_) => f.write_str("Fibre Channel"),
            Self::SmBus(_) => f.write_str("SMBus"),
            Self::InfiniBand(_) => f.write_str("InfiniBand"),
            Self::Ipmi(prog) => f.format_with_subtype("IPMI", prog),
            Self::Sercos(_) => f.write_str("SERCOS (IEC 61491)"),
            Self::CanBus(_) => f.write_str("CANbus"),
            Self::Unknown(..) => Ok(()),
        }
    }
}

impl fmt::Display for FireWireProgIf {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Generic => f.write_str("Generic"),
            Self::Ohci => f.write_str("OHCI"),
            Self::Unknown(..) => Ok(()),
        }
    }
}

impl fmt::Display for UsbProgIf {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Uhci => f.write_str("UHCI"),
            Self::Ohci => f.write_str("OHCI"),
            Self::Ehci => f.write_str("EHCI (USB2)"),
            Self::Xhci => f.write_str("XHCI (USB3)"),
            Self::Unspecified => f.write_str("Unspecified"),
            Self::UsbDevice => f.write_str("USB Device (Not a host controller)"),
            Self::Unknown(..) => Ok(()),
        }
    }
}

impl fmt::Display for IpmiProgIf {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Smic => f.write_str("SMIC"),
            Self::KeyboardControllerStyle => f.write_str("Keyboard Controller Style"),
            Self::BlockTransfer => f.write_str("Block Transfer"),
            Self::Unknown(..) => Ok(()),
        }
    }
}

impl fmt::Display for WirelessSubclass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Irda(_) => f.write_str("iRDA Compatible Controller"),
            Self::ConsumerIr(_) => f.write_str("Consumer IR Controller"),
            Self::Rf(_) => f.write_str("RF Controller"),
            Self::Bluetooth(_) => f.write_str("Bluetooth Controller"),
            Self::Broadband(_) => f.write_str("Broadband Controller"),
            Self::Ethernet802_1a(_) => f.write_str("Ethernet Controller (802.1a)"),
            Self::Ethernet802_1b(_) => f.write_str("Ethernet Controller (802.1b)"),
            Self::Unknown(..) => Ok(()),
        }
    }
}

impl fmt::Display for IntelligentSubclass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::I20(_) => f.write_str("I20"),
            Self::Unknown(..) => Ok(()),
        }
    }
}

impl fmt::Display for SatelliteCommunicationSubclass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Tv(_) => f.write_str("Satellite TV Controller"),
            Self::Audio(_) => f.write_str("Satellite Audio Controller"),
            Self::Voice(_) => f.write_str("Satellite Voice Controller"),
            Self::Data(_) => f.write_str("Satellite Data Controller"),
            Self::Unknown(..) => Ok(()),
        }
    }
}

impl fmt::Display for EncryptionSubclass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NetworkAndComputing(_) => {
                f.write_str("Network and Computing Encryption/Decryption")
            }
            Self::Entertainment(_) => f.write_str("Entertainment Encryption/Decryption"),
            Self::Unknown(..) => Ok(()),
        }
    }
}

impl fmt::Display for SignalProcessingSubclass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DpioModules(_) => f.write_str("DPIO Modules"),
            Self::PerformanceCounters(_) => f.write_str("Performance Counters"),
            Self::CommunicationSynchronizer(_) => f.write_str("Communication Synchronizer"),
            Self::SignalProcessingManagement(_) => f.write_str("Signal Processing Management"),
            Self::Unknown(..) => Ok(()),
        }
    }
}

// ============================================================================
// Parsing implementations
// ============================================================================

impl From<PciClass> for PciClassKind {
    fn from(class: PciClass) -> Self {
        match class.class {
            0x00 => Self::Unclassified(UnclassifiedSubclass::parse(class.subclass, class.prog_if)),
            0x01 => Self::MassStorage(MassStorageSubclass::parse(class.subclass, class.prog_if)),
            0x02 => Self::Network(NetworkSubclass::parse(class.subclass, class.prog_if)),
            0x03 => Self::Display(DisplaySubclass::parse(class.subclass, class.prog_if)),
            0x04 => Self::Multimedia(MultimediaSubclass::parse(class.subclass, class.prog_if)),
            0x05 => Self::Memory(MemorySubclass::parse(class.subclass, class.prog_if)),
            0x06 => Self::Bridge(BridgeSubclass::parse(class.subclass, class.prog_if)),
            0x07 => Self::SimpleCommunication(SimpleCommunicationSubclass::parse(
                class.subclass,
                class.prog_if,
            )),
            0x08 => Self::BaseSystemPeripheral(BaseSystemPeripheralSubclass::parse(
                class.subclass,
                class.prog_if,
            )),
            0x09 => Self::InputDevice(InputDeviceSubclass::parse(class.subclass, class.prog_if)),
            0x0A => {
                Self::DockingStation(DockingStationSubclass::parse(class.subclass, class.prog_if))
            }
            0x0B => Self::Processor(ProcessorSubclass::parse(class.subclass, class.prog_if)),
            0x0C => Self::SerialBus(SerialBusSubclass::parse(class.subclass, class.prog_if)),
            0x0D => Self::Wireless(WirelessSubclass::parse(class.subclass, class.prog_if)),
            0x0E => Self::Intelligent(IntelligentSubclass::parse(class.subclass, class.prog_if)),
            0x0F => Self::SatelliteCommunication(SatelliteCommunicationSubclass::parse(
                class.subclass,
                class.prog_if,
            )),
            0x10 => Self::Encryption(EncryptionSubclass::parse(class.subclass, class.prog_if)),
            0x11 => Self::SignalProcessing(SignalProcessingSubclass::parse(
                class.subclass,
                class.prog_if,
            )),
            0x12 => Self::ProcessingAccelerator(class.subclass, class.prog_if),
            0x13 => Self::NonEssentialInstrumentation(class.subclass, class.prog_if),
            0x40 => Self::CoProcessor(class.subclass, class.prog_if),
            _ => Self::Unknown(class.class, class.subclass, class.prog_if),
        }
    }
}

impl UnclassifiedSubclass {
    fn parse(subclass: u8, prog_if: u8) -> Self {
        match subclass {
            0x00 => Self::NotVgaCompatible(prog_if),
            0x01 => Self::VgaCompatible(prog_if),
            _ => Self::Unknown(subclass, prog_if),
        }
    }
}

impl MassStorageSubclass {
    fn parse(subclass: u8, prog_if: u8) -> Self {
        match subclass {
            0x00 => Self::ScsiBus(prog_if),
            0x01 => Self::Ide(IdeProgIf::parse(prog_if)),
            0x02 => Self::FloppyDisk(prog_if),
            0x03 => Self::IpiBus(prog_if),
            0x04 => Self::Raid(prog_if),
            0x05 => Self::Ata(AtaProgIf::parse(prog_if)),
            0x06 => Self::Sata(SataProgIf::parse(prog_if)),
            0x07 => Self::Sas(SasProgIf::parse(prog_if)),
            0x08 => Self::Nvm(NvmProgIf::parse(prog_if)),
            0x80 => Self::Unknown(subclass, prog_if),
            _ => Self::Unknown(subclass, prog_if),
        }
    }
}

impl IdeProgIf {
    fn parse(prog_if: u8) -> Self {
        match prog_if {
            0x00 => Self::IsaCompatibilityOnly,
            0x05 => Self::PciNativeOnly,
            0x0A => Self::IsaCompatibilitySwitchable,
            0x0F => Self::PciNativeSwitchable,
            0x80 => Self::IsaCompatibilityOnlyBusMastering,
            0x85 => Self::PciNativeOnlyBusMastering,
            0x8A => Self::IsaCompatibilitySwitchableBusMastering,
            0x8F => Self::PciNativeSwitchableBusMastering,
            _ => Self::Unknown(prog_if),
        }
    }
}

impl AtaProgIf {
    fn parse(prog_if: u8) -> Self {
        match prog_if {
            0x20 => Self::SingleDma,
            0x30 => Self::ChainedDma,
            _ => Self::Unknown(prog_if),
        }
    }
}

impl SataProgIf {
    fn parse(prog_if: u8) -> Self {
        match prog_if {
            0x00 => Self::VendorSpecific,
            0x01 => Self::Ahci,
            0x02 => Self::SerialStorageBus,
            _ => Self::Unknown(prog_if),
        }
    }
}

impl SasProgIf {
    fn parse(prog_if: u8) -> Self {
        match prog_if {
            0x00 => Self::Sas,
            0x01 => Self::SerialStorageBus,
            _ => Self::Unknown(prog_if),
        }
    }
}

impl NvmProgIf {
    fn parse(prog_if: u8) -> Self {
        match prog_if {
            0x01 => Self::NvmHci,
            0x02 => Self::NvmExpress,
            _ => Self::Unknown(prog_if),
        }
    }
}

impl NetworkSubclass {
    fn parse(subclass: u8, prog_if: u8) -> Self {
        match subclass {
            0x00 => Self::Ethernet(prog_if),
            0x01 => Self::TokenRing(prog_if),
            0x02 => Self::Fddi(prog_if),
            0x03 => Self::Atm(prog_if),
            0x04 => Self::Isdn(prog_if),
            0x05 => Self::WorldFip(prog_if),
            0x06 => Self::PicmgMultiComputing(prog_if),
            0x07 => Self::Infiniband(prog_if),
            0x08 => Self::Fabric(prog_if),
            0x80 => Self::Unknown(subclass, prog_if),
            _ => Self::Unknown(subclass, prog_if),
        }
    }
}

impl DisplaySubclass {
    fn parse(subclass: u8, prog_if: u8) -> Self {
        match subclass {
            0x00 => Self::Vga(VgaProgIf::parse(prog_if)),
            0x01 => Self::Xga(prog_if),
            0x02 => Self::Controller3D(prog_if),
            0x80 => Self::Unknown(subclass, prog_if),
            _ => Self::Unknown(subclass, prog_if),
        }
    }
}

impl VgaProgIf {
    fn parse(prog_if: u8) -> Self {
        match prog_if {
            0x00 => Self::VgaController,
            0x01 => Self::Compatible8514,
            _ => Self::Unknown(prog_if),
        }
    }
}

impl MultimediaSubclass {
    fn parse(subclass: u8, prog_if: u8) -> Self {
        match subclass {
            0x00 => Self::Video(prog_if),
            0x01 => Self::Audio(prog_if),
            0x02 => Self::ComputerTelephony(prog_if),
            0x03 => Self::AudioDevice(prog_if),
            0x80 => Self::Unknown(subclass, prog_if),
            _ => Self::Unknown(subclass, prog_if),
        }
    }
}

impl MemorySubclass {
    fn parse(subclass: u8, prog_if: u8) -> Self {
        match subclass {
            0x00 => Self::Ram(prog_if),
            0x01 => Self::Flash(prog_if),
            0x80 => Self::Unknown(subclass, prog_if),
            _ => Self::Unknown(subclass, prog_if),
        }
    }
}

impl BridgeSubclass {
    fn parse(subclass: u8, prog_if: u8) -> Self {
        match subclass {
            0x00 => Self::Host(prog_if),
            0x01 => Self::Isa(prog_if),
            0x02 => Self::Eisa(prog_if),
            0x03 => Self::Mca(prog_if),
            0x04 => Self::PciToPci(PciToPciProgIf::parse(prog_if)),
            0x05 => Self::Pcmcia(prog_if),
            0x06 => Self::NuBus(prog_if),
            0x07 => Self::CardBus(prog_if),
            0x08 => Self::RaceWay(RaceWayProgIf::parse(prog_if)),
            0x09 => Self::PciToPciSemiTransparent(SemiTransparentProgIf::parse(prog_if)),
            0x0A => Self::InfiniBandToPci(prog_if),
            0x80 => Self::Unknown(subclass, prog_if),
            _ => Self::Unknown(subclass, prog_if),
        }
    }
}

impl PciToPciProgIf {
    fn parse(prog_if: u8) -> Self {
        match prog_if {
            0x00 => Self::NormalDecode,
            0x01 => Self::SubtractiveDecode,
            _ => Self::Unknown(prog_if),
        }
    }
}

impl RaceWayProgIf {
    fn parse(prog_if: u8) -> Self {
        match prog_if {
            0x00 => Self::Transparent,
            0x01 => Self::Endpoint,
            _ => Self::Unknown(prog_if),
        }
    }
}

impl SemiTransparentProgIf {
    fn parse(prog_if: u8) -> Self {
        match prog_if {
            0x40 => Self::PrimaryTowardsCpu,
            0x80 => Self::SecondaryTowardsCpu,
            _ => Self::Unknown(prog_if),
        }
    }
}

impl SimpleCommunicationSubclass {
    fn parse(subclass: u8, prog_if: u8) -> Self {
        match subclass {
            0x00 => Self::Serial(SerialProgIf::parse(prog_if)),
            0x01 => Self::Parallel(ParallelProgIf::parse(prog_if)),
            0x02 => Self::MultiportSerial(prog_if),
            0x03 => Self::Modem(ModemProgIf::parse(prog_if)),
            0x04 => Self::Gpib(prog_if),
            0x05 => Self::SmartCard(prog_if),
            0x80 => Self::Unknown(subclass, prog_if),
            _ => Self::Unknown(subclass, prog_if),
        }
    }
}

impl SerialProgIf {
    fn parse(prog_if: u8) -> Self {
        match prog_if {
            0x00 => Self::Compatible8250,
            0x01 => Self::Compatible16450,
            0x02 => Self::Compatible16550,
            0x03 => Self::Compatible16650,
            0x04 => Self::Compatible16750,
            0x05 => Self::Compatible16850,
            0x06 => Self::Compatible16950,
            _ => Self::Unknown(prog_if),
        }
    }
}

impl ParallelProgIf {
    fn parse(prog_if: u8) -> Self {
        match prog_if {
            0x00 => Self::Standard,
            0x01 => Self::BiDirectional,
            0x02 => Self::Ecp,
            0x03 => Self::Ieee1284Controller,
            0xFE => Self::Ieee1284Target,
            _ => Self::Unknown(prog_if),
        }
    }
}

impl ModemProgIf {
    fn parse(prog_if: u8) -> Self {
        match prog_if {
            0x00 => Self::Generic,
            0x01 => Self::Hayes16450,
            0x02 => Self::Hayes16550,
            0x03 => Self::Hayes16650,
            0x04 => Self::Hayes16750,
            _ => Self::Unknown(prog_if),
        }
    }
}

impl BaseSystemPeripheralSubclass {
    fn parse(subclass: u8, prog_if: u8) -> Self {
        match subclass {
            0x00 => Self::Pic(PicProgIf::parse(prog_if)),
            0x01 => Self::Dma(DmaProgIf::parse(prog_if)),
            0x02 => Self::Timer(TimerProgIf::parse(prog_if)),
            0x03 => Self::Rtc(RtcProgIf::parse(prog_if)),
            0x04 => Self::PciHotPlug(prog_if),
            0x05 => Self::SdHost(prog_if),
            0x06 => Self::Iommu(prog_if),
            0x80 => Self::Unknown(subclass, prog_if),
            _ => Self::Unknown(subclass, prog_if),
        }
    }
}

impl PicProgIf {
    fn parse(prog_if: u8) -> Self {
        match prog_if {
            0x00 => Self::Generic8259,
            0x01 => Self::IsaCompatible,
            0x02 => Self::EisaCompatible,
            0x10 => Self::IoApic,
            0x20 => Self::IoXApic,
            _ => Self::Unknown(prog_if),
        }
    }
}

impl DmaProgIf {
    fn parse(prog_if: u8) -> Self {
        match prog_if {
            0x00 => Self::Generic8237,
            0x01 => Self::IsaCompatible,
            0x02 => Self::EisaCompatible,
            _ => Self::Unknown(prog_if),
        }
    }
}

impl TimerProgIf {
    fn parse(prog_if: u8) -> Self {
        match prog_if {
            0x00 => Self::Generic8254,
            0x01 => Self::IsaCompatible,
            0x02 => Self::EisaCompatible,
            0x03 => Self::Hpet,
            _ => Self::Unknown(prog_if),
        }
    }
}

impl RtcProgIf {
    fn parse(prog_if: u8) -> Self {
        match prog_if {
            0x00 => Self::Generic,
            0x01 => Self::IsaCompatible,
            _ => Self::Unknown(prog_if),
        }
    }
}

impl InputDeviceSubclass {
    fn parse(subclass: u8, prog_if: u8) -> Self {
        match subclass {
            0x00 => Self::Keyboard(prog_if),
            0x01 => Self::Digitizer(prog_if),
            0x02 => Self::Mouse(prog_if),
            0x03 => Self::Scanner(prog_if),
            0x04 => Self::Gameport(GameportProgIf::parse(prog_if)),
            0x80 => Self::Unknown(subclass, prog_if),
            _ => Self::Unknown(subclass, prog_if),
        }
    }
}

impl GameportProgIf {
    fn parse(prog_if: u8) -> Self {
        match prog_if {
            0x00 => Self::Generic,
            0x10 => Self::Extended,
            _ => Self::Unknown(prog_if),
        }
    }
}

impl DockingStationSubclass {
    fn parse(subclass: u8, _prog_if: u8) -> Self {
        match subclass {
            0x00 => Self::Generic,
            _ => Self::Unknown(subclass),
        }
    }
}

impl ProcessorSubclass {
    fn parse(subclass: u8, prog_if: u8) -> Self {
        match subclass {
            0x00 => Self::I386(prog_if),
            0x01 => Self::I486(prog_if),
            0x02 => Self::Pentium(prog_if),
            0x03 => Self::PentiumPro(prog_if),
            0x10 => Self::Alpha(prog_if),
            0x20 => Self::PowerPc(prog_if),
            0x30 => Self::Mips(prog_if),
            0x40 => Self::CoProcessor(prog_if),
            0x80 => Self::Unknown(subclass, prog_if),
            _ => Self::Unknown(subclass, prog_if),
        }
    }
}

impl SerialBusSubclass {
    fn parse(subclass: u8, prog_if: u8) -> Self {
        match subclass {
            0x00 => Self::FireWire(FireWireProgIf::parse(prog_if)),
            0x01 => Self::AccessBus(prog_if),
            0x02 => Self::Ssa(prog_if),
            0x03 => Self::Usb(UsbProgIf::parse(prog_if)),
            0x04 => Self::FibreChannel(prog_if),
            0x05 => Self::SmBus(prog_if),
            0x06 => Self::InfiniBand(prog_if),
            0x07 => Self::Ipmi(IpmiProgIf::parse(prog_if)),
            0x08 => Self::Sercos(prog_if),
            0x09 => Self::CanBus(prog_if),
            0x80 => Self::Unknown(subclass, prog_if),
            _ => Self::Unknown(subclass, prog_if),
        }
    }
}

impl FireWireProgIf {
    fn parse(prog_if: u8) -> Self {
        match prog_if {
            0x00 => Self::Generic,
            0x10 => Self::Ohci,
            _ => Self::Unknown(prog_if),
        }
    }
}

impl UsbProgIf {
    fn parse(prog_if: u8) -> Self {
        match prog_if {
            0x00 => Self::Uhci,
            0x10 => Self::Ohci,
            0x20 => Self::Ehci,
            0x30 => Self::Xhci,
            0x80 => Self::Unspecified,
            0xFE => Self::UsbDevice,
            _ => Self::Unknown(prog_if),
        }
    }
}

impl IpmiProgIf {
    fn parse(prog_if: u8) -> Self {
        match prog_if {
            0x00 => Self::Smic,
            0x01 => Self::KeyboardControllerStyle,
            0x02 => Self::BlockTransfer,
            _ => Self::Unknown(prog_if),
        }
    }
}

impl WirelessSubclass {
    fn parse(subclass: u8, prog_if: u8) -> Self {
        match subclass {
            0x00 => Self::Irda(prog_if),
            0x01 => Self::ConsumerIr(prog_if),
            0x10 => Self::Rf(prog_if),
            0x11 => Self::Bluetooth(prog_if),
            0x12 => Self::Broadband(prog_if),
            0x20 => Self::Ethernet802_1a(prog_if),
            0x21 => Self::Ethernet802_1b(prog_if),
            0x80 => Self::Unknown(subclass, prog_if),
            _ => Self::Unknown(subclass, prog_if),
        }
    }
}

impl IntelligentSubclass {
    fn parse(subclass: u8, prog_if: u8) -> Self {
        match subclass {
            0x00 => Self::I20(prog_if),
            _ => Self::Unknown(subclass, prog_if),
        }
    }
}

impl SatelliteCommunicationSubclass {
    fn parse(subclass: u8, prog_if: u8) -> Self {
        match subclass {
            0x01 => Self::Tv(prog_if),
            0x02 => Self::Audio(prog_if),
            0x03 => Self::Voice(prog_if),
            0x04 => Self::Data(prog_if),
            _ => Self::Unknown(subclass, prog_if),
        }
    }
}

impl EncryptionSubclass {
    fn parse(subclass: u8, prog_if: u8) -> Self {
        match subclass {
            0x00 => Self::NetworkAndComputing(prog_if),
            0x10 => Self::Entertainment(prog_if),
            0x80 => Self::Unknown(subclass, prog_if),
            _ => Self::Unknown(subclass, prog_if),
        }
    }
}

impl SignalProcessingSubclass {
    fn parse(subclass: u8, prog_if: u8) -> Self {
        match subclass {
            0x00 => Self::DpioModules(prog_if),
            0x01 => Self::PerformanceCounters(prog_if),
            0x10 => Self::CommunicationSynchronizer(prog_if),
            0x20 => Self::SignalProcessingManagement(prog_if),
            0x80 => Self::Unknown(subclass, prog_if),
            _ => Self::Unknown(subclass, prog_if),
        }
    }
}
