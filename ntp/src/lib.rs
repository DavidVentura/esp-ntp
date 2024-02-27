use chrono::{DateTime, NaiveDate, Utc};
use std::net::Ipv4Addr;

const NTP_MESSAGE_LEN: usize = 48;

struct Fix32 {
    i: u16,
    f: u16,
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
impl From<f32> for Fix32 {
    fn from(f: f32) -> Fix32 {
        Fix32 {
            i: f.trunc() as u16,
            f: f.fract() as u16,
        }
    }
}
struct NTPTimestamp {
    int_part: u32,
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

impl From<DateTime<Utc>> for NTPTimestamp {
    fn from(dt: DateTime<Utc>) -> Self {
        let ntp_zero = DateTime::<Utc>::from_naive_utc_and_offset(
            NaiveDate::from_ymd_opt(1900, 1, 1)
                .unwrap()
                .and_hms_opt(0, 0, 0)
                .unwrap(),
            Utc,
        );
        let delta = dt.signed_duration_since(ntp_zero);
        let s = delta.num_seconds();
        let n = delta.subsec_nanos();
        NTPTimestamp {
            int_part: s as u32,
            frac_part: n as u32,
        }
    }
}

struct NTPFlags {
    l: LeapIndicator,
    v: VersionNumber,
    m: Mode,
}
impl Into<u8> for &NTPFlags {
    fn into(self) -> u8 {
        let l: u8 = (&self.l).into();
        let v: u8 = (&self.v).into();
        let m: u8 = (&self.m).into();
        ((l & 0b011) << 6) | ((v & 0b111) << 3) | ((m & 0b11) << 0)
    }
}

enum Reference {
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

struct VersionNumber {
    version: u8,
}

impl Into<VersionNumber> for u8 {
    fn into(self) -> VersionNumber {
        VersionNumber { version: self }
    }
}

enum LeapIndicator {
    NoWarning,
    LastMinuteHas61Seconds,
    LastMinuteHas59Seconds,
    Alarm,
}

impl Into<u8> for &VersionNumber {
    fn into(self) -> u8 {
        self.version & 0b111
    }
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
enum Mode {
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
    flags: NTPFlags,
    peer_stratum: u8,
    peer_polling_interval: u8,
    /// eight signed bits indicates the precision of the clock in seconds, expressed as a power of 2:
    /// -10 means 2^-10 == 1s/1024 == 0.97ms
    peer_clock_precision: u8,
    root_delay: Fix32,
    root_dispersion: Fix32,
    ref_id: Reference,
    ref_tstamp: NTPTimestamp,
    origin_tstamp: NTPTimestamp,
    rcv_tstamp: NTPTimestamp,
    transmit_tstamp: NTPTimestamp,
}

impl NTPMessage {
    pub fn serialize(&self) -> Vec<u8> {
        let mut message = Vec::with_capacity(NTP_MESSAGE_LEN);
        message.push((&self.flags).into());
        message.push(self.peer_stratum);
        message.push(self.peer_polling_interval);
        message.push(self.peer_clock_precision);
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
            v: 3.into(),
            l: LeapIndicator::Alarm,
            m: Mode::SymActive,
        };
        let result: u8 = (&flags).into();
        assert_eq!(result, 0xd9);
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
                v: 3.into(),
                l: LeapIndicator::NoWarning,
                m: Mode::SymPassive,
            },
            peer_stratum: 1,
            peer_polling_interval: 10,
            peer_clock_precision: 0xf0,   //0.000015, TODO
            root_delay: Fix32::from(0.0), // 0.0,
            root_dispersion: Fix32::from(0.000320),
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
                0, 0, 0, 0,
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
