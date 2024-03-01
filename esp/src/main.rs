use chrono::{offset::Local, DateTime, Utc};
use esp_idf_hal::prelude::Peripherals;
use ntp::proto::*;
use ntp::server::GPSServer;
use std::net::UdpSocket;
use std::sync::{Arc, Mutex};
use std::thread;

mod uart;
use ubx::proto::{NavPacket, PacketIterator, ParsedPacket};

fn main() -> std::io::Result<()> {
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    let peripherals = Peripherals::take().unwrap();
    let tx = peripherals.pins.gpio5;
    let rx = peripherals.pins.gpio6;

    let s = Arc::new(Mutex::new(GPSServer::new()));
    let s2 = s.clone();

    thread::spawn(move || handle_ntp_queries(s2));

    let u = uart::Ublox::new(peripherals.uart1, tx, rx);
    loop {
        let byte_iter = u.into_iter();
        for packet in PacketIterator::new(byte_iter) {
            let pp = ParsedPacket::from(packet);
            match pp {
                ParsedPacket::Navigation(n) => match n {
                    NavPacket::TimeGPS(t) => {
                        let now: Option<DateTime<Utc>> = t.into();
                        if now.is_some() {
                            s.lock().unwrap().update_reference_time(now.unwrap());
                        }
                    }
                    NavPacket::TimeUTC(_t) => {
                        println!("UTC");
                    }
                },
            }
        }
    }
}

fn handle_ntp_queries(s: Arc<Mutex<GPSServer>>) -> std::io::Result<()> {
    let socket = UdpSocket::bind("0.0.0.0:123")?;
    loop {
        let mut buf = [0; 128]; // 48 should be enough
        let (amt, src) = socket.recv_from(&mut buf)?;
        if amt != NTP_MESSAGE_LEN {
            println!("bad, len was {}: {:?}", amt, &buf);
            continue;
        }
        let now = Local::now().with_timezone(&Utc);
        println!("pkt from {:?}", src);
        let buf = &mut buf[..NTP_MESSAGE_LEN];
        let q = NTPQuery::deserialize(buf).unwrap();
        let a = s.lock().unwrap().answer_query(q, now, now);

        let outbuf = a.serialize();
        socket.send_to(&outbuf, src)?;
    }
}
