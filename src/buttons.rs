use bitflags::bitflags;
use embassy_rp::{
    gpio::{Input, Level, Output, Pin, Pull},
    Peri,
};

bitflags! {
    #[derive(PartialEq, Clone)]
    pub struct Buttons: u16 {
    const MetStart = 0x01;
    const MetStop = 0x02;
    const Back = 0x04;
    const Menu = 0x08;
    const MetTPlus = 0x10;
    const MetTMinus = 0x20;
    const MetBPlus = 0x40;
    const MetBMinus = 0x80;
    const Next = 0x100;
    const Prev = 0x200;
    const Stop = 0x400;
    const Start = 0x800;
    }
}

pub struct ButtonController<'a> {
    cols: [Input<'a>; 4],
    rows: [Output<'a>; 3],
}

impl<'a> ButtonController<'a> {
    pub fn new(
        col1: Peri<'a, impl Pin>,
        col2: Peri<'a, impl Pin>,
        col3: Peri<'a, impl Pin>,
        col4: Peri<'a, impl Pin>,
        row1: Peri<'a, impl Pin>,
        row2: Peri<'a, impl Pin>,
        row3: Peri<'a, impl Pin>,
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

    pub fn button_scan(&mut self) -> Buttons {
        let mut buttons = Buttons::empty();
        buttons.set(Buttons::MetStart, self.button_down(1, 0));
        buttons.set(Buttons::MetStop, self.button_down(0, 0));
        buttons.set(Buttons::Back, self.button_down(2, 0));
        buttons.set(Buttons::Menu, self.button_down(3, 0));
        buttons.set(Buttons::MetTPlus, self.button_down(0, 1));
        buttons.set(Buttons::MetTMinus, self.button_down(0, 2));
        buttons.set(Buttons::MetBPlus, self.button_down(1, 1));
        buttons.set(Buttons::MetBMinus, self.button_down(1, 2));
        buttons.set(Buttons::Next, self.button_down(2, 1));
        buttons.set(Buttons::Prev, self.button_down(3, 1));
        buttons.set(Buttons::Stop, self.button_down(2, 2));
        buttons.set(Buttons::Start, self.button_down(3, 2));
        buttons
    }

    fn button_down(&mut self, col: usize, row: usize) -> bool {
        self.rows[row].set_low();
        let b = self.cols[col].is_low();
        self.rows[row].set_high();
        return b;
    }
}
