use esp_idf_hal::gpio::{Gpio0, Gpio1, InputPin, OutputPin};
use esp_idf_hal::peripheral::Peripheral;
use esp_idf_hal::uart::config::Config;
use esp_idf_hal::uart::Uart;
use esp_idf_hal::uart::UartDriver;
use esp_idf_hal::units::Hertz;
use std::collections::VecDeque;
use ubx::proto::{BadDeserialization, Packet};

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
            &Config::new().baudrate(Hertz(115_200)),
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
}

pub struct UbloxIterator<'a> {
    buf: VecDeque<u8>,
    u: &'a Ublox<'a>,
}

impl<'a> Iterator for UbloxIterator<'a> {
    type Item = u8;
    fn next(&mut self) -> Option<u8> {
        let remaining = self.u.u.remaining_read().unwrap();
        let mut buf = vec![0 as u8; remaining];
        self.u.u.read(buf.as_mut_slice(), 1).unwrap();
        self.buf.extend(&buf);

        if self.buf.len() == 0 {
            self.u.u.read(buf.as_mut_slice(), u32::MAX).unwrap();
            self.buf.extend(&buf);
        }
        self.buf.pop_front()
    }
}
