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
