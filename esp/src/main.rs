mod clock;
mod clock_face;
mod http;
mod max7219;
mod metrics;
mod uart;
mod wifi;

use chrono::{DateTime, Utc};
use esp_idf_hal::prelude::Peripherals;
use esp_idf_svc::nvs::{EspDefaultNvs, EspDefaultNvsPartition};
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

    let sclk = peripherals.pins.gpio25;
    let scs = peripherals.pins.gpio26;
    let sdo = peripherals.pins.gpio27;
    let mut max7219 = max7219::Max7219::new(scs, sclk, sdo, 8);
    max7219.clear();
    max7219.set_intensity(2);

    /*
     * ........
     * .##..##.
     * ########
     * ########
     * .######.
     * ..####..
     * ...##...
     * ........
     */
    /*
    max7219.shift_out(1, &[0b0011_0000, 0b0000_1111, 0b1111_0000, 0b1010_0101]);
    max7219.shift_out(2, &[0b0111_1000, 0b0000_1111, 0b1111_0000, 0b1010_0101]);
    max7219.shift_out(3, &[0b0111_1100, 0b0000_1111, 0b1111_0000, 0b1010_0101]);
    max7219.shift_out(4, &[0b0011_1110, 0b0000_1111, 0b1111_0000, 0b1010_0101]);
    max7219.shift_out(5, &[0b0011_1110, 0b0000_1111, 0b1111_0000, 0b1010_0101]);
    max7219.shift_out(6, &[0b0111_1100, 0b0000_1111, 0b1111_0000, 0b1010_0101]);
    max7219.shift_out(7, &[0b0111_1000, 0b0000_1111, 0b1111_0000, 0b1010_0101]);
    max7219.shift_out(8, &[0b0011_0000, 0b0000_1111, 0b1111_0000, 0b1010_0101]);
    */
    max7219.render("1234567890123456");
    let gpsserver = Arc::new(Mutex::new(GPSServer::new()));
    let gpsserver2 = gpsserver.clone();
    let u = uart::Ublox::new(peripherals.uart1, tx, rx);

    let _ = u.write(&disable_nmea(9600));

    let (metric_tx, metric_rx) = mpsc::channel();
    let metric_tx2 = metric_tx.clone();
    let metric_tx3 = metric_tx.clone();

    let nvsp = EspDefaultNvsPartition::take().unwrap();
    let nvs = EspDefaultNvs::new(nvsp.clone(), "name", true).unwrap();

    let c = clock_face::ClockFace::with_nvs(nvs);
    let clockm = Arc::new(Mutex::new(c));
    let clockm2 = clockm.clone();

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

        let _w = wifi::configure(SSID, PASS, nvsp, modem).expect("Could not configure wifi");
        println!("Wifi is up");

        s.spawn(move || {
            println!("Handling NTP queries");
            handle_ntp_queries(gpsserver2, metric_tx2)
        });

        println!("Serving metrics");
        let _h = http::server(metrics2, clockm).expect("Could not start up metrics server");

        loop {
            let now = clockm2.lock().unwrap().now();
            let t = now.time();
            // TODO add dot back
            let tstr = t.format("%H:%M:%S.6%f").to_string(); // only 3/6/9 are allowed: https://github.com/chronotope/chrono/issues/956
            max7219.render(&tstr[0..tstr.len()]);
            // update clock
            thread::sleep(Duration::from_millis(1));
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
