use crate::proto::*;
use chrono::{DateTime, NaiveDate, Utc};
use std::time::Duration;
pub struct GPSServer {
    reftime: DateTime<Utc>,
}

const NTP_VERSION: u8 = 3;
impl GPSServer {
    pub fn new() -> GPSServer {
        let ntp_zero = DateTime::<Utc>::from_naive_utc_and_offset(
            NaiveDate::from_ymd_opt(1900, 1, 1)
                .unwrap()
                .and_hms_opt(0, 0, 0)
                .unwrap(),
            Utc,
        );
        GPSServer { reftime: ntp_zero }
    }
    pub fn update_reference_time(&mut self, dt: DateTime<Utc>) {
        self.reftime = dt;
    }
    pub fn answer_query(
        &self,
        q: NTPMessage,
        received_at: DateTime<Utc>,
        now: DateTime<Utc>,
    ) -> NTPMessage {
        NTPMessage {
            flags: NTPFlags {
                l: LeapIndicator::NoWarning,
                v: VersionNumber(NTP_VERSION),
                m: Mode::SymPassive,
            },
            peer_stratum: 1,
            peer_polling_interval: q.peer_polling_interval,
            root_delay: Fix32 { i: 0, f: 0 },
            peer_clock_precision: PeerPrecision::from(Duration::from_micros(1)),
            root_dispersion: Fix32 { i: 0, f: 0 },
            ref_id: Reference::GPS,
            ref_tstamp: NTPTimestamp::from(self.reftime),
            origin_tstamp: q.transmit_tstamp,
            rcv_tstamp: NTPTimestamp::from(received_at),
            transmit_tstamp: NTPTimestamp::from(now),
        }
    }
}
