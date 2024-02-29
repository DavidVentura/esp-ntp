use chrono::{DateTime, NaiveDate, Utc};
use std::{net::Ipv4Addr, time::Duration};

pub const NTP_VERSION: u8 = 3;
const NTP_MESSAGE_LEN: usize = 48;

#[derive(PartialEq, Debug)]
pub struct Fix32 {
    pub i: u16,
    pub f: u16,
}

impl Serialize for Fix32 {
    fn serialize(&self) -> Vec<u8> {
        vec![
            ((self.i & 0xff00) >> 8) as u8,
            ((self.i & 0x00ff) >> 0) as u8,
            ((self.f & 0xff00) >> 8) as u8,
            ((self.f & 0x00ff) >> 0) as u8,
        ]
    }
}
// FIXME
impl From<Fix32> for f32 {
    fn from(f: Fix32) -> f32 {
        0.0
    }
}
// FIXME
impl From<f32> for Fix32 {
    fn from(f: f32) -> Fix32 {
        Fix32 {
            i: f.trunc() as u16,
            f: f.fract() as u16,
        }
    }
}
/// Prevision is ~15.2us
impl From<Duration> for Fix32 {
    fn from(d: Duration) -> Self {
        let i = d.as_secs() as u16;
        // subsec_micros is by definition <= 1_000_000, which is < 2**20
        // it's safe to << 12, as a u32, which is equivalent to * 4_000_000

        // the output .f goes up by 1 every ~15.2us
        let micros = d.subsec_micros() * 1_000_000 / 15_258_789;
        Fix32 {
            i,
            f: micros as u16,
        }
    }
}

// This is basically Duration
#[derive(PartialEq, Debug)]
pub struct NTPTimestamp {
    /// seconds since NTP time zero; 1900-1-1 00:00:00 UTC.
    int_part: u32,
    /// nanoseconds
    frac_part: u32,
}

impl Serialize for NTPTimestamp {
    fn serialize(&self) -> Vec<u8> {
        let i = self.int_part.to_be_bytes();
        let f = self.frac_part.to_be_bytes();
        let mut ret = Vec::with_capacity(8);
        ret.extend(i);
        ret.extend(f);
        ret
    }
}

pub fn ntp_zero() -> DateTime<Utc> {
    DateTime::<Utc>::from_naive_utc_and_offset(
        NaiveDate::from_ymd_opt(1900, 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap(),
        Utc,
    )
}

impl From<DateTime<Utc>> for NTPTimestamp {
    fn from(dt: DateTime<Utc>) -> Self {
        let delta = dt.signed_duration_since(ntp_zero());
        let s = delta.num_seconds();
        let n = delta.subsec_nanos();
        NTPTimestamp {
            int_part: s as u32,
            frac_part: n as u32,
        }
    }
}

pub struct NTPFlags {
    pub l: LeapIndicator,
    pub v: VersionNumber,
    pub m: Mode,
}
impl Into<u8> for &NTPFlags {
    fn into(self) -> u8 {
        let l: u8 = (&self.l).into();
        let v: u8 = self.v.0;
        let m: u8 = (&self.m).into();
        ((l & 0b011) << 6) | ((v & 0b111) << 3) | ((m & 0b11) << 0)
    }
}

pub enum Reference {
    GPS,
    IPv4(Ipv4Addr),
}

trait Serialize {
    fn serialize(&self) -> Vec<u8>;
}

impl Serialize for Reference {
    fn serialize(&self) -> Vec<u8> {
        match self {
            Self::GPS => vec![b'G', b'P', b'S', 0],
            Self::IPv4(i) => i.octets().into(),
        }
    }
}

#[derive(Debug, PartialEq, Copy, Clone)]
pub struct PeerPrecision(pub i8);

/// Only tolerates down to nano precision
impl From<Duration> for PeerPrecision {
    fn from(d: Duration) -> Self {
        let s = d.as_secs();
        if s > 0 {
            let trailing = s.next_power_of_two().trailing_zeros();
            return PeerPrecision(trailing as i8);
        }
        let nanos = d.subsec_nanos();
        let trailing = nanos.next_power_of_two().trailing_zeros();
        PeerPrecision(trailing as i8 - 30)
    }
}

impl From<f32> for PeerPrecision {
    fn from(f: f32) -> Self {
        for i in -30..=2 {
            let p = 2_f32.powi(i);
            if p > f {
                return PeerPrecision(i as i8 - 1);
            }
        }
        // panic?
        PeerPrecision(127)
    }
}

impl Into<u8> for PeerPrecision {
    fn into(self) -> u8 {
        self.0 as u8
    }
}
pub struct VersionNumber(pub u8);

pub enum LeapIndicator {
    NoWarning,
    LastMinuteHas61Seconds,
    LastMinuteHas59Seconds,
    Alarm,
}

impl Into<u8> for &Mode {
    fn into(self) -> u8 {
        match self {
            Mode::Unspecified => 0,
            Mode::SymActive => 1,
            Mode::SymPassive => 2,
            Mode::Client => 3,
            Mode::Server => 4,
            Mode::Broadcast => 5,
            Mode::ControlMessage => 6,
            Mode::Reserved => 7,
        }
    }
}
impl Into<u8> for &LeapIndicator {
    fn into(self) -> u8 {
        match self {
            LeapIndicator::NoWarning => 0b00,
            LeapIndicator::LastMinuteHas61Seconds => 0b01,
            LeapIndicator::LastMinuteHas59Seconds => 0b10,
            LeapIndicator::Alarm => 0b11,
        }
    }
}
pub enum Mode {
    Unspecified,
    SymActive,
    SymPassive,
    Client,
    Server,
    Broadcast,
    ControlMessage,
    Reserved,
}

pub struct NTPMessage {
    pub flags: NTPFlags,
    pub peer_stratum: u8,
    pub peer_polling_interval: u8,
    /// eight signed bits indicates the precision of the clock in seconds, expressed as a power of 2:
    /// -10 means 2^-10 == 1s/1024 == 0.97ms
    pub peer_clock_precision: PeerPrecision,
    pub root_delay: Fix32,
    pub root_dispersion: Fix32,
    pub ref_id: Reference,
    pub ref_tstamp: NTPTimestamp,
    pub origin_tstamp: NTPTimestamp,
    pub rcv_tstamp: NTPTimestamp,
    pub transmit_tstamp: NTPTimestamp,
}

impl NTPMessage {
    pub fn serialize(&self) -> Vec<u8> {
        let mut message = Vec::with_capacity(NTP_MESSAGE_LEN);
        message.push((&self.flags).into());
        message.push(self.peer_stratum);
        message.push(self.peer_polling_interval);
        message.push(self.peer_clock_precision.into());
        message.extend(self.root_delay.serialize());
        message.extend(self.root_dispersion.serialize());
        message.extend(self.ref_id.serialize());
        message.extend(self.ref_tstamp.serialize());
        message.extend(self.origin_tstamp.serialize());
        message.extend(self.rcv_tstamp.serialize());
        message.extend(self.transmit_tstamp.serialize());
        message
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serialize_flags() {
        let flags = NTPFlags {
            v: VersionNumber(3),
            l: LeapIndicator::Alarm,
            m: Mode::SymActive,
        };
        let result: u8 = (&flags).into();
        assert_eq!(result, 0xd9);
    }
    #[test]
    fn serialize_root_delay() {
        assert_eq!(Fix32 { i: 0, f: 0 }.serialize(), vec![0, 0, 0, 0]);

        assert_eq!(Fix32 { i: 1, f: 0 }.serialize(), vec![0, 1, 0, 0]);
        assert_eq!(Fix32 { i: 2, f: 0 }.serialize(), vec![0, 2, 0, 0]);

        assert_eq!(Fix32 { i: 256 * 1, f: 0 }.serialize(), vec![1, 0, 0, 0]);
        assert_eq!(Fix32 { i: 256 * 2, f: 0 }.serialize(), vec![2, 0, 0, 0]);

        assert_eq!(Fix32 { i: 0, f: 1 }.serialize(), vec![0, 0, 0, 1]);
        assert_eq!(Fix32 { i: 0, f: 2 }.serialize(), vec![0, 0, 0, 2]);

        assert_eq!(Fix32 { i: 0, f: 256 * 1 }.serialize(), vec![0, 0, 1, 0]);
        assert_eq!(Fix32 { i: 0, f: 256 * 2 }.serialize(), vec![0, 0, 2, 0]);
    }

    #[test]
    fn test_serialize_ref_id() {
        assert_eq!(Reference::GPS.serialize(), vec![b'G', b'P', b'S', 0]);
        assert_eq!(
            Reference::IPv4(Ipv4Addr::new(1, 2, 3, 4)).serialize(),
            vec![1, 2, 3, 4]
        );
    }

    #[test]
    fn test_serialize_timestamp() {
        assert_eq!(
            NTPTimestamp::from(ntp_zero()).serialize(),
            vec![0, 0, 0, 0, 0, 0, 0, 0]
        )
    }

    #[test]
    fn test_peer_precision_duration() {
        assert_eq!(PeerPrecision::from(Duration::from_nanos(1)).0, -30);
        assert_eq!(PeerPrecision::from(Duration::from_micros(1)).0, -20);
        assert_eq!(PeerPrecision::from(Duration::from_millis(1)).0, -10);
        assert_eq!(PeerPrecision::from(Duration::from_secs(1)).0, 0);
        assert_eq!(PeerPrecision::from(Duration::from_secs(2)).0, 1);
    }
    #[test]
    fn test_peer_precision_f32() {
        assert_eq!(PeerPrecision::from(1.0 / 1025.0).0, -11);
        assert_eq!(PeerPrecision::from(1.0 / 1024.0).0, -10);
        assert_eq!(PeerPrecision::from(1.0 / 513.0).0, -10);
        assert_eq!(PeerPrecision::from(1.0 / 512.0).0, -9);
        assert_eq!(PeerPrecision::from(1.0 / 511.0).0, -9);
        assert_eq!(PeerPrecision::from(1.0).0, 0);
    }

    #[test]
    fn test_fix32_from_duration() {
        let got = Fix32::from(Duration::from_micros(1526));
        let expected = Fix32 { i: 0, f: 100 };
        assert_eq!(expected, got);

        let got = Fix32::from(Duration::from_micros(153));
        let expected = Fix32 { i: 0, f: 10 };
        assert_eq!(expected, got);

        let got = Fix32::from(Duration::from_micros(16));
        let expected = Fix32 { i: 0, f: 1 };
        assert_eq!(expected, got);
    }

    #[test]
    fn serialize_root_answer() {
        let ts = DateTime::<Utc>::from_naive_utc_and_offset(
            NaiveDate::from_ymd_opt(2004, 9, 27)
                .unwrap()
                .and_hms_opt(3, 16, 10)
                .unwrap(),
            Utc,
        ); // missing nanos?
        let m = NTPMessage {
            flags: NTPFlags {
                v: VersionNumber(3),
                l: LeapIndicator::NoWarning,
                m: Mode::SymPassive,
            },
            peer_stratum: 1,
            peer_polling_interval: 10,
            peer_clock_precision: PeerPrecision::from(Duration::from_micros(15)),
            root_delay: Fix32::from(Duration::from_micros(0)),
            root_dispersion: Fix32::from(Duration::from_micros(320)),
            ref_id: Reference::GPS,
            ref_tstamp: NTPTimestamp::from(ts),
            origin_tstamp: NTPTimestamp::from(ts),
            rcv_tstamp: NTPTimestamp::from(ts),
            transmit_tstamp: NTPTimestamp::from(ts),
        };

        #[rustfmt::skip]
        assert_eq!(
            m.serialize(),
            vec![
                0x1a, 0x1, 0xa, 0xf0,
                /* root_delay*/
                0, 0, 0, 0,
                /* root_dispersion*/
                0, 0, 0, 0x14,
                /* ref id*/
                b'G', b'P', b'S', 0,
                /* ref tstamp */
                197, 2, 4, 122, 0, 0, 0, 0,
                /* origin tstamp */
                197, 2, 4, 122, 0, 0, 0, 0,
                /* rcv tstamp */
                197, 2, 4, 122, 0, 0, 0, 0,
                /* transmit tstamp */
                197, 2, 4, 122, 0, 0, 0, 0,
            ]
        );
    }
}
