use chrono::{DateTime, NaiveDate, TimeDelta, Utc};

#[derive(Debug)]
pub enum NavPacket {
    TimeUTC(TimeUTC),
    TimeGPS(TimeGPS),
}
impl<'a> From<Packet<'a>> for NavPacket {
    fn from(p: Packet) -> NavPacket {
        match p.id {
            0x20 => NavPacket::TimeGPS(TimeGPS::from(p.payload)),
            0x21 => NavPacket::TimeUTC(TimeUTC::from(p.payload)),
            _ => panic!("idk how to handle id {}", p.id),
        }
    }
}
#[derive(Debug)]
pub enum ParsedPacket {
    Navigation(NavPacket),
}

impl<'a> From<Packet<'a>> for ParsedPacket {
    fn from(p: Packet) -> ParsedPacket {
        match p.class {
            Class::Navigation => ParsedPacket::Navigation(NavPacket::from(p)),
            _ => panic!(),
        }
    }
}

#[derive(Debug)]
pub enum Class {
    Navigation,
    ReceiverManager,
    Information,
    AckNack,
    ConfigInput,
    Monitoring,
    AssistNowAid,
    Timing,
}

impl From<u8> for Class {
    fn from(u: u8) -> Class {
        match u {
            0x1 => Class::Navigation,
            0x2 => Class::ReceiverManager,
            0x4 => Class::Information,
            0x5 => Class::AckNack,
            0x6 => Class::ConfigInput,
            0xA => Class::Monitoring,
            0xB => Class::AssistNowAid,
            0xD => Class::Timing,
            other => panic!("Illegal class: {other}"),
        }
    }
}

#[derive(Debug)]
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

#[derive(Debug)]
pub struct TimeGPS {
    pub milli: u32,
    /// -500k .. 500k
    pub nanos: i32,
    pub week: i16,
    pub leap_sec: i8,
    pub valid_flags: Valid,
    pub accuracy: u32,
}

fn buf_to_2u8(buf: &[u8]) -> [u8; 2] {
    [buf[0], buf[1]]
}
fn buf_to_4u8(buf: &[u8]) -> [u8; 4] {
    [buf[0], buf[1], buf[2], buf[3]]
}

impl<'a> From<&[u8]> for TimeGPS {
    fn from(buf: &[u8]) -> TimeGPS {
        TimeGPS {
            milli: u32::from_le_bytes(buf_to_4u8(buf)),
            nanos: i32::from_le_bytes(buf_to_4u8(&buf[4..8])),
            week: i16::from_le_bytes(buf_to_2u8(&buf[8..10])),
            leap_sec: buf[10] as i8,
            valid_flags: Valid::from(buf[11]),
            accuracy: u32::from_le_bytes(buf_to_4u8(&buf[12..])),
        }
    }
}
impl From<TimeGPS> for DateTime<Utc> {
    fn from(t: TimeGPS) -> DateTime<Utc> {
        let d = DateTime::<Utc>::from_naive_utc_and_offset(
            NaiveDate::from_ymd_opt(1980, 1, 6)
                .unwrap()
                .and_hms_opt(0, 0, 0)
                .unwrap(),
            Utc,
        );

        let d = d + TimeDelta::weeks(t.week as i64);
        // is this supposed to be + or -
        let d = d + TimeDelta::seconds(t.leap_sec as i64);
        let d = d + TimeDelta::milliseconds(t.milli as i64);
        let d = d + TimeDelta::nanoseconds(t.nanos as i64);
        d
    }
}
impl<'a> From<&[u8]> for TimeUTC {
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
pub struct Packet<'a> {
    pub class: Class,
    pub id: u8,
    pub payload_len: u16,
    pub payload: &'a [u8],
    pub ck_a: u8,
    pub ck_b: u8,
}

impl<'a> Packet<'a> {
    const SYNC_CHAR_1: u8 = 0xb5;
    const SYNC_CHAR_2: u8 = 0x62;
    const MIN_PKT_LEN: usize = 8; // with 0 data len
    pub fn deserialize(buf: &[u8]) -> Option<Packet> {
        if buf.len() < Self::MIN_PKT_LEN {
            return None;
        }
        if buf[0] != Self::SYNC_CHAR_1 {
            return None;
        }
        if buf[1] != Self::SYNC_CHAR_2 {
            return None;
        }
        let class = buf[2];
        let id = buf[3];
        let len = u16::from_le_bytes([buf[4], buf[5]]);
        if buf.len() != (len as usize + Self::MIN_PKT_LEN) {
            return None;
        }
        let payload = &buf[6..buf.len() - 1];
        let ck_a = buf[buf.len() - 2];
        let ck_b = buf[buf.len() - 1];

        let (exp_ck_a, exp_ck_b) = checksum(&buf[2..buf.len() - 2]);
        if ck_a != exp_ck_a {
            return None;
        }
        if ck_b != exp_ck_b {
            return None;
        }
        Some(Packet {
            class: Class::from(class),
            id,
            payload_len: len,
            payload,
            ck_a,
            ck_b,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_gps_packet() {
        let buf = vec![
            0xb5, 0x62, 0x01, 0x20, 0x10, 0x00, 0xce, 0x74, 0x3e, 0x04, 0x88, 0xcc, 0xfa, 0xff,
            0x81, 0x07, 0x11, 0x07, 0x2c, 0x33, 0x31, 0x01, 0x33, 0x25,
        ];
        let p = Packet::deserialize(&buf).unwrap();
        assert_eq!(p.ck_a, 51);
        assert_eq!(p.ck_b, 37);
        println!("{:?}", p);
        let pp = ParsedPacket::from(p);
        println!("{:?}", pp);
        match pp {
            ParsedPacket::Navigation(n) => match n {
                NavPacket::TimeGPS(t) => {
                    println!("GPS {:?}", DateTime::<Utc>::from(t));
                    // 2016-10-30T19:46:58.997659144Z
                }
                NavPacket::TimeUTC(t) => {
                    println!("UTC {:?}", DateTime::<Utc>::from(t));
                }
            },
        }
    }
}
