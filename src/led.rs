use crate::{events::Action, ui::debug, LED_CH};
use bitflags::bitflags;
use embassy_rp::pwm::{self, SetDutyCycle};
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

pub struct LEDController {
    metr: pwm::PwmOutput<'static>,
    conn: pwm::PwmOutput<'static>,
    play: pwm::PwmOutput<'static>,
    vlt: pwm::PwmOutput<'static>,
    metr_val: u16,
    conn_val: u16,
    play_val: u16,
    vlt_val: u16,
}

impl LEDController {
    pub async fn new(metr_conn: pwm::Pwm<'static>, play_vlt: pwm::Pwm<'static>) -> Self {
        if let (Some(metr), Some(conn)) = metr_conn.split() {
            if let (Some(play), Some(vlt)) = play_vlt.split() {
                return Self {
                    metr,
                    conn,
                    play,
                    vlt,
                    metr_val: 0,
                    conn_val: 0,
                    play_val: 0,
                    vlt_val: 0,
                };
            }
        }
        debug("failed pwm init").await;
        Timer::after_secs(5).await;
        panic!();
    }

    pub fn demap_val(&mut self, led: LED) -> &mut u16 {
        match led {
            LED::Metronome => &mut self.metr_val,
            LED::Playing => &mut self.play_val,
            LED::VLT => &mut self.vlt_val,
            _ => &mut self.conn_val,
        }
    }

    pub fn demap_pin(&mut self, led: LED) -> &mut pwm::PwmOutput<'static> {
        match led {
            LED::Metronome => &mut self.metr,
            LED::Playing => &mut self.play,
            LED::VLT => &mut self.vlt,
            _ => &mut self.conn,
        }
    }

    pub fn set(&mut self, led: LED, val: u16) {
        self.demap_pin(led).set_duty_cycle(val);
        *self.demap_val(led) = val;
    }

    pub fn set_bool(&mut self, led: LED, on: bool) {
        let val = if on { u16::MAX } else { 0 };
        self.demap_pin(led).set_duty_cycle(val);
        *self.demap_val(led) = val;
    }

    pub fn toggle(&mut self, led: LED) {
        let pin = self.demap_pin(led);
        let max_duty = pin.max_duty_cycle();
        let current_duty = *self.demap_val(led);
        self.set(led, max_duty - current_duty);
    }

    pub async fn flash(&mut self, led: LED, duration_us: u64) {
        self.set(led, u16::MAX);
        Timer::after_micros(duration_us).await;
        self.set(led, u16::MIN);
    }
}

#[embassy_executor::task]
pub async fn led_task(mut c: LEDController) {
    loop {
        match LED_CH.receive().await {
            Action::LEDSet(led, state) => {
                c.set_bool(led, state);
            }
            Action::LEDToggle(led) => {
                c.toggle(led);
            }
            Action::LEDBlip(led) => {
                c.flash(led, 10000).await;
            }
            Action::NewTransportData(data) => {
                c.set_bool(LED::Playing, data.running);
                c.set_bool(LED::VLT, data.vlt);
            }
            Action::GainConnection => {
                c.set_bool(LED::Connection, true);
            }
            Action::LoseConnection => {
                c.set_bool(LED::Connection, false);
            }
            Action::NewBeatData(_) => {
                c.flash(LED::Metronome, 10000).await;
            }
            _ => {}
        }
    }
}
