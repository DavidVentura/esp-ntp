use esp_idf_hal::gpio::{Output, OutputPin, PinDriver};

pub struct Max7219<'a, CS: OutputPin, CLK: OutputPin, DATA: OutputPin> {
    scs: PinDriver<'a, CS, Output>,
    sclk: PinDriver<'a, CLK, Output>,
    sdo: PinDriver<'a, DATA, Output>,
    display_count: usize,
}

enum Command {
    DecodeMode,
    Intensity,
    ScanLimit,
    Shutdown,
    DisplayTest,
}

enum Glyph {
    Thinner([[u8; 1]; 8]),
    Thin([[u8; 2]; 8]),
    Thick([[u8; 4]; 8]),
}

impl From<Command> for u8 {
    fn from(c: Command) -> u8 {
        match c {
            Command::DecodeMode => 0x9,
            Command::Intensity => 0xa,
            Command::ScanLimit => 0xb,
            Command::Shutdown => 0xc,
            Command::DisplayTest => 0xf,
        }
    }
}
impl<'a, CS, CLK, DATA> Max7219<'a, CS, CLK, DATA>
where
    CS: OutputPin,
    CLK: OutputPin,
    DATA: OutputPin,
{
    pub fn new(scs: CS, sclk: CLK, sdo: DATA, display_count: usize) -> Self {
        let mut scs = PinDriver::output(scs).unwrap();
        let mut sclk = PinDriver::output(sclk).unwrap();
        let sdo = PinDriver::output(sdo).unwrap();
        sclk.set_high().unwrap();
        scs.set_high().unwrap();

        let mut ret = Max7219 {
            sclk,
            scs,
            sdo,
            display_count,
        };

        ret.shift_out(Command::ScanLimit.into(), &vec![7; display_count]);
        ret.shift_out(Command::DecodeMode.into(), &vec![0; display_count]); // using an led matrix (not digits)
        ret.shift_out(Command::Shutdown.into(), &vec![1; display_count]); // not in shutdown mode
        ret.shift_out(Command::DisplayTest.into(), &vec![0; display_count]); // no display test
        ret.shift_out(Command::Intensity.into(), &vec![15; display_count]); // no display test
        ret
    }

    pub fn set_intensity(&mut self, i: u8) {
        self.shift_out(Command::Intensity.into(), &vec![i; self.display_count]);
    }

    pub fn clear(&mut self) {
        for j in 1..=8 {
            self.shift_out(j, &vec![0b000000000; self.display_count]);
        }
    }

    fn shift_out_b(&mut self, hb: u8, lb: &[u8]) {
        let mut agg = Vec::with_capacity(lb.len() / 8);
        for bits in lb.chunks(8) {
            // aggregate bits-as-byte into a real byte
            let mut byte: u8 = 0;
            for (i, bit) in bits.iter().enumerate() {
                byte |= bit << i;
            }
            agg.push(byte);
        }
        self.shift_out(hb, &agg)
    }

    pub fn shift_out(&mut self, hb: u8, lb: &[u8]) {
        self.scs.set_low().unwrap();

        for lb in lb.iter().rev() {
            let item: u16 = ((hb as u16) << 8) | *lb as u16;
            for i in 0..16 {
                self.sclk.set_low().unwrap();
                let bitmask = 1 << (15 - i);
                let bit = item & bitmask;
                if bit > 0 {
                    self.sdo.set_high().unwrap();
                } else {
                    self.sdo.set_low().unwrap();
                }
                self.sclk.set_high().unwrap();
            }
        }
        self.scs.set_high().unwrap();
    }

    pub fn render(&mut self, data: &str) {
        // TODO maybe alloc once?
        let mut rowdata: Vec<Vec<u8>> = vec![vec![]; 8]; // 64 width

        let glyphs: Vec<Glyph> = data.chars().rev().map(|c| self.font(c)).collect();
        for pair in glyphs.chunks(2) {
            for row in 0..8 {
                for digit in pair.iter() {
                    match digit {
                        Glyph::Thinner(bits) => rowdata[row].extend(bits[row].iter().rev()),
                        Glyph::Thin(bits) => rowdata[row].extend(bits[row].iter().rev()),
                        Glyph::Thick(bits) => rowdata[row].extend(bits[row].iter().rev()),
                    }
                }
            }
        }
        // 01:00:54.6476399000
        for rowidx in 0..8 {
            let d = &rowdata[rowidx];
            // this clamps _from the end_, we need to push the least-significant-symbol first
            let rd = &d[0..std::cmp::min(d.len(), self.display_count * 8)];
            self.shift_out_b(1 + rowidx as u8, rd);
        }
    }

    fn font(&self, c: char) -> Glyph {
        match c {
            '1' => Glyph::Thick([
                [0, 0, 0, 0],
                [0, 0, 1, 0],
                [0, 1, 1, 0],
                [0, 0, 1, 0],
                [0, 0, 1, 0],
                [0, 1, 1, 1],
                [0, 0, 0, 0],
                [0, 0, 0, 0],
            ]),
            '2' => Glyph::Thick([
                [0, 0, 0, 0],
                [0, 1, 1, 1],
                [0, 0, 0, 1],
                [0, 1, 1, 1],
                [0, 1, 0, 0],
                [0, 1, 1, 1],
                [0, 0, 0, 0],
                [0, 0, 0, 0],
            ]),
            '3' => Glyph::Thick([
                [0, 0, 0, 0],
                [0, 1, 1, 1],
                [0, 0, 0, 1],
                [0, 1, 1, 1],
                [0, 0, 0, 1],
                [0, 1, 1, 1],
                [0, 0, 0, 0],
                [0, 0, 0, 0],
            ]),
            '4' => Glyph::Thick([
                [0, 0, 0, 0],
                [0, 1, 0, 1],
                [0, 1, 0, 1],
                [0, 1, 1, 1],
                [0, 0, 0, 1],
                [0, 0, 0, 1],
                [0, 0, 0, 0],
                [0, 0, 0, 0],
            ]),
            '5' => Glyph::Thick([
                [0, 0, 0, 0],
                [0, 1, 1, 1],
                [0, 1, 0, 0],
                [0, 1, 1, 1],
                [0, 0, 0, 1],
                [0, 1, 1, 1],
                [0, 0, 0, 0],
                [0, 0, 0, 0],
            ]),
            '6' => Glyph::Thick([
                [0, 0, 0, 0],
                [0, 1, 1, 1],
                [0, 1, 0, 0],
                [0, 1, 1, 1],
                [0, 1, 0, 1],
                [0, 1, 1, 1],
                [0, 0, 0, 0],
                [0, 0, 0, 0],
            ]),
            '7' => Glyph::Thick([
                [0, 0, 0, 0],
                [0, 1, 1, 1],
                [0, 0, 0, 1],
                [0, 0, 0, 1],
                [0, 0, 0, 1],
                [0, 0, 0, 1],
                [0, 0, 0, 0],
                [0, 0, 0, 0],
            ]),
            '8' => Glyph::Thick([
                [0, 0, 0, 0],
                [0, 1, 1, 1],
                [0, 1, 0, 1],
                [0, 1, 1, 1],
                [0, 1, 0, 1],
                [0, 1, 1, 1],
                [0, 0, 0, 0],
                [0, 0, 0, 0],
            ]),
            '9' => Glyph::Thick([
                [0, 0, 0, 0],
                [0, 1, 1, 1],
                [0, 1, 0, 1],
                [0, 1, 1, 1],
                [0, 0, 0, 1],
                [0, 0, 0, 1],
                [0, 0, 0, 0],
                [0, 0, 0, 0],
            ]),
            '0' => Glyph::Thick([
                [0, 0, 0, 0],
                [0, 1, 1, 1],
                [0, 1, 0, 1],
                [0, 1, 0, 1],
                [0, 1, 0, 1],
                [0, 1, 1, 1],
                [0, 0, 0, 0],
                [0, 0, 0, 0],
            ]),
            ':' => Glyph::Thin([
                [0, 0],
                [0, 0],
                [0, 1],
                [0, 0],
                [0, 1],
                [0, 0],
                [0, 0],
                [0, 0],
            ]),
            '.' => Glyph::Thin([
                [0, 0],
                [0, 0],
                [0, 0],
                [0, 0],
                [0, 0],
                [0, 1],
                [0, 0],
                [0, 0],
            ]),
            ' ' => Glyph::Thin([
                [0, 0],
                [0, 0],
                [0, 0],
                [0, 0],
                [0, 0],
                [0, 0],
                [0, 0],
                [0, 0],
            ]),
            '!' => Glyph::Thinner([[0], [0], [0], [0], [0], [0], [0], [0]]),
            other => panic!("Unhandled {other}"),
        }
    }
}
