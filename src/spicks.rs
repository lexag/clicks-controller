use embassy_rp::gpio::{Input, Output};
use embedded_hal::spi::FullDuplex;

pub struct Spicks {
    sck: Output<'static>,
    mosi: Output<'static>,
    miso: Input<'static>,
}

//impl FullDuplex<u8> for Spicks {
//    type Error = core::convert::Infallible;
//
//    fn read(&mut self) -> nb::Result<u8, Self::Error> {
//        // read happens during transfer; just return last bit-shifted value
//        Ok(0)
//    }
//
//    fn send(&mut self, byte: u8) -> nb::Result<(), Self::Error> {
//        let mut recv = 0u8;
//
//        for bit in (0..8).rev() {
//            self.mosi.set_level((byte >> bit) & 1 != 0);
//            self.sck.set_high();
//
//            if self.miso.is_high() {
//                recv |= 1 << bit;
//            }
//
//            self.sck.set_low();
//        }
//
//        Ok(())
//    }
//}
