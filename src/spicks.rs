use embedded_hal::digital::{InputPin, OutputPin};
use embedded_hal::spi::SpiBus;

/// Bit-banged SPI bus for W5500 / W55RP20
pub struct Spicks<SCK, MOSI, MISO> {
    sck: SCK,
    mosi: MOSI,
    miso: MISO,
}

impl<SCK, MOSI, MISO> Spicks<SCK, MOSI, MISO>
where
    SCK: OutputPin<Error = ()>,
    MOSI: OutputPin<Error = ()>,
    MISO: InputPin<Error = ()>,
{
    pub fn new(sck: SCK, mosi: MOSI, miso: MISO) -> Self {
        let mut bus = Self { sck, mosi, miso };
        // Idle state: SCK low
        bus.sck.set_low().unwrap();
        bus
    }
}

impl<SCK, MOSI, MISO> SpiBus<u8> for Spicks<SCK, MOSI, MISO>
where
    SCK: OutputPin<Error = ()>,
    MOSI: OutputPin<Error = ()>,
    MISO: InputPin<Error = ()>,
{
    /// Transfer a single buffer
    fn read(&mut self, words: &mut [u8]) -> Result<(), ()> {
        for byte in words.iter_mut() {
            let mut val = 0u8;
            for i in (0..8).rev() {
                self.sck.set_high()?;
                if self.miso.is_high()? {
                    val |= 1 << i;
                }
                self.sck.set_low()?;
            }
            *byte = val;
        }
        Ok(())
    }

    fn write(&mut self, words: &[u8]) -> Result<(), ()> {
        for &byte in words {
            for i in (0..8).rev() {
                // Set MOSI bit
                if (byte >> i) & 1 != 0 {
                    self.mosi.set_high()?;
                } else {
                    self.mosi.set_low()?;
                }

                // Pulse clock
                self.sck.set_high()?;
                self.sck.set_low()?;
            }
        }
        Ok(())
    }

    fn transfer(&mut self, read: &mut [u8], write: &[u8]) -> Result<(), ()> {
        let len = read.len().min(write.len());
        for i in 0..len {
            let mut val = 0u8;
            for bit in (0..8).rev() {
                // Output MOSI
                if (write[i] >> bit) & 1 != 0 {
                    self.mosi.set_high()?;
                } else {
                    self.mosi.set_low()?;
                }

                // Clock high → sample MISO
                self.sck.set_high()?;
                if self.miso.is_high()? {
                    val |= 1 << bit;
                }
                self.sck.set_low()?;
            }
            read[i] = val;
        }
        Ok(())
    }

    fn transfer_in_place(&mut self, words: &mut [u8]) -> Result<(), ()> {
        let mut tmp = [0u8; 256]; // adjust max size as needed
        let len = words.len().min(tmp.len());
        self.transfer(&mut tmp[..len], &words[..len])?;
        words[..len].copy_from_slice(&tmp[..len]);
        Ok(())
    }

    fn flush(&mut self) -> Result<(), ()> {
        Ok(())
    }
}
