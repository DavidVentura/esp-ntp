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
use ubx::proto_nav::{NavPacket, NavStatusPoll, SVInfoPoll, TimeGPS};

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
    let metric_tx2 = metric_tx.clone();
    let metric_tx3 = metric_tx.clone();

    thread::scope(|s| {
        s.spawn(|| {
            poll_ubx(&u);
        });
        s.spawn(|| {
            handle_ubx_feed(&u, gpsserver, metric_tx3);
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

        s.spawn(move || {
            println!("Handling NTP queries");
            handle_ntp_queries(gpsserver2, metric_tx2)
        });

        println!("Serving metrics");
        let _h = http::server(metrics2).expect("Could not start up metrics server");

        loop {
            thread::sleep(Duration::from_millis(100));
        }
    })
}

fn poll_ubx(u: &uart::Ublox<'_>) {
    let buf = TimeGPS::frame();
    let buf2 = NavStatusPoll::frame();
    let buf3 = SVInfoPoll::frame();
    loop {
        let _ = u.write(&buf);
        thread::sleep(Duration::from_secs(1));
        let _ = u.write(&buf2);
        thread::sleep(Duration::from_secs(1));
        let _ = u.write(&buf3);
        thread::sleep(Duration::from_secs(3));
    }
}
fn handle_ubx_feed(
    u: &uart::Ublox<'_>,
    gpsserver: Arc<Mutex<GPSServer>>,
    metrics: mpsc::Sender<Metric>,
) {
    let byte_iter = u.into_iter();
    let mut synced_once = false;
    for packet in PacketIterator::new(byte_iter) {
        let pp = ParsedPacket::from(packet);
        match pp {
            ParsedPacket::Navigation(n) => match n {
                NavPacket::TimeGPS(t) => {
                    metrics.send(Metric::Accuracy(t.accuracy)).unwrap();
                    let now: Option<DateTime<Utc>> = t.into();
                    if now.is_some() {
                        let now = now.unwrap();
                        let adj = (now - clock::now()).num_milliseconds();
                        if synced_once {
                            metrics.send(Metric::ClockAdjust(adj)).unwrap();
                        }
                        synced_once = true;
                        gpsserver.lock().unwrap().update_reference_time(now);
                        clock::set_time(now);
                    }
                }
                NavPacket::Status(s) => {
                    metrics.send(Metric::HasFix(s.fix.valid())).unwrap();
                    metrics.send(Metric::SensorUptime(s.uptime)).unwrap();
                }
                NavPacket::TimeUTC(_t) => {
                    println!("UTC");
                }
                NavPacket::SVInfo(s) => {
                    metrics
                        .send(Metric::SatelliteCount(s.healthy_channels))
                        .unwrap();
                }
            },
            ParsedPacket::Nack => {}
            ParsedPacket::Configuration(c) => {
                println!("Configuration, {:?}", c)
            }
        }
    }
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
        let answer = {
            let srv = s.lock().unwrap();
            if srv.reftime.is_some() {
                metrics.send(Metric::AnsweredNtpQuery).unwrap();
            }
            let now = clock::now();
            srv.answer_query(q, now, now)
        };

        let outbuf = answer.serialize();
        socket.send_to(&outbuf, src)?;
    }
}
