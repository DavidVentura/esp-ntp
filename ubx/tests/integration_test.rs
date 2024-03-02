use std::fs::File;
use std::io::prelude::*;
use ubx::proto::PacketIterator;

#[test]
fn integration_test() {
    // Can parse two arbitrary files downloaded from Ublox
    for (fname, c) in [("rover3.ubx", 108_491), ("rover7.ubx", 245_544)] {
        let mut f = File::open(fname).unwrap();
        let mut buf = Vec::new();
        f.read_to_end(&mut buf).unwrap();
        let mut _iter = buf.into_iter();

        let iter = PacketIterator::new(&mut _iter);
        let mut count = 0;
        for _ in iter {
            count += 1;
        }
        assert_eq!(count, c);
    }
}
