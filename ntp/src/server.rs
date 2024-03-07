use crate::proto::*;
use chrono::{DateTime, Utc};
use std::time::Duration;
pub struct GPSServer {
    pub reftime: Option<DateTime<Utc>>,
}

impl GPSServer {
    pub fn new() -> GPSServer {
        GPSServer { reftime: None }
    }

    pub fn update_reference_time(&mut self, dt: DateTime<Utc>) {
        self.reftime = Some(dt);
    }

    pub fn answer_query(
        &self,
        q: NTPQuery,
        received_at: DateTime<Utc>,
        now: DateTime<Utc>,
    ) -> NTPMessage {
        NTPMessage {
            flags: NTPFlags {
                l: match self.reftime {
                    Some(_) => LeapIndicator::NoWarning,
                    None => LeapIndicator::Alarm,
                },
                v: VersionNumber(NTP_VERSION),
                m: Mode::Server,
            },
            peer_stratum: if self.reftime.is_some() {
                NTP_STRATUM_ONE
            } else {
                NTP_STRATUM_UNSYNCHRONIZED
            },
            peer_polling_interval: 4, //q.peer_polling_interval,
            root_delay: Fix32 { i: 0, f: 0 },
            peer_clock_precision: PeerPrecision::from(Duration::from_micros(1)),
            root_dispersion: Fix32 { i: 0, f: 0 },
            ref_id: Reference::GPS,
            ref_tstamp: NTPTimestamp::from(self.reftime.unwrap_or(ntp_zero())),
            origin_tstamp: q.transmit_tstamp,
            rcv_tstamp: NTPTimestamp::from(received_at),
            transmit_tstamp: NTPTimestamp::from(now),
        }
    }
}

impl Default for GPSServer {
    fn default() -> Self {
        Self::new()
    }
}
