use crate::proto::{Class, Packet, Serialize};
use crate::proto_cfg::*;

pub fn disable_nmea(baudrate: u32) -> Vec<u8> {
    let pc = Port {
        port_mode: PortMode::UART(UartCfg {
            baudrate,
            mode: UartMode::Mode8N1,
            lsb: true,
        }),
        proto_in: PortProto::UBX,
        proto_out: PortProto::UBX,
    };
    let buf = pc.serialize();
    let p = Packet {
        class: Class::ConfigInput,
        id: 0x0,
        payload: buf,
    };
    p.serialize()
}
pub(crate) fn buf_to_2u8(buf: &[u8]) -> [u8; 2] {
    [buf[0], buf[1]]
}
pub(crate) fn buf_to_4u8(buf: &[u8]) -> [u8; 4] {
    [buf[0], buf[1], buf[2], buf[3]]
}
