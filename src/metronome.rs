use crate::{led::LED, LED_CONTROLLER, METRONOME_CONTROLLER};
use embassy_time::Timer;
use embedded_time::duration::Microseconds;

pub struct MetronomeController {
    pub bpm: u16,
    pub enabled: bool,
}

impl MetronomeController {
    pub fn new() -> Self {
        Self {
            bpm: 120,
            enabled: false,
        }
    }

    pub fn change_bpm(&mut self, change: i16) {
        self.bpm = self.bpm.saturating_add_signed(change);
    }
}
