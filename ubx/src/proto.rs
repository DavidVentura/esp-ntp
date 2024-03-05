use chrono::{DateTime, NaiveDate, TimeDelta, Utc};
use std::collections::VecDeque;
use std::time::Duration;

pub trait Serialize {
    fn serialize(&self) -> Vec<u8>;
}
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
    /*
    2, 16bit le bitflags (00 01; 00 01), first is PROTO IN, second is PROTO OUT
    B5 62
    06 00
    14 00
    04 00 00 00 00 32 00 00 00 00 00 00 01 00 01 00 00 00 00 00
    52 94

    00 = None
    01 = UBX
    02 = NMEA
    03 = NMEA + UBX
    (others)

    04 = SPI
    uart in 9600 8N1, ubx-ubx, lsb
    B5 62
    06 00
    14 00
    01 00 00 00 D0 08 00 00 80 25 00 00 01 00 01 00 00 00 00 00
    ^^^^^ serial
                ^^^^^ 8N1
                      ^^ LSB (MSB=1)
                            ^^^^^^^^^^^ baudrate
                                        ^^^^^^^^^^^ in/out
    9A 79
    */
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
pub enum NavPacket {
    TimeUTC(TimeUTC),
    TimeGPS(TimeGPS),
}
impl From<Packet> for NavPacket {
    fn from(p: Packet) -> NavPacket {
        match p.id {
            0x20 => NavPacket::TimeGPS(TimeGPS::from(p.payload.as_slice())),
            0x21 => NavPacket::TimeUTC(TimeUTC::from(p.payload.as_slice())),
            _ => panic!("idk how to handle id {}", p.id),
        }
    }
}

impl TimeGPS {
    pub fn serialize_request() -> Vec<u8> {
        let p = Packet {
            class: Class::Navigation,
            id: 0x20,
            payload: vec![],
        };
        p.serialize()
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

#[derive(Debug)]
pub enum ParsedPacket {
    Navigation(NavPacket),
    Configuration(CfgPacket),
    Nack,
}

impl From<Packet> for ParsedPacket {
    fn from(p: Packet) -> ParsedPacket {
        match p.class {
            Class::Navigation => ParsedPacket::Navigation(NavPacket::from(p)),
            Class::AckNack => ParsedPacket::Nack,
            _ => panic!("what do {:?}", p),
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Class {
    Navigation,
    ReceiverManager,
    Information,
    AckNack,
    ConfigInput,
    Monitoring,
    AssistNowAid,
    Timing,
    Reserved3,
}

impl From<Class> for u8 {
    fn from(c: Class) -> Self {
        match c {
            Class::Navigation => 0x1,
            Class::ReceiverManager => 0x2,
            Class::Reserved3 => 0x3,
            Class::Information => 0x4,
            Class::AckNack => 0x5,
            Class::ConfigInput => 0x6,
            Class::Monitoring => 0xA,
            Class::AssistNowAid => 0xB,
            Class::Timing => 0xD,
        }
    }
}

impl TryFrom<u8> for Class {
    type Error = ();
    fn try_from(u: u8) -> Result<Self, ()> {
        match u {
            0x1 => Ok(Class::Navigation),
            0x2 => Ok(Class::ReceiverManager),
            0x3 => Ok(Class::Reserved3),
            0x4 => Ok(Class::Information),
            0x5 => Ok(Class::AckNack),
            0x6 => Ok(Class::ConfigInput),
            0xA => Ok(Class::Monitoring),
            0xB => Ok(Class::AssistNowAid),
            0xD => Ok(Class::Timing),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Valid {
    pub time_of_week: bool,
    pub week_num: bool,
    pub leap_sec: bool,
}
impl From<u8> for Valid {
    fn from(b: u8) -> Valid {
        Valid {
            time_of_week: (b & 0x1) > 0,
            week_num: (b & 0x2) > 0,
            leap_sec: (b & 0x3) > 0,
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct TimeGPS {
    pub milli: u32,
    /// -500k .. 500k
    pub nanos: i32,
    pub week: i16,
    pub leap_sec: i8,
    pub valid_flags: Valid,
    pub accuracy: Duration,
}

fn buf_to_2u8(buf: &[u8]) -> [u8; 2] {
    [buf[0], buf[1]]
}
fn buf_to_4u8(buf: &[u8]) -> [u8; 4] {
    [buf[0], buf[1], buf[2], buf[3]]
}

impl From<&[u8]> for TimeGPS {
    fn from(buf: &[u8]) -> TimeGPS {
        TimeGPS {
            milli: u32::from_le_bytes(buf_to_4u8(buf)),
            nanos: i32::from_le_bytes(buf_to_4u8(&buf[4..8])),
            week: i16::from_le_bytes(buf_to_2u8(&buf[8..10])),
            leap_sec: buf[10] as i8,
            valid_flags: Valid::from(buf[11]),
            accuracy: Duration::from_nanos(u32::from_le_bytes(buf_to_4u8(&buf[12..])) as u64),
        }
    }
}

impl From<TimeGPS> for Option<DateTime<Utc>> {
    fn from(t: TimeGPS) -> Option<DateTime<Utc>> {
        if t.accuracy > Duration::from_millis(100) {
            return None;
        }
        if !t.valid_flags.time_of_week || !t.valid_flags.week_num || !t.valid_flags.leap_sec {
            return None;
        }
        // https://www.gps.gov/technical/icwg/IS-GPS-200G.pdf, page 39
        let d = DateTime::<Utc>::from_naive_utc_and_offset(
            NaiveDate::from_ymd_opt(1980, 1, 6)
                .unwrap()
                .and_hms_opt(0, 0, 0)
                .unwrap(),
            Utc,
        );

        let d = d + TimeDelta::weeks(t.week as i64);
        let d = d + TimeDelta::milliseconds(t.milli as i64);
        let d = d + TimeDelta::nanoseconds(t.nanos as i64);

        // this converts GPS time to UTC time
        Some(d - TimeDelta::seconds(t.leap_sec as i64))
    }
}

impl From<&[u8]> for TimeUTC {
    fn from(buf: &[u8]) -> TimeUTC {
        TimeUTC {
            weeks_milli: u32::from_le_bytes(buf_to_4u8(buf)),
            accuracy: u32::from_le_bytes(buf_to_4u8(&buf[4..8])),
            nanos: i32::from_le_bytes(buf_to_4u8(&buf[8..12])),
            year: u16::from_le_bytes(buf_to_2u8(&buf[12..14])),
            month: buf[14],
            day: buf[15],
            hour: buf[16],
            min: buf[17],
            sec: buf[18],
            valid: Valid::from(buf[19]),
        }
    }
}

#[derive(Debug)]
pub struct TimeUTC {
    pub weeks_milli: u32,
    pub accuracy: u32,
    pub nanos: i32,
    pub year: u16,
    pub month: u8,
    pub day: u8,
    pub hour: u8,
    pub min: u8,
    pub sec: u8,
    pub valid: Valid,
}

impl From<TimeUTC> for DateTime<Utc> {
    fn from(t: TimeUTC) -> DateTime<Utc> {
        let d = DateTime::<Utc>::from_naive_utc_and_offset(
            NaiveDate::from_ymd_opt(t.year.into(), t.month.into(), t.day.into())
                .unwrap()
                .and_hms_opt(t.hour.into(), t.min.into(), t.sec.into())
                .unwrap(),
            Utc,
        );

        d + TimeDelta::nanoseconds(t.nanos as i64)
    }
}

#[derive(Debug)]
pub struct Packet {
    pub class: Class,
    pub id: u8,
    pub payload: Vec<u8>,
}

#[derive(Debug)]
pub enum BadDeserialization {
    IncompleteRead,
    BadChecksum,
    BadMagic,
    Unsupported(u8),
}

impl Packet {
    const SYNC_CHAR_1: u8 = 0xb5;
    const SYNC_CHAR_2: u8 = 0x62;
    const MIN_PKT_LEN: usize = 8; // with 0 data len
    pub fn deserialize(buf: &[u8]) -> Result<Packet, BadDeserialization> {
        Packet::from_iter(&mut buf.into_iter().copied())
    }

    fn len_with_frame(&self) -> usize {
        self.payload.len() + Self::MIN_PKT_LEN
    }
    pub fn serialize(&self) -> Vec<u8> {
        let mut v = Vec::with_capacity(self.len_with_frame());
        v.push(Self::SYNC_CHAR_1);
        v.push(Self::SYNC_CHAR_2);
        v.push(u8::from(self.class));
        v.push(self.id);
        v.extend((self.payload.len() as u16).to_le_bytes());
        v.extend(&self.payload);
        let (ck_a, ck_b) = checksum(&v[2..]);
        v.push(ck_a);
        v.push(ck_b);
        v
    }
    pub fn from_iter<I>(iter: &mut I) -> Result<Packet, BadDeserialization>
    where
        I: Iterator<Item = u8>,
    {
        let s1 = iter.next().ok_or(BadDeserialization::IncompleteRead)?;
        if s1 != Self::SYNC_CHAR_1 {
            return Err(BadDeserialization::BadMagic);
        }

        let s2 = iter.next().ok_or(BadDeserialization::IncompleteRead)?;
        if s2 != Self::SYNC_CHAR_2 {
            return Err(BadDeserialization::BadMagic);
        }

        let class_u8 = iter.next().ok_or(BadDeserialization::IncompleteRead)?;
        let id = iter.next().ok_or(BadDeserialization::IncompleteRead)?;
        let (l1, l2) = (
            iter.next().ok_or(BadDeserialization::IncompleteRead)?,
            iter.next().ok_or(BadDeserialization::IncompleteRead)?,
        );

        let payload_len = u16::from_le_bytes([l1, l2]);
        let mut b = Vec::with_capacity(payload_len as usize + Packet::MIN_PKT_LEN);
        b.push(s1);
        b.push(s2);
        b.push(class_u8);
        b.push(id);
        b.push(l1);
        b.push(l2);
        for _ in 0..(payload_len) {
            b.push(iter.next().ok_or(BadDeserialization::IncompleteRead)?);
        }

        let ck_a = iter.next().ok_or(BadDeserialization::IncompleteRead)?;
        let ck_b = iter.next().ok_or(BadDeserialization::IncompleteRead)?;

        let (exp_ck_a, exp_ck_b) = checksum(&b[2..b.len()]);
        if ck_a != exp_ck_a {
            return Err(BadDeserialization::BadChecksum);
        }
        if ck_b != exp_ck_b {
            return Err(BadDeserialization::BadChecksum);
        }

        let class =
            Class::try_from(class_u8).map_err(|_| BadDeserialization::Unsupported(class_u8))?;
        Ok(Packet {
            class,
            id,
            payload: b[6..b.len()].into(),
        })
    }
}

fn checksum(buf: &[u8]) -> (u8, u8) {
    let mut ck_a: u8 = 0;
    let mut ck_b: u8 = 0;
    for b in buf {
        ck_a = ck_a.wrapping_add(*b);
        ck_b = ck_b.wrapping_add(ck_a);
    }

    (ck_a, ck_b)
}

pub struct PacketIterator<I: Iterator<Item = u8>> {
    stream: I,
    consecutive_inc: u8,
    buf: VecDeque<u8>,
}

impl<I: Iterator<Item = u8>> PacketIterator<I> {
    pub fn new(i: I) -> PacketIterator<I>
    where
        I: Iterator<Item = u8>,
    {
        let b = VecDeque::with_capacity(128);
        PacketIterator {
            stream: i,
            buf: b,
            consecutive_inc: 0,
        }
    }
}
impl<I: Iterator<Item = u8>> Iterator for PacketIterator<I> {
    type Item = Packet;
    fn next(&mut self) -> Option<Packet> {
        loop {
            if self.buf.len() == 0 {
                self.buf.push_back(self.stream.next()?);
            }
            let r = Packet::from_iter(&mut self.buf.iter().copied());
            match r {
                Err(BadDeserialization::BadChecksum) => {
                    self.consecutive_inc = 0;
                    self.buf.pop_front();
                }
                Err(BadDeserialization::BadMagic) => {
                    self.consecutive_inc = 0;
                    self.buf.pop_front();
                }
                Err(BadDeserialization::IncompleteRead) => {
                    // if we trigger IncompleteRead twice in a row,
                    // the upstream iterator returned None twice; so it's empty

                    self.buf.push_back(self.stream.next()?);
                    /*
                    println!("incomplete");
                    if self.consecutive_inc > 0 {
                        return None;
                    }
                    // this should probably be just `take.collect_into()` (unstable) or
                    // next_chunk() (also unstable)

                    for _ in 0..2 {
                        match self.stream.next() {
                            Some(u) => {
                                println!("batch buf, got b {u:x}");
                                self.buf.push_back(u)
                            }
                            None => {
                                self.consecutive_inc += 1;
                                break;
                            }
                        }
                    }
                    */
                }
                Err(BadDeserialization::Unsupported(class)) => {
                    self.consecutive_inc = 0;
                    self.buf.pop_front();
                    println!("Eeek! {}", class);
                }
                Ok(p) => {
                    self.consecutive_inc = 0;
                    drop(self.buf.drain(..(p.len_with_frame())));
                    return Some(p);
                }
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Datelike, Timelike};
    #[test]
    fn test_checksum() {
        let buf = vec![
            0xb5, 0x62, 0x01, 0x20, 0x10, 0x00, 0xce, 0x74, 0x3e, 0x04, 0x88, 0xcc, 0xfa, 0xff,
            0x81, 0x07, 0x11, 0x07, 0x2c, 0x33, 0x31, 0x01, 0x33, 0x25,
        ];

        let (ck_a, ck_b) = checksum(&buf[2..buf.len() - 2]);
        assert_eq!(ck_a, 0x33);
        assert_eq!(ck_b, 0x25);
    }
    #[test]
    fn test_roundtrip() {
        let inbuf = vec![
            0xb5, 0x62, 0x01, 0x20, 0x10, 0x00, 0xce, 0x74, 0x3e, 0x04, 0x88, 0xcc, 0xfa, 0xff,
            0x81, 0x07, 0x11, 0x07, 0x2c, 0x33, 0x31, 0x01, 0x33, 0x25,
        ];
        let p = Packet::deserialize(&inbuf).unwrap();
        let outbuf = p.serialize();
        assert_eq!(inbuf, outbuf);
    }
    #[test]
    fn parse_gps_packet() {
        let buf = vec![
            0xb5, 0x62, 0x01, 0x20, 0x10, 0x00, 0xce, 0x74, 0x3e, 0x04, 0x88, 0xcc, 0xfa, 0xff,
            0x81, 0x07, 0x11, 0x07, 0x1, 0x00, 0x00, 0x00, 163, 125,
        ];
        let p = Packet::deserialize(&buf).unwrap();
        let pp = ParsedPacket::from(p);
        match pp {
            ParsedPacket::Navigation(n) => match n {
                NavPacket::TimeGPS(t) => {
                    assert_eq!(t.accuracy, Duration::from_nanos(1));
                    // converted from gps week + gps seconds of week with
                    // https://www.labsat.co.uk/index.php/en/gps-time-calculator
                    // 2016-10-30T19:46:24.997659144Z
                    let dt = Option::<DateTime<Utc>>::from(t).unwrap();
                    assert_eq!(dt.year(), 2016);
                    assert_eq!(dt.month(), 10);
                    assert_eq!(dt.day(), 30);
                    assert_eq!(dt.hour(), 19);
                    assert_eq!(dt.minute(), 46);
                    assert_eq!(dt.second(), 24);
                    assert_eq!(dt.nanosecond(), 997659144);
                }
                NavPacket::TimeUTC(t) => {
                    println!("UTC {:?}", DateTime::<Utc>::from(t));
                }
            },
            _ => panic!(),
        }
    }
    #[test]
    fn from_iterator() {
        let buf = vec![
            0xb5, 0x62, 0x01, 0x20, 0x10, 0x00, 0xce, 0x74, 0x3e, 0x04, 0x88, 0xcc, 0xfa, 0xff,
            0x81, 0x07, 0x11, 0x07, 0x2c, 0x33, 0x31, 0x01, 0x33, 0x25,
        ];
        let p = Packet::from_iter(&mut buf.into_iter()).unwrap();
        assert_eq!(p.class, Class::Navigation);
        assert_eq!(p.id, 0x20);
    }

    #[test]
    fn from_iterator_many() {
        let buf = vec![
            0xb5, 0x62, 0x01, 0x20, 0x10, 0x00, 0xce, 0x74, 0x3e, 0x04, 0x88, 0xcc, 0xfa, 0xff,
            0x81, 0x07, 0x11, 0x07, 0x2c, 0x33, 0x31, 0x01, 0x33, 0x25, 0xb5, 0x62, 0x01, 0x20,
            0x10, 0x00, 0xce, 0x74, 0x3e, 0x04, 0x88, 0xcc, 0xfa, 0xff, 0x81, 0x07, 0x11, 0x07,
            0x2c, 0x33, 0x31, 0x01, 0x33, 0x25,
        ];
        let mut _iter = buf.into_iter();
        let iter = PacketIterator::new(&mut _iter);
        let mut count = 0;
        for p in iter {
            assert_eq!(p.class, Class::Navigation);
            assert_eq!(p.id, 0x20);
            count += 1;
        }
        assert_eq!(count, 2);
    }
    #[test]
    fn from_iterator_leading_garbage() {
        let buf = vec![
            0xaa, 0xaa, 0xbb, /* <-- 3 'noise' elements */
            0xb5, 0x62, 0x01, 0x20, 0x10, 0x00, 0xce, 0x74, 0x3e, 0x04, 0x88, 0xcc, 0xfa, 0xff,
            0x81, 0x07, 0x11, 0x07, 0x2c, 0x33, 0x31, 0x01, 0x33, 0x25, 0xff,
            /* <-- also 1 garbage here */ 0xb5, 0x62, 0x01, 0x20, 0x10, 0x00, 0xce, 0x74,
            0x3e, 0x04, 0x88, 0xcc, 0xfa, 0xff, 0x81, 0x07, 0x11, 0x07, 0x2c, 0x33, 0x31, 0x01,
            0x33, 0x25,
        ];
        let mut _iter = buf.into_iter();
        let iter = PacketIterator::new(&mut _iter);
        let mut count = 0;
        for p in iter {
            assert_eq!(p.class, Class::Navigation);
            assert_eq!(p.id, 0x20);
            count += 1;
        }
        assert_eq!(count, 2);
    }
    #[test]
    fn from_iterator_incomplete_payload() {
        let buf = vec![
            0xb5, 0x62, 0x01, 0x20, 0x10, 0x00, 0xce, 0x74, 0x3e, 0x04, 0x88, 0xcc, 0xfa, 0xff,
            0x81, 0x07, 0x11, 0x07, 0x2c, 0x33, 0x31, 0x01,
        ];
        let mut _iter = buf.into_iter();
        let iter = PacketIterator::new(&mut _iter);
        let mut count = 0;
        for _ in iter {
            count += 1;
        }
        assert_eq!(count, 0);
    }

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
