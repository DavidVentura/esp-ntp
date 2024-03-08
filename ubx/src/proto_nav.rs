use crate::helpers::*;
use crate::proto::*;
use chrono::{DateTime, NaiveDate, TimeDelta, Utc};
use std::time::Duration;

#[derive(Debug)]
pub enum NavPacket {
    Status(NavStatus),
    TimeUTC(TimeUTC),
    TimeGPS(TimeGPS),
    SVInfo(SVInfo),
}
impl From<Packet> for NavPacket {
    fn from(p: Packet) -> NavPacket {
        match p.id {
            0x03 => NavPacket::Status(NavStatus::from(p.payload.as_slice())),
            0x20 => NavPacket::TimeGPS(TimeGPS::from(p.payload.as_slice())),
            0x21 => NavPacket::TimeUTC(TimeUTC::from(p.payload.as_slice())),
            0x30 => NavPacket::SVInfo(SVInfo::from(p.payload.as_slice())),
            _ => unimplemented!("idk how to handle id {}", p.id),
        }
    }
}

impl Poll for TimeGPS {
    fn class() -> Class {
        Class::Navigation
    }
    fn id() -> u8 {
        0x20
    }
    fn polling_payload() -> Vec<u8> {
        vec![]
    }
}

#[derive(Debug)]
pub struct SVInfoPoll {}

#[derive(Debug)]
struct SVFlags {
    unhealthy: bool,
}
impl From<u8> for SVFlags {
    fn from(b: u8) -> SVFlags {
        SVFlags {
            unhealthy: (b & 0b10000) > 0,
        }
    }
}
#[derive(Debug)]
struct SVQuality {
    _idle: bool,
    _searching: bool,
    signal_acquired: bool,
}
impl From<u8> for SVQuality {
    fn from(b: u8) -> SVQuality {
        SVQuality {
            _idle: (b & 0b001) > 0,
            _searching: (b & 0b010) > 0,
            signal_acquired: (b & 0b100) > 0,
        }
    }
}

#[derive(Debug)]
pub struct SVInfo {
    pub healthy_channels: u8,
}

impl From<&[u8]> for SVInfo {
    fn from(buf: &[u8]) -> SVInfo {
        let chan_n = buf[4];
        let mut healthy_n = 0;
        for i in 0..chan_n {
            let _chn = buf[8 + 12 * i as usize];
            let _svid = buf[9 + 12 * i as usize];
            let flags = buf[10 + 12 * i as usize];
            let f = SVFlags::from(flags);
            let quality = buf[11 + 12 * i as usize];
            let q = SVQuality::from(quality);

            if !f.unhealthy && q.signal_acquired {
                healthy_n += 1;
            }
        }
        SVInfo {
            healthy_channels: healthy_n,
        }
    }
}
impl Poll for SVInfoPoll {
    fn class() -> Class {
        Class::Navigation
    }
    fn id() -> u8 {
        0x30
    }
    fn polling_payload() -> Vec<u8> {
        vec![]
    }
}

#[derive(Debug)]
pub enum NavFix {
    NoFix,
    DeadReckoning,
    Fix2D,
    Fix3D,
    GpsDeadReckoning,
    TimeOnly,
    Reserved,
}

impl NavFix {
    pub fn valid(&self) -> bool {
        match self {
            NavFix::NoFix => false,
            NavFix::Reserved => false,
            _ => true,
        }
    }
}
impl From<u8> for NavFix {
    fn from(u: u8) -> NavFix {
        match u {
            0 => NavFix::NoFix,
            1 => NavFix::DeadReckoning,
            2 => NavFix::Fix2D,
            3 => NavFix::Fix3D,
            4 => NavFix::GpsDeadReckoning,
            5 => NavFix::TimeOnly,
            _ => NavFix::Reserved,
        }
    }
}

#[derive(Debug)]
pub struct NavStatus {
    _milli: u32,
    pub fix: NavFix,
    _time_to_fix: u32,
    pub uptime: Duration,
}

#[derive(Debug)]
pub struct NavStatusPoll {}
impl Poll for NavStatusPoll {
    fn class() -> Class {
        Class::Navigation
    }
    fn id() -> u8 {
        0x03
    }
    fn polling_payload() -> Vec<u8> {
        vec![]
    }
}

impl From<&[u8]> for NavStatus {
    fn from(buf: &[u8]) -> NavStatus {
        let up = u32::from_le_bytes(buf_to_4u8(&buf[12..16])) as u64;
        NavStatus {
            _milli: u32::from_le_bytes(buf_to_4u8(&buf[0..4])),
            fix: NavFix::from(buf[4]),
            _time_to_fix: u32::from_le_bytes(buf_to_4u8(&buf[8..12])),
            uptime: Duration::from_millis(up),
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
    milli: u32,
    /// -500k .. 500k
    nanos: i32,
    week: i16,
    leap_sec: i8,
    valid_flags: Valid,
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
            _weeks_milli: u32::from_le_bytes(buf_to_4u8(buf)),
            _accuracy: u32::from_le_bytes(buf_to_4u8(&buf[4..8])),
            nanos: i32::from_le_bytes(buf_to_4u8(&buf[8..12])),
            year: u16::from_le_bytes(buf_to_2u8(&buf[12..14])),
            month: buf[14],
            day: buf[15],
            hour: buf[16],
            min: buf[17],
            sec: buf[18],
            _valid: Valid::from(buf[19]),
        }
    }
}

#[derive(Debug)]
pub struct TimeUTC {
    _weeks_milli: u32,
    _accuracy: u32,
    nanos: i32,
    year: u16,
    month: u8,
    day: u8,
    hour: u8,
    min: u8,
    sec: u8,
    _valid: Valid,
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
                _ => panic!(),
            },
            _ => panic!(),
        }
    }
}
