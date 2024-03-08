use std::collections::VecDeque;

use crate::proto_cfg::CfgPacket;
use crate::proto_nav::NavPacket;

pub trait Serialize {
    fn serialize(&self) -> Vec<u8>;
}
pub trait Poll {
    fn class() -> Class;
    fn id() -> u8;
    fn polling_payload() -> Vec<u8>;
}

pub trait Frame {
    fn frame() -> Vec<u8>;
}

impl<T: Poll> Frame for T {
    fn frame() -> Vec<u8> {
        Packet {
            class: Self::class().into(),
            id: Self::id(),
            payload: Self::polling_payload(),
        }
        .serialize()
    }
}

#[derive(Debug)]
pub enum ParsedPacket {
    Navigation(NavPacket),
    Configuration(CfgPacket),
    Nack,
}

impl From<Packet> for ParsedPacket {
    fn from(p: Packet) -> ParsedPacket {
        match p.class {
            Class::Navigation => ParsedPacket::Navigation(NavPacket::from(p)),
            Class::AckNack => ParsedPacket::Nack,
            _ => panic!("what do {:?}", p),
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Class {
    Navigation,
    ReceiverManager,
    Information,
    AckNack,
    ConfigInput,
    Monitoring,
    AssistNowAid,
    Timing,
    Reserved3,
}

impl From<Class> for u8 {
    fn from(c: Class) -> Self {
        match c {
            Class::Navigation => 0x1,
            Class::ReceiverManager => 0x2,
            Class::Reserved3 => 0x3,
            Class::Information => 0x4,
            Class::AckNack => 0x5,
            Class::ConfigInput => 0x6,
            Class::Monitoring => 0xA,
            Class::AssistNowAid => 0xB,
            Class::Timing => 0xD,
        }
    }
}

impl TryFrom<u8> for Class {
    type Error = ();
    fn try_from(u: u8) -> Result<Self, ()> {
        match u {
            0x1 => Ok(Class::Navigation),
            0x2 => Ok(Class::ReceiverManager),
            0x3 => Ok(Class::Reserved3),
            0x4 => Ok(Class::Information),
            0x5 => Ok(Class::AckNack),
            0x6 => Ok(Class::ConfigInput),
            0xA => Ok(Class::Monitoring),
            0xB => Ok(Class::AssistNowAid),
            0xD => Ok(Class::Timing),
            _ => Err(()),
        }
    }
}

#[derive(Debug)]
pub struct Packet {
    pub class: Class,
    pub id: u8,
    pub payload: Vec<u8>,
}

#[derive(Debug)]
pub enum BadDeserialization {
    IncompleteRead,
    BadChecksum,
    BadMagic,
    Unsupported(u8),
}

impl Packet {
    const SYNC_CHAR_1: u8 = 0xb5;
    const SYNC_CHAR_2: u8 = 0x62;
    const MIN_PKT_LEN: usize = 8; // with 0 data len
    pub fn deserialize(buf: &[u8]) -> Result<Packet, BadDeserialization> {
        Packet::from_iter(&mut buf.into_iter().copied())
    }

    fn len_with_frame(&self) -> usize {
        self.payload.len() + Self::MIN_PKT_LEN
    }
    pub fn serialize(&self) -> Vec<u8> {
        let mut v = Vec::with_capacity(self.len_with_frame());
        v.push(Self::SYNC_CHAR_1);
        v.push(Self::SYNC_CHAR_2);
        v.push(u8::from(self.class));
        v.push(self.id);
        v.extend((self.payload.len() as u16).to_le_bytes());
        v.extend(&self.payload);
        let (ck_a, ck_b) = checksum(&v[2..]);
        v.push(ck_a);
        v.push(ck_b);
        v
    }
    pub fn from_iter<I>(iter: &mut I) -> Result<Packet, BadDeserialization>
    where
        I: Iterator<Item = u8>,
    {
        let s1 = iter.next().ok_or(BadDeserialization::IncompleteRead)?;
        if s1 != Self::SYNC_CHAR_1 {
            return Err(BadDeserialization::BadMagic);
        }

        let s2 = iter.next().ok_or(BadDeserialization::IncompleteRead)?;
        if s2 != Self::SYNC_CHAR_2 {
            return Err(BadDeserialization::BadMagic);
        }

        let class_u8 = iter.next().ok_or(BadDeserialization::IncompleteRead)?;
        let id = iter.next().ok_or(BadDeserialization::IncompleteRead)?;
        let (l1, l2) = (
            iter.next().ok_or(BadDeserialization::IncompleteRead)?,
            iter.next().ok_or(BadDeserialization::IncompleteRead)?,
        );

        let payload_len = u16::from_le_bytes([l1, l2]);
        let mut b = Vec::with_capacity(payload_len as usize + Packet::MIN_PKT_LEN);
        b.push(s1);
        b.push(s2);
        b.push(class_u8);
        b.push(id);
        b.push(l1);
        b.push(l2);
        for _ in 0..(payload_len) {
            b.push(iter.next().ok_or(BadDeserialization::IncompleteRead)?);
        }

        let ck_a = iter.next().ok_or(BadDeserialization::IncompleteRead)?;
        let ck_b = iter.next().ok_or(BadDeserialization::IncompleteRead)?;

        let (exp_ck_a, exp_ck_b) = checksum(&b[2..b.len()]);
        if ck_a != exp_ck_a {
            return Err(BadDeserialization::BadChecksum);
        }
        if ck_b != exp_ck_b {
            return Err(BadDeserialization::BadChecksum);
        }

        let class =
            Class::try_from(class_u8).map_err(|_| BadDeserialization::Unsupported(class_u8))?;
        Ok(Packet {
            class,
            id,
            payload: b[6..b.len()].into(),
        })
    }
}

fn checksum(buf: &[u8]) -> (u8, u8) {
    let mut ck_a: u8 = 0;
    let mut ck_b: u8 = 0;
    for b in buf {
        ck_a = ck_a.wrapping_add(*b);
        ck_b = ck_b.wrapping_add(ck_a);
    }

    (ck_a, ck_b)
}

pub struct PacketIterator<I: Iterator<Item = u8>> {
    stream: I,
    consecutive_inc: u8,
    buf: VecDeque<u8>,
}

impl<I: Iterator<Item = u8>> PacketIterator<I> {
    pub fn new(i: I) -> PacketIterator<I>
    where
        I: Iterator<Item = u8>,
    {
        let b = VecDeque::with_capacity(128);
        PacketIterator {
            stream: i,
            buf: b,
            consecutive_inc: 0,
        }
    }
}
impl<I: Iterator<Item = u8>> Iterator for PacketIterator<I> {
    type Item = Packet;
    fn next(&mut self) -> Option<Packet> {
        loop {
            if self.buf.len() == 0 {
                self.buf.push_back(self.stream.next()?);
            }
            let r = Packet::from_iter(&mut self.buf.iter().copied());
            match r {
                Err(BadDeserialization::BadChecksum) => {
                    self.consecutive_inc = 0;
                    self.buf.pop_front();
                }
                Err(BadDeserialization::BadMagic) => {
                    self.consecutive_inc = 0;
                    self.buf.pop_front();
                }
                Err(BadDeserialization::IncompleteRead) => {
                    // This is terrible for performance, but
                    // simple iterators will block when reading, instead of returning None
                    // which means the batch optimization does not work
                    self.buf.push_back(self.stream.next()?);
                    /*
                    // if we trigger IncompleteRead twice in a row,
                    // the upstream iterator returned None twice; so it's empty
                    println!("incomplete");
                    if self.consecutive_inc > 0 {
                        return None;
                    }
                    // this should probably be just `take.collect_into()` (unstable) or
                    // next_chunk() (also unstable)

                    for _ in 0..2 {
                        match self.stream.next() {
                            Some(u) => {
                                println!("batch buf, got b {u:x}");
                                self.buf.push_back(u)
                            }
                            None => {
                                self.consecutive_inc += 1;
                                break;
                            }
                        }
                    }
                    */
                }
                Err(BadDeserialization::Unsupported(class)) => {
                    self.consecutive_inc = 0;
                    self.buf.pop_front();
                    println!("Eeek! {}", class);
                }
                Ok(p) => {
                    self.consecutive_inc = 0;
                    drop(self.buf.drain(..(p.len_with_frame())));
                    return Some(p);
                }
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_checksum() {
        let buf = vec![
            0xb5, 0x62, 0x01, 0x20, 0x10, 0x00, 0xce, 0x74, 0x3e, 0x04, 0x88, 0xcc, 0xfa, 0xff,
            0x81, 0x07, 0x11, 0x07, 0x2c, 0x33, 0x31, 0x01, 0x33, 0x25,
        ];

        let (ck_a, ck_b) = checksum(&buf[2..buf.len() - 2]);
        assert_eq!(ck_a, 0x33);
        assert_eq!(ck_b, 0x25);
    }
    #[test]
    fn test_roundtrip() {
        let inbuf = vec![
            0xb5, 0x62, 0x01, 0x20, 0x10, 0x00, 0xce, 0x74, 0x3e, 0x04, 0x88, 0xcc, 0xfa, 0xff,
            0x81, 0x07, 0x11, 0x07, 0x2c, 0x33, 0x31, 0x01, 0x33, 0x25,
        ];
        let p = Packet::deserialize(&inbuf).unwrap();
        let outbuf = p.serialize();
        assert_eq!(inbuf, outbuf);
    }
    #[test]
    fn from_iterator() {
        let buf = vec![
            0xb5, 0x62, 0x01, 0x20, 0x10, 0x00, 0xce, 0x74, 0x3e, 0x04, 0x88, 0xcc, 0xfa, 0xff,
            0x81, 0x07, 0x11, 0x07, 0x2c, 0x33, 0x31, 0x01, 0x33, 0x25,
        ];
        let p = Packet::from_iter(&mut buf.into_iter()).unwrap();
        assert_eq!(p.class, Class::Navigation);
        assert_eq!(p.id, 0x20);
    }

    #[test]
    fn from_iterator_many() {
        let buf = vec![
            0xb5, 0x62, 0x01, 0x20, 0x10, 0x00, 0xce, 0x74, 0x3e, 0x04, 0x88, 0xcc, 0xfa, 0xff,
            0x81, 0x07, 0x11, 0x07, 0x2c, 0x33, 0x31, 0x01, 0x33, 0x25, 0xb5, 0x62, 0x01, 0x20,
            0x10, 0x00, 0xce, 0x74, 0x3e, 0x04, 0x88, 0xcc, 0xfa, 0xff, 0x81, 0x07, 0x11, 0x07,
            0x2c, 0x33, 0x31, 0x01, 0x33, 0x25,
        ];
        let mut _iter = buf.into_iter();
        let iter = PacketIterator::new(&mut _iter);
        let mut count = 0;
        for p in iter {
            assert_eq!(p.class, Class::Navigation);
            assert_eq!(p.id, 0x20);
            count += 1;
        }
        assert_eq!(count, 2);
    }
    #[test]
    fn from_iterator_leading_garbage() {
        let buf = vec![
            0xaa, 0xaa, 0xbb, /* <-- 3 'noise' elements */
            0xb5, 0x62, 0x01, 0x20, 0x10, 0x00, 0xce, 0x74, 0x3e, 0x04, 0x88, 0xcc, 0xfa, 0xff,
            0x81, 0x07, 0x11, 0x07, 0x2c, 0x33, 0x31, 0x01, 0x33, 0x25, 0xff,
            /* <-- also 1 garbage here */ 0xb5, 0x62, 0x01, 0x20, 0x10, 0x00, 0xce, 0x74,
            0x3e, 0x04, 0x88, 0xcc, 0xfa, 0xff, 0x81, 0x07, 0x11, 0x07, 0x2c, 0x33, 0x31, 0x01,
            0x33, 0x25,
        ];
        let mut _iter = buf.into_iter();
        let iter = PacketIterator::new(&mut _iter);
        let mut count = 0;
        for p in iter {
            assert_eq!(p.class, Class::Navigation);
            assert_eq!(p.id, 0x20);
            count += 1;
        }
        assert_eq!(count, 2);
    }
    #[test]
    fn from_iterator_incomplete_payload() {
        let buf = vec![
            0xb5, 0x62, 0x01, 0x20, 0x10, 0x00, 0xce, 0x74, 0x3e, 0x04, 0x88, 0xcc, 0xfa, 0xff,
            0x81, 0x07, 0x11, 0x07, 0x2c, 0x33, 0x31, 0x01,
        ];
        let mut _iter = buf.into_iter();
        let iter = PacketIterator::new(&mut _iter);
        let mut count = 0;
        for _ in iter {
            count += 1;
        }
        assert_eq!(count, 0);
    }
}
