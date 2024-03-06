mod clock;
mod http;
mod metrics;
mod uart;
mod wifi;

use chrono::{DateTime, Utc};
use esp_idf_hal::prelude::Peripherals;
use ntp::proto::*;
use ntp::server::GPSServer;
use std::net::UdpSocket;
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::Duration;

use crate::metrics::{Metric, Metrics};
use ubx::helpers::disable_nmea;
use ubx::proto::{Frame, PacketIterator, ParsedPacket};
use ubx::proto_nav::{NavPacket, NavStatusPoll, TimeGPS};

const SSID: &'static str = env!("SSID");
const PASS: &'static str = env!("PASS");

fn main() -> std::io::Result<()> {
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    let peripherals = Peripherals::take().unwrap();
    let modem = peripherals.modem;
    let tx = peripherals.pins.gpio15;
    let rx = peripherals.pins.gpio4;

    let gpsserver = Arc::new(Mutex::new(GPSServer::new()));
    let gpsserver2 = gpsserver.clone();
    let u = uart::Ublox::new(peripherals.uart1, tx, rx);

    let _ = u.write(&disable_nmea(9600));

    let (metric_tx, metric_rx) = mpsc::channel();
    thread::scope(|s| {
        s.spawn(|| {
            let buf = TimeGPS::serialize_request();
            let buf2 = (NavStatusPoll {}).frame();
            loop {
                let _ = u.write(&buf);
                thread::sleep(Duration::from_secs(1));
                let _ = u.write(&buf2);
                thread::sleep(Duration::from_secs(4));
            }
        });
        s.spawn(|| loop {
            let byte_iter = u.into_iter();
            for packet in PacketIterator::new(byte_iter) {
                let pp = ParsedPacket::from(packet);
                // println!("{pp:?}");
                match pp {
                    ParsedPacket::Navigation(n) => match n {
                        NavPacket::TimeGPS(t) => {
                            metric_tx.send(Metric::Accuracy(t.accuracy)).unwrap();
                            let now: Option<DateTime<Utc>> = t.into();
                            if now.is_some() {
                                let now = now.unwrap();
                                let adj = (now - clock::now()).num_milliseconds();
                                metric_tx.send(Metric::ClockAdjust(adj)).unwrap();
                                gpsserver.lock().unwrap().update_reference_time(now);
                                clock::set_time(now);
                            }
                        }
                        NavPacket::Status(s) => {
                            metric_tx.send(Metric::HasFix(s.fix.valid())).unwrap();
                            metric_tx.send(Metric::SensorUptime(s.uptime)).unwrap();
                        }
                        NavPacket::TimeUTC(_t) => {
                            println!("UTC");
                        }
                    },
                    ParsedPacket::Nack => {}
                    ParsedPacket::Configuration(c) => {
                        println!("Configuration, {:?}", c)
                    }
                }
            }
        });

        let metrics = Metrics::default();
        let metrics1 = Arc::new(Mutex::new(metrics));
        let metrics2 = metrics1.clone();
        s.spawn(move || {
            println!("Handling metrics");
            loop {
                let metric = metric_rx.recv().unwrap();
                println!("Metric {:?}", metric);
                metrics1.lock().unwrap().update(metric);
            }
        });

        let _w = wifi::configure(SSID, PASS, modem).expect("Could not configure wifi");
        println!("Wifi is up");

        let metric_tx2 = metric_tx.clone();
        s.spawn(move || {
            println!("Handling NTP queries");
            handle_ntp_queries(gpsserver2, metric_tx2)
        });

        println!("Serving metrics");
        http::server(metrics2).expect("Could not start up metrics server");

        loop {
            thread::sleep(Duration::from_millis(100));
        }
    })
}

fn handle_ntp_queries(
    s: Arc<Mutex<GPSServer>>,
    metrics: mpsc::Sender<Metric>,
) -> std::io::Result<()> {
    let socket = UdpSocket::bind("0.0.0.0:123")?;
    loop {
        let mut buf = [0; 128]; // 48 should be enough
        let (amt, src) = socket.recv_from(&mut buf)?;
        if amt != NTP_MESSAGE_LEN {
            println!("bad, len was {}: {:?}", amt, &buf);
            continue;
        }
        metrics.send(Metric::ReceivedNtpQuery).unwrap();
        println!("pkt from {:?}", src);
        let buf = &mut buf[..NTP_MESSAGE_LEN];
        let q = NTPQuery::deserialize(buf).unwrap();
        let a = {
            let srv = s.lock().unwrap();
            match srv.reftime {
                Some(_) => {
                    let now = clock::now();
                    Some(srv.answer_query(q, now, now))
                }
                None => None,
            }
        };

        match a {
            Some(answer) => {
                let outbuf = answer.serialize();
                socket.send_to(&outbuf, src)?;
                metrics.send(Metric::AnsweredNtpQuery).unwrap();
            }
            None => println!("No GPS fix, dropping packet"),
        }
    }
}
