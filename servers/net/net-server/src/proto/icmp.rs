use core::{fmt, mem};

use libruntime::r#async;
use log::{debug, warn};

use crate::packet::{Packet, PacketCursor};

use super::*;

/// ICMP header struct, representing the header of an ICMP packet.
#[derive(Debug)]
#[repr(packed)]
struct IcmpHeader {
    r#type: u8,
    code: u8,
    checksum: NetU16,
    rest: Rest,
}

#[derive(Debug, Clone, Copy)]
#[repr(transparent)]
struct Rest([u8; 4]);

impl Rest {
    pub const ZERO: Rest = Rest([0; 4]);

    pub fn as_echo(&self) -> &RestEcho {
        const {
            assert!(mem::size_of::<Rest>() == mem::size_of::<RestEcho>());
            assert!(mem::align_of::<Rest>() == mem::align_of::<RestEcho>());
        }

        // Safety: memory layout enforced above
        unsafe { &*(self.0.as_ptr() as *const RestEcho) }
    }

    pub fn as_echo_mut(&mut self) -> &mut RestEcho {
        const {
            assert!(mem::size_of::<Rest>() == mem::size_of::<RestEcho>());
            assert!(mem::align_of::<Rest>() == mem::align_of::<RestEcho>());
        }

        // Safety: memory layout enforced above
        unsafe { &mut *(self.0.as_ptr() as *mut RestEcho) }
    }
}

/// Implementation of rest header for Echo
#[derive(Debug)]
#[repr(packed)]
pub struct RestEcho {
    ident: NetU16,
    seq: NetU16,
}

/// ICMP protocol implementation.
#[derive(Debug)]
pub struct Icmp {}

impl Icmp {
    /// Create a new ICMP protocol instance.
    pub fn new() -> Self {
        Self {}
    }

    /// Process an incoming ICMP packet.
    pub fn receive(&self, metadata: IpMetadata, packet: Packet) {
        if packet.len() < mem::size_of::<IcmpHeader>() {
            warn!(
                "Received packet too short to contain Icmp header: length={}, from={} (dropped)",
                packet.len(),
                metadata.source,
            );
            return;
        }

        let mut cursor = PacketCursor::new(&packet);
        let mut header = cursor
            .read::<IcmpHeader>()
            .expect("Could not read icmp header");

        let payload = cursor
            .read_data(packet.len() - mem::size_of::<IcmpHeader>())
            .expect("Could not read icmp payload");

        let expected_checksum = self.compute_checksum(&mut header, payload.view());
        if header.checksum.to_u16() != expected_checksum {
            warn!(
                "Received packet with invalid header checksum: checksum={:#04x}, expected={:#04x}, from={} (dropped)",
                header.checksum.to_u16(),
                expected_checksum,
                metadata.source,
            );
            return;
        }

        let kind = IcmpKind::from_type_code(header.r#type, header.code);

        match kind {
            IcmpKind::EchoRequest => {
                debug!("Got ping from {}, sending pong", metadata.source);

                let mut packet = PacketBuilder::new();
                for buffer in payload.view() {
                    packet.append_data(buffer);
                }

                r#async::spawn(async move {
                    GlobalProtocols::instance()
                        .icmp()
                        .send(metadata.source, IcmpKind::EchoReply, header.rest, packet)
                        .await
                });
            }

            other => debug!(
                "Received unhandled ICMP packet from {}: {}",
                metadata.source, other
            ),
        }
    }

    fn compute_checksum<'a>(
        &self,
        header: &mut IcmpHeader,
        payload: impl Iterator<Item = &'a [u8]>,
    ) -> u16 {
        // Backup the original checksum value before setting it to zero for calculation
        let checksum_backup = header.checksum;
        header.checksum = NetU16::ZERO;

        let mut computer = Checksum::new();
        computer.update(header);

        // Restore the original checksum value
        header.checksum = checksum_backup;

        computer.update_packet_view(payload);
        computer.finalize()
    }

    async fn send(
        &self,
        destination: IpAddress,
        kind: IcmpKind,
        rest: Rest,
        mut packet: PacketBuilder,
    ) {
        let (r#type, code) = kind.as_type_code();

        let mut header = IcmpHeader {
            r#type,
            code,
            checksum: NetU16::ZERO,
            rest,
        };

        header.checksum = NetU16::from_u16(self.compute_checksum(&mut header, packet.view()));

        packet.prepend(header);

        GlobalProtocols::instance()
            .ip()
            .send(destination, Ip::ICMP, packet)
            .await;
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum IcmpKind {
    EchoReply,
    EchoRequest,
    DestUnreach(DestUnreachCode),
    TimeExceeded(TimeExceededCode),
    Unknown { ty: u8, code: u8 },
}

impl IcmpKind {
    pub const fn from_type_code(r#type: u8, code: u8) -> Self {
        match r#type {
            0 => Self::EchoReply,
            8 => Self::EchoRequest,

            3 => Self::DestUnreach(match code {
                0 => DestUnreachCode::Net,
                1 => DestUnreachCode::Host,
                3 => DestUnreachCode::Port,
                x => DestUnreachCode::Unknown(x),
            }),

            11 => Self::TimeExceeded(match code {
                0 => TimeExceededCode::TtlExpired,
                x => TimeExceededCode::Unknown(x),
            }),

            t => Self::Unknown { ty: t, code },
        }
    }

    pub const fn as_type_code(&self) -> (u8, u8) {
        match *self {
            Self::EchoReply => (0, 0),
            Self::EchoRequest => (8, 0),

            Self::DestUnreach(code) => {
                let code = match code {
                    DestUnreachCode::Net => 0,
                    DestUnreachCode::Host => 1,
                    DestUnreachCode::Port => 3,
                    DestUnreachCode::Unknown(c) => c,
                };
                (3, code)
            }

            Self::TimeExceeded(code) => {
                let code = match code {
                    TimeExceededCode::TtlExpired => 0,
                    TimeExceededCode::Unknown(c) => c,
                };
                (11, code)
            }

            Self::Unknown { ty, code } => (ty, code),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DestUnreachCode {
    Net,
    Host,
    Port,
    Unknown(u8),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TimeExceededCode {
    TtlExpired,
    Unknown(u8),
}

impl fmt::Display for DestUnreachCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Net => write!(f, "network unreachable"),
            Self::Host => write!(f, "host unreachable"),
            Self::Port => write!(f, "port unreachable"),
            Self::Unknown(c) => write!(f, "unknown code {}", c),
        }
    }
}

impl fmt::Display for TimeExceededCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TtlExpired => write!(f, "ttl expired"),
            Self::Unknown(c) => write!(f, "unknown code {}", c),
        }
    }
}

impl fmt::Display for IcmpKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EchoReply => write!(f, "echo reply"),
            Self::EchoRequest => write!(f, "echo request"),

            Self::DestUnreach(code) => {
                write!(f, "destination unreachable ({})", code)
            }

            Self::TimeExceeded(code) => {
                write!(f, "time exceeded ({})", code)
            }

            Self::Unknown { ty, code } => {
                write!(f, "unknown icmp (type {}, code {})", ty, code)
            }
        }
    }
}
