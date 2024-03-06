use crate::helpers::*;
use crate::proto::*;
use chrono::{DateTime, NaiveDate, TimeDelta, Utc};
use std::time::Duration;

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
            _ => unimplemented!("idk how to handle id {}", p.id),
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
#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Datelike, Timelike};
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
}
