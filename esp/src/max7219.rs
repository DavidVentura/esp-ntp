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
    pub fn shift_out(&mut self, hb: u8, lb: &[u8]) {
        self.scs.set_low().unwrap();
        let data = lb.to_vec();

        for lb in data.iter().rev() {
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
        // each char is half-width, so they have to be tacked together
        assert!(data.len() <= self.display_count * 2);

        let mut glyphs = vec![]; // 8 = rows
        for c in data.chars().rev() {
            let f = self.font(c);
            glyphs.push(f);
        }

        let mut concat_glyphs = vec![];
        for pair in glyphs.chunks(2) {
            let mut concat_ = vec![]; // 8 = rows 2 glyps, 8x8
            for row in 0..8 {
                let v = if pair.len() == 2 {
                    (pair[0][row]) | (pair[1][row] << 4)
                } else {
                    pair[0][row]
                };
                concat_.push(v);
            }
            concat_glyphs.push(concat_);
        }
        for rowidx in 0..8 {
            let mut rowdata = vec![];
            for g in concat_glyphs.iter() {
                rowdata.push(g[rowidx]);
            }
            self.shift_out(1 + rowidx as u8, &rowdata);
        }
    }

    #[rustfmt::skip]
    fn font(&self, c: char) -> [u8; 8] {
        match c {
            '1' => [
                0b0000,
                0b0001,
                0b0011,
                0b0001,
                0b0001,
                0b0001,
                0b0000,
                0b0000,

            ],
            '2' => [
                0b0000,
                0b0111,
                0b0001,
                0b0111,
                0b0100,
                0b0111,
                0b0000,
                0b0000,
            ],
            '3' => [
                0b0000,
                0b0111,
                0b0001,
                0b0111,
                0b0001,
                0b0111,
                0b0000,
                0b0000,
            ],
            '4' => [
                0b0000,
                0b0101,
                0b0101,
                0b0111,
                0b0001,
                0b0001,
                0b0000,
                0b0000,
            ],
            '5' => [
                0b0000,
                0b0111,
                0b0100,
                0b0111,
                0b0001,
                0b0111,
                0b0000,
                0b0000,
            ],
            '6' => [
                0b0000,
                0b0111,
                0b0100,
                0b0111,
                0b0101,
                0b0111,
                0b0000,
                0b0000,
            ],
            '7' => [
                0b0000,
                0b0111,
                0b0001,
                0b0001,
                0b0001,
                0b0001,
                0b0000,
                0b0000,
            ],
            '8' => [
                0b0000,
                0b0111,
                0b0101,
                0b0111,
                0b0101,
                0b0111,
                0b0000,
                0b0000,
            ],
            '9' => [
                0b0000,
                0b0111,
                0b0101,
                0b0111,
                0b0001,
                0b0001,
                0b0000,
                0b0000,
            ],
            '0' => [
                0b0000,
                0b0111,
                0b0101,
                0b0101,
                0b0101,
                0b0111,
                0b0000,
                0b0000,
            ],
            ':' => [
                0b0000,
                0b0000,
                0b0000,
                0b0110,
                0b0000,
                0b0110,
                0b0000,
                0b0000,
            ],
            ' ' => [
                0b0000,
                0b0000,
                0b0000,
                0b0000,
                0b0000,
                0b0000,
                0b0000,
                0b0000,
            ],
            other => panic!("Unhandled {other}"),
        }
    }
}
