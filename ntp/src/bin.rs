use chrono::{offset::Local, Utc};
use ntp::proto::*;
use ntp::server::GPSServer;
use std::net::UdpSocket;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

fn main() -> std::io::Result<()> {
    let s = Arc::new(Mutex::new(GPSServer::new()));
    let s2 = s.clone();

    thread::spawn(move || handle_ntp_queries(s2));
    loop {
        let now = Local::now().with_timezone(&Utc);
        s.lock().unwrap().update_reference_time(now);
        std::thread::sleep(Duration::from_millis(1000));
    }
}

fn handle_ntp_queries(s: Arc<Mutex<GPSServer>>) -> std::io::Result<()> {
    let socket = UdpSocket::bind("127.0.0.1:123")?;
    loop {
        let mut buf = [0; 128];
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
