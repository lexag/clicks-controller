use embassy_rp::{
    gpio::{Input, Level, Output, Pin, Pull},
    Peri,
};

pub struct ButtonScanner {
    cols: [Input<'static>; 4],
    rows: [Output<'static>; 3],
}

impl ButtonScanner {
    pub fn new(
        col1: Peri<'static, impl Pin>,
        col2: Peri<'static, impl Pin>,
        col3: Peri<'static, impl Pin>,
        col4: Peri<'static, impl Pin>,
        row1: Peri<'static, impl Pin>,
        row2: Peri<'static, impl Pin>,
        row3: Peri<'static, impl Pin>,
    ) -> Self {
        Self {
            cols: [
                Input::new(col1, Pull::Up),
                Input::new(col2, Pull::Up),
                Input::new(col3, Pull::Up),
                Input::new(col4, Pull::Up),
            ],
            rows: [
                Output::new(row1, Level::High),
                Output::new(row2, Level::High),
                Output::new(row3, Level::High),
            ],
        }
    }

    fn button_down(&mut self, col: usize, row: usize) -> bool {
        self.rows[row].set_low();
        let b = self.cols[col].is_low();
        self.rows[row].set_high();
        return b;
    }
}

use crate::events::ButtonEvent;
use crate::events::ButtonId;
use crate::BUTTON_CH;
use embassy_time::Timer;

#[embassy_executor::task]
pub async fn button_scanner_task(mut scanner: ButtonScanner) {
    let tx = BUTTON_CH.sender();

    loop {
        tx.send(ButtonEvent {
            id: ButtonId::MetronomeStart,
            pressed: scanner.button_down(1, 0),
        })
        .await;
        tx.send(ButtonEvent {
            id: ButtonId::MetronomeStop,
            pressed: scanner.button_down(0, 0),
        })
        .await;
        tx.send(ButtonEvent {
            id: ButtonId::Shift,
            pressed: scanner.button_down(2, 0),
        })
        .await;
        tx.send(ButtonEvent {
            id: ButtonId::Menu,
            pressed: scanner.button_down(3, 0),
        })
        .await;
        tx.send(ButtonEvent {
            id: ButtonId::MetronomeTempoPlus,
            pressed: scanner.button_down(0, 1),
        })
        .await;
        tx.send(ButtonEvent {
            id: ButtonId::MetronomeTempoMinus,
            pressed: scanner.button_down(0, 2),
        })
        .await;
        tx.send(ButtonEvent {
            id: ButtonId::MetronomeBrightPlus,
            pressed: scanner.button_down(1, 1),
        })
        .await;
        tx.send(ButtonEvent {
            id: ButtonId::MetronomeBrightMinus,
            pressed: scanner.button_down(1, 2),
        })
        .await;
        tx.send(ButtonEvent {
            id: ButtonId::Next,
            pressed: scanner.button_down(3, 1),
        })
        .await;
        tx.send(ButtonEvent {
            id: ButtonId::Previous,
            pressed: scanner.button_down(2, 1),
        })
        .await;
        tx.send(ButtonEvent {
            id: ButtonId::Stop,
            pressed: scanner.button_down(2, 2),
        })
        .await;
        tx.send(ButtonEvent {
            id: ButtonId::Start,
            pressed: scanner.button_down(3, 2),
        })
        .await;
        Timer::after_millis(50).await;
    }
}
