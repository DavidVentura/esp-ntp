use crate::proto::*;
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
