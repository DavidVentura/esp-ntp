use std::fs::File;
use std::io::prelude::*;
use ubx::proto::PacketIterator;

#[test]
fn integration_test() {
    let mut f = File::open("rover3.ubx").unwrap();
    let mut buf = Vec::new();
    f.read_to_end(&mut buf).unwrap();
    let mut _iter = buf.into_iter();

    let iter = PacketIterator::new(&mut _iter);
    let mut count = 0;
    for _ in iter {
        count += 1;
    }
    assert_eq!(count, 108491);
}
