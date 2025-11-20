use crate::events::{Action, ButtonId, Mode};
use crate::{ACTION_SRC, BUTTON_CH, MODE_SIGNAL};
use embassy_executor::task;

#[task]
pub async fn input_translator_task() {
    let mut rx_button = BUTTON_CH.receiver();
    let action_tx = ACTION_SRC.sender();

    let mut mode = Mode::Main;
    let mut shift = false;

    loop {
        let btn = rx_button.receive().await;
        if btn.id == ButtonId::Shift {
            shift = btn.pressed;
        }

        let maybe_action = action_lut(mode, shift, btn.id);

        if let Some(action) = maybe_action {
            action_tx.send(action).await;

            if let Action::ModeChange(new_mode) = action {
                mode = new_mode;
                MODE_SIGNAL.signal(new_mode);
            }
        }
    }
}

fn action_lut(mode: Mode, shift: bool, id: ButtonId) -> Option<Action> {
    match (mode, shift, id) {
        (Mode::Lock, true, ButtonId::Stop) => Some(Action::ModeChange(Mode::Main)),
        (Mode::Lock, _, _) => None,
        (Mode::Main, false, ButtonId::Menu) => Some(Action::ModeChange(Mode::Menu)),
        (Mode::Main, false, ButtonId::Next) => Some(Action::NextCue),
        (Mode::Main, false, ButtonId::Previous) => Some(Action::PreviousCue),
        (Mode::Main, true, ButtonId::Previous) => Some(Action::SeekCheckpoint),
        (Mode::Menu, false, ButtonId::Next) => Some(Action::NextItem),
        (Mode::Menu, false, ButtonId::Previous) => Some(Action::PreviousItem),
        (Mode::Menu, false, ButtonId::Menu) => Some(Action::SelectItem),
        (Mode::Menu, true, ButtonId::Menu) => Some(Action::ModeChange(Mode::Main)),
        (Mode::TextEntry, false, ButtonId::Menu) => Some(Action::Confirm),
        (Mode::TextEntry, true, ButtonId::Menu) => Some(Action::Escape),
        (Mode::TextEntry, false, ButtonId::MetronomeStop) => Some(Action::Character(b'1')),
        (Mode::TextEntry, false, ButtonId::MetronomeStart) => Some(Action::Character(b'2')),
        (Mode::TextEntry, false, ButtonId::MetronomeTempoPlus) => Some(Action::Character(b'3')),
        (Mode::TextEntry, false, ButtonId::MetronomeBrightPlus) => Some(Action::Character(b'4')),
        (Mode::TextEntry, false, ButtonId::MetronomeTempoMinus) => Some(Action::Character(b'5')),
        (Mode::TextEntry, false, ButtonId::MetronomeBrightMinus) => Some(Action::Character(b'6')),
        (Mode::TextEntry, true, ButtonId::MetronomeStop) => Some(Action::Character(b'7')),
        (Mode::TextEntry, true, ButtonId::MetronomeStart) => Some(Action::Character(b'8')),
        (Mode::TextEntry, true, ButtonId::MetronomeTempoPlus) => Some(Action::Character(b'9')),
        (Mode::TextEntry, true, ButtonId::MetronomeBrightPlus) => Some(Action::Character(b'0')),
        (Mode::TextEntry, true, ButtonId::MetronomeTempoMinus) => Some(Action::Character(b'.')),
        (Mode::TextEntry, true, ButtonId::MetronomeBrightMinus) => Some(Action::Character(b' ')),

        _ => None,
    }
}
