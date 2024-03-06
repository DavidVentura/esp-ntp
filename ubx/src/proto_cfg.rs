use crate::proto::*;
use std::time::Duration;

#[derive(Debug)]
pub enum CfgPacket {
    Msg(CfgMsg),
    Port(Port),
}

#[derive(Debug)]
#[allow(non_camel_case_types)]
pub enum PortProto {
    UBX,
    NMEA,
    UBX_NMEA,
}
#[derive(Debug)]
pub enum PortMode {
    UART(UartCfg),
}
impl PortMode {}
#[derive(Debug)]
pub enum UartMode {
    Mode8N1,
}
#[derive(Debug)]
pub struct UartCfg {
    pub baudrate: u32,
    pub mode: UartMode,
    pub lsb: bool,
}
#[derive(Debug)]
pub struct Port {
    pub port_mode: PortMode,
    pub proto_in: PortProto,
    pub proto_out: PortProto,
}

impl Serialize for PortProto {
    fn serialize(&self) -> Vec<u8> {
        let mut out = Vec::new();
        let id: u16 = match self {
            PortProto::UBX => 1,
            PortProto::NMEA => 2,
            PortProto::UBX_NMEA => 3,
        };
        out.extend(u16::to_le_bytes(id));
        out
    }
}

impl Serialize for UartCfg {
    fn serialize(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend(u32::to_le_bytes(1)); // 1 = UART; 4 SPI
        out.extend(match self.mode {
            UartMode::Mode8N1 => vec![0xD0, 0x08],
        });
        out.extend(match self.lsb {
            true => u16::to_le_bytes(0),
            false => u16::to_le_bytes(1),
        });
        out.extend(u32::to_le_bytes(self.baudrate));
        out
    }
}
impl Serialize for PortMode {
    fn serialize(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend(match self {
            PortMode::UART(cfg) => cfg.serialize(),
        });
        out
    }
}
impl Serialize for Port {
    fn serialize(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend(self.port_mode.serialize());
        out.extend(self.proto_in.serialize());
        out.extend(self.proto_out.serialize());
        out.extend(vec![0, 0, 0, 0]);
        out
    }
}

#[derive(Debug)]
pub struct CfgMsg {
    pub c: Class,
    pub id: u8,
    pub rate: Duration,
}

impl CfgMsg {
    pub fn serialize_request(&self) -> Vec<u8> {
        let p = Packet {
            class: Class::ConfigInput,
            id: self.id,
            //B5 62 hdr
            //06 08 class id
            //06 00 len
            //E8 03 01 00 01 00 payload
            //01 39 ck
            payload: vec![0xe8, 0x03, 0x1, 0x0, 0x1, 0x0],
        };
        p.serialize()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn portconfig_serialization() {
        let pc = Port {
            port_mode: PortMode::UART(UartCfg {
                baudrate: 9600,
                mode: UartMode::Mode8N1,
                lsb: true,
            }),
            proto_in: PortProto::UBX,
            proto_out: PortProto::UBX,
        };

        let expected = vec![
            0x01, 0x00, 0x00, 0x00, 0xD0, 0x08, 0x00, 0x00, 0x80, 0x25, 0x00, 0x00, 0x01, 0x00,
            0x01, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];
        assert_eq!(pc.serialize(), expected);
    }
}
