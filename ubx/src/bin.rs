use chrono::{DateTime, Utc};
use ntp::proto::*;
use ntp::server::GPSServer;
use std::collections::VecDeque;
use std::env;
use std::io;
use std::net::UdpSocket;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use ubx::proto::*;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        panic!("Expected 1 argument, serial device (ie: /dev/ttyUSB1)");
    }
    let port = serial2::SerialPort::open(args[1].clone(), 9600).unwrap();

    let si = SerialIterator {
        buf: VecDeque::new(),
        port: &port,
    };
    let buf = CfgMsg {
        c: Class::Navigation,
        id: 0x8,
        rate: Duration::from_millis(1000),
    }
    .serialize_request();
    disable_nmea(&port).unwrap();
    port.write(&buf).unwrap();

    let buf = TimeGPS::serialize_request();
    let m_srv = Arc::new(Mutex::new(GPSServer::new()));
    let m_srv2 = m_srv.clone();
    std::thread::scope(|s| {
        s.spawn(|| handle_ntp_queries(m_srv));
        s.spawn(|| loop {
            port.write(&buf).unwrap();
            std::thread::sleep(Duration::from_secs(2));
        });
        for p in PacketIterator::new(si.into_iter()) {
            let pp = ParsedPacket::from(p);
            println!("pp {:?}", pp);
            match pp {
                ParsedPacket::Configuration(_) => println!("cfg {:?}", pp),
                ParsedPacket::Navigation(n) => match n {
                    NavPacket::TimeGPS(t) => {
                        let dt = Option::<DateTime<Utc>>::from(t);
                        println!("dt {:?}", dt);
                        if dt.is_some() {
                            m_srv2.lock().unwrap().update_reference_time(dt.unwrap());
                        }
                    }
                    NavPacket::TimeUTC(t) => {}
                },
                ParsedPacket::Nack => println!("sad nack"),
            };
        }
    });
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
        println!("pkt from {:?}", src);
        let buf = &mut buf[..NTP_MESSAGE_LEN];
        let q = NTPQuery::deserialize(buf).unwrap();
        let a = {
            let gps = s.lock().unwrap();
            match gps.reftime {
                Some(now) => Some(gps.answer_query(q, now, now)),
                None => None,
            }
        };

        match a {
            Some(answer) => {
                let outbuf = answer.serialize();
                socket.send_to(&outbuf, src)?;
            }
            None => {
                println!("GPS server not in sync, dropping packet");
            }
        }
    }
}

fn disable_nmea(port: &serial2::SerialPort) -> Result<(), Box<dyn std::error::Error>> {
    let pc = Port {
        port_mode: PortMode::UART(UartCfg {
            baudrate: 9600,
            mode: UartMode::Mode8N1,
            lsb: true,
        }),
        proto_in: PortProto::UBX,
        proto_out: PortProto::UBX,
    };
    let buf = pc.serialize();
    let p = Packet {
        class: Class::ConfigInput,
        id: 0x0,
        payload: buf,
    };
    let buf = p.serialize();
    println!("shutting up! {:x?}", buf);
    port.write_all(&buf)?;
    Ok(())
}

pub struct SerialIterator<'a> {
    buf: VecDeque<u8>,
    port: &'a serial2::SerialPort,
}

impl<'a> Iterator for SerialIterator<'a> {
    type Item = u8;
    fn next(&mut self) -> Option<u8> {
        if self.buf.len() > 0 {
            let y = self.buf.pop_front();
            return y;
        }
        let mut inbuf = vec![0, 128];
        loop {
            {
                let p = self.port;
                match p.read(inbuf.as_mut_slice()) {
                    Ok(t) => {
                        if t > 0 {
                            self.buf.extend(&inbuf[..t]);
                            break;
                        }
                    }
                    Err(ref e) if e.kind() == io::ErrorKind::TimedOut => (),
                    Err(e) => eprintln!("{:?}", e),
                }
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        let y = self.buf.pop_front();
        y
    }
}
