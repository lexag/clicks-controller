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

    let mut last_buttons = [false; 12];

    loop {
        for (i, id, x, y) in [
            (0, ButtonId::MetronomeStart, 1, 0),
            (1, ButtonId::MetronomeStop, 0, 0),
            (2, ButtonId::Shift, 2, 0),
            (3, ButtonId::Menu, 3, 0),
            (4, ButtonId::MetronomeTempoPlus, 0, 1),
            (5, ButtonId::MetronomeTempoMinus, 0, 2),
            (6, ButtonId::MetronomeBrightPlus, 1, 1),
            (7, ButtonId::MetronomeBrightMinus, 1, 2),
            (8, ButtonId::Next, 3, 1),
            (9, ButtonId::Previous, 2, 1),
            (10, ButtonId::Stop, 2, 2),
            (11, ButtonId::Start, 3, 2),
        ] {
            let pressed = scanner.button_down(x, y);
            if pressed != last_buttons[i] {
                last_buttons[i] = pressed;
                tx.send(ButtonEvent { id, pressed }).await;
            }
        }

        Timer::after_millis(50).await;
    }
}
