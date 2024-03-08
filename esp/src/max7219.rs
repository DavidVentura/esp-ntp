use esp_idf_hal::gpio::{Output, OutputPin, PinDriver};

pub struct Max7219<'a, CS: OutputPin, CLK: OutputPin, DATA: OutputPin> {
    scs: PinDriver<'a, CS, Output>,
    sclk: PinDriver<'a, CLK, Output>,
    sdo: PinDriver<'a, DATA, Output>,
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
    pub fn new(scs: CS, sclk: CLK, sdo: DATA) -> Self {
        let mut scs = PinDriver::output(scs).unwrap();
        let mut sclk = PinDriver::output(sclk).unwrap();
        let sdo = PinDriver::output(sdo).unwrap();
        sclk.set_high();
        scs.set_high();

        let mut ret = Max7219 { sclk, scs, sdo };

        ret.shift_out(Command::ScanLimit.into(), 7);
        ret.shift_out(Command::DecodeMode.into(), 0); // using an led matrix (not digits)
        ret.shift_out(Command::Shutdown.into(), 1); // not in shutdown mode
        ret.shift_out(Command::DisplayTest.into(), 0); // no display test
        ret
    }

    pub fn set_intensity(&mut self, i: u8) {
        self.shift_out(Command::Intensity.into(), i);
    }

    pub fn clear(&mut self) {
        for j in 1..=8 {
            self.shift_out(j, 0b000000000);
        }
    }
    pub fn shift_out(&mut self, hb: u8, lb: u8) {
        self.scs.set_low().unwrap();
        let data: u16 = ((hb as u16) << 8) | lb as u16;
        for i in 0..16 {
            self.sclk.set_low();
            let bitmask = 1 << (15 - i);
            let bit = data & bitmask;
            if bit > 0 {
                self.sdo.set_high();
            } else {
                self.sdo.set_low();
            }
            self.sclk.set_high();
        }
        self.scs.set_high().unwrap();
    }
}
