use esp_idf_hal::gpio::{Gpio0, Gpio1, InputPin, OutputPin};
use esp_idf_hal::peripheral::Peripheral;
use esp_idf_hal::sys::EspError;
use esp_idf_hal::uart::config::Config;
use esp_idf_hal::uart::Uart;
use esp_idf_hal::uart::UartDriver;
use esp_idf_hal::units::Hertz;
use std::collections::VecDeque;

pub struct Ublox<'d> {
    pub(crate) u: UartDriver<'d>,
}

impl<'d> Ublox<'d> {
    pub fn new<UART: Uart>(
        uart: impl Peripheral<P = UART> + 'd,
        tx: impl Peripheral<P = impl OutputPin> + 'd,
        rx: impl Peripheral<P = impl InputPin> + 'd,
    ) -> Ublox<'d> {
        let u = UartDriver::new(
            uart,
            tx,
            rx,
            Option::<Gpio0>::None,
            Option::<Gpio1>::None,
            &Config::new().baudrate(Hertz(9600)),
        )
        .expect("Can't set up UartDriver");
        Ublox { u }
    }

    pub fn into_iter(&'d self) -> UbloxIterator<'d> {
        UbloxIterator {
            buf: VecDeque::with_capacity(128),
            u: self,
        }
    }
    pub fn write(&self, buf: &[u8]) -> Result<(), EspError> {
        self.u.write(buf)?;
        Ok(())
    }
}

pub struct UbloxIterator<'a> {
    buf: VecDeque<u8>,
    u: &'a Ublox<'a>,
}

impl<'a> Iterator for UbloxIterator<'a> {
    type Item = u8;
    fn next(&mut self) -> Option<u8> {
        if let Some(b) = self.buf.pop_front() {
            return Some(b);
        }
        let mut read: usize = 0;
        let mut buf = vec![0 as u8; 128];
        while read == 0 {
            read = self.u.u.read(buf.as_mut_slice(), 20).unwrap();
        }
        self.buf.extend(&buf);
        self.buf.pop_front()
    }
}
