use chrono::{offset::Local, Utc};
use ntp::proto::*;
use ntp::server::GPSServer;
use std::net::UdpSocket;

fn main() -> std::io::Result<()> {
    let now = Local::now().with_timezone(&Utc);
    let s = GPSServer::new();

    let socket = UdpSocket::bind("127.0.0.1:123")?;
    let mut buf = [0; 512];
    let (amt, src) = socket.recv_from(&mut buf)?;
    if amt != NTP_MESSAGE_LEN {
        println!("baaad");
        return Ok(());
    }
    println!("pkt, {:?}", src);
    let buf = &mut buf[..NTP_MESSAGE_LEN];
    let q = NTPQuery::deserialize(buf).unwrap();
    let a = s.answer_query(q, now, now);

    let outbuf = a.serialize();
    socket.send_to(&outbuf, src)?;
    Ok(())
}
