use bitflags::bitflags;
use embassy_rp::{
    gpio::{Level, Output, Pin},
    Peri,
};
use embassy_time::Timer;

bitflags! {
    #[derive(PartialEq, Clone, Copy)]
    pub struct LED: u16 {
    const Metronome = 0x01;
    const Connection = 0x02;
    const Playing = 0x04;
    const VLT = 0x08;
    }
}

pub struct LEDController<'a> {
    pins: [Output<'a>; 4],
}

impl<'a> LEDController<'a> {
    pub fn new(
        metr: Peri<'a, impl Pin>,
        conn: Peri<'a, impl Pin>,
        play: Peri<'a, impl Pin>,
        vlt: Peri<'a, impl Pin>,
    ) -> Self {
        Self {
            pins: [
                Output::new(metr, Level::Low),
                Output::new(conn, Level::Low),
                Output::new(play, Level::Low),
                Output::new(vlt, Level::Low),
            ],
        }
    }

    pub fn demap_pin(&mut self, led: LED) -> &mut Output<'a> {
        match led {
            LED::Metronome => &mut self.pins[0],
            LED::Playing => &mut self.pins[2],
            LED::VLT => &mut self.pins[3],
            _ => &mut self.pins[1],
        }
    }

    pub fn set(&mut self, led: LED, val: bool) {
        self.demap_pin(led).set_level(Level::from(val));
    }

    pub fn toggle(&mut self, led: LED) {
        self.demap_pin(led).toggle();
    }

    pub async fn flash(&mut self, led: LED, duration_us: u64) {
        self.set(led, true);
        Timer::after_micros(duration_us).await;
        self.set(led, false);
    }
}
