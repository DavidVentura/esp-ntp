use std::time::Duration;
#[derive(Debug)]
pub enum Metric {
    /// todo
    SatelliteCount(u8),
    HasFix(bool),
    Accuracy(Duration),
    SensorUptime(Duration),
    ClockAdjust(i64),
    ReceivedNtpQuery,
    AnsweredNtpQuery,
}

#[derive(Debug, Default)]
pub struct Metrics {
    /// summary, gauge
    sat_count: u8,
    /// gauge
    has_fix: bool,
    /// summary, gauge
    accuracy: Duration,
    /// counter
    uptime: Duration,
    /// counter
    rcvd_ntp_queries: u32,
    /// counter
    answered_ntp_queries: u32,
    /// summary, gauge
    clock_adjust: i64,
}

impl Metrics {
    pub fn update(&mut self, m: Metric) {
        match m {
            Metric::SatelliteCount(n) => self.sat_count = n,
            Metric::HasFix(b) => self.has_fix = b,
            Metric::Accuracy(n) => self.accuracy = n,
            Metric::SensorUptime(n) => self.uptime = n,
            Metric::ReceivedNtpQuery => self.rcvd_ntp_queries += 1,
            Metric::AnsweredNtpQuery => self.answered_ntp_queries += 1,
            Metric::ClockAdjust(n) => self.clock_adjust = n,
        }
    }
    pub fn serialize(&self) -> String {
        format!("{:?}", self)
    }
}
