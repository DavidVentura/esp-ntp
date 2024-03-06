use std::time::Duration;

const QUANTILES: [u8; 4] = [10, 50, 90, 99];

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

#[derive(Debug)]
struct QuantileMetric<T> {
    data: Vec<Option<T>>,
    latest_point: Option<T>,
    idx: u8,
    capacity: u8,
}

impl<T: Copy + Ord> QuantileMetric<T> {
    fn new(cap: u8) -> QuantileMetric<T> {
        QuantileMetric {
            data: vec![None; cap as usize],
            latest_point: None,
            idx: 0,
            capacity: cap,
        }
    }

    fn update(&mut self, point: T) {
        self.data[self.idx as usize] = Some(point);
        self.latest_point = Some(point);
        self.idx = self.idx.wrapping_add(1) % self.capacity;
    }

    fn quantile(&self, q: u8) -> Option<T> {
        let mut filtered = vec![];
        for item in &self.data {
            if item.is_some() {
                filtered.push(item.unwrap());
            }
        }
        if filtered.len() == 0 {
            return None;
        }
        let idx = (q as usize * filtered.len()) / 100;
        filtered.sort_unstable();
        Some(filtered[idx as usize])
    }
}

#[derive(Debug)]
pub struct Metrics {
    /// gauge
    sat_count: QuantileMetric<u8>,
    /// gauge
    has_fix: bool,
    /// gauge
    accuracy: QuantileMetric<Duration>,
    /// counter
    uptime: Duration,
    /// counter
    rcvd_ntp_queries: u32,
    /// counter
    answered_ntp_queries: u32,
    /// gauge
    clock_adjust: QuantileMetric<i64>,

    quantiles: Vec<u8>,
}

impl Default for Metrics {
    fn default() -> Self {
        Metrics::new(&QUANTILES)
    }
}

impl Metrics {
    pub fn new(quantiles: &[u8]) -> Metrics {
        Metrics {
            quantiles: quantiles.to_vec(),
            sat_count: QuantileMetric::new(30),
            accuracy: QuantileMetric::new(30),
            clock_adjust: QuantileMetric::new(30),
            has_fix: false,
            rcvd_ntp_queries: 0,
            answered_ntp_queries: 0,
            uptime: Duration::default(),
        }
    }
    pub fn update(&mut self, m: Metric) {
        match m {
            Metric::SatelliteCount(n) => self.sat_count.update(n),
            Metric::Accuracy(n) => self.accuracy.update(n),
            Metric::ClockAdjust(n) => self.clock_adjust.update(n),
            Metric::HasFix(b) => self.has_fix = b,
            Metric::SensorUptime(n) => self.uptime = n,
            Metric::ReceivedNtpQuery => self.rcvd_ntp_queries += 1,
            Metric::AnsweredNtpQuery => self.answered_ntp_queries += 1,
        }
    }

    // TODO: iter
    pub fn serialize(&self) -> Vec<String> {
        let mut ret = vec![];
        for q in self.quantiles.iter() {
            let quantile = *q;
            match self.sat_count.quantile(quantile) {
                Some(value) => ret.push(format!(
                    r#"satellite_count{{quantile="0.{quantile}"}} {value}"#
                )),
                None => (),
            }
        }
        for q in self.quantiles.iter() {
            let quantile = *q;
            match self.accuracy.quantile(quantile) {
                Some(value) => ret.push(format!(
                    r#"gps_clock_accuracy_us{{quantile="0.{quantile}"}} {}"#,
                    value.as_micros() as u64
                )),
                None => (),
            }
        }
        for q in self.quantiles.iter() {
            let quantile = *q;
            match self.clock_adjust.quantile(quantile) {
                Some(value) => ret.push(format!(
                    r#"rtc_clock_adjust_ms{{quantile="0.{quantile}"}} {value}"#
                )),
                None => (),
            }
        }

        ret.push(format!("has_fix {}", self.has_fix as u8));
        ret.push(format!("received_ntp_queries {}", self.rcvd_ntp_queries));
        ret.push(format!(
            "answered_ntp_queries {}",
            self.answered_ntp_queries
        ));
        ret
    }
}
