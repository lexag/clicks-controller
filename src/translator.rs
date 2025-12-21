use crate::events::{Action, ButtonId, Mode};
use crate::{menu, ACTION_SRC, ACTION_UPSTREAM, BUTTON_CH, MODE_SIGNAL, STATE};
use embassy_executor::task;
use embassy_futures::select::{select, Either};

#[task]
pub async fn input_translator_task() {
    let rx_button = BUTTON_CH.receiver();
    let action_tx = ACTION_SRC.sender();

    let mut mode = Mode::Lock;
    let mut shift = false;

    loop {
        let maybe_action = match select(rx_button.receive(), ACTION_UPSTREAM.receive()).await {
            Either::First(btn) => {
                if btn.id == ButtonId::Shift {
                    shift = btn.pressed;
                }

                if btn.pressed {
                    action_lut(mode, shift, btn.id)
                } else {
                    None
                }
            }
            Either::Second(maybe_action) => Some(maybe_action),
        };

        if let Some(action) = maybe_action {
            match action {
                Action::ModeChange(new_mode) => {
                    mode = new_mode;
                }
                Action::TextEntryStart { .. } => {
                    mode = Mode::TextEntry;
                    action_tx.send(Action::ModeChange(mode)).await;
                }
                _ => {}
            }

            MODE_SIGNAL.signal(mode);
            action_tx.send(action).await;
        }
    }
}

fn action_lut(mode: Mode, shift: bool, id: ButtonId) -> Option<Action> {
    match (mode, shift, id) {
        //(Mode::Lock, _, ButtonId::Start) => panic!(),
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
        (Mode::TextEntry, true, ButtonId::Menu) => Some(Action::ModeChange(Mode::Menu)),
        (Mode::TextEntry, false, ButtonId::MetronomeStop) => Some(Action::Character(b'1')),
        (Mode::TextEntry, false, ButtonId::MetronomeStart) => Some(Action::Character(b'2')),
        (Mode::TextEntry, false, ButtonId::MetronomeTempoPlus) => Some(Action::Character(b'3')),
        (Mode::TextEntry, false, ButtonId::MetronomeBrightPlus) => Some(Action::Character(b'4')),
        (Mode::TextEntry, false, ButtonId::MetronomeTempoMinus) => Some(Action::Character(b'5')),
        (Mode::TextEntry, false, ButtonId::MetronomeBrightMinus) => Some(Action::Character(b'6')),
        (Mode::TextEntry, false, ButtonId::Previous) => Some(Action::Backspace),
        (Mode::TextEntry, true, ButtonId::MetronomeStop) => Some(Action::Character(b'7')),
        (Mode::TextEntry, true, ButtonId::MetronomeStart) => Some(Action::Character(b'8')),
        (Mode::TextEntry, true, ButtonId::MetronomeTempoPlus) => Some(Action::Character(b'9')),
        (Mode::TextEntry, true, ButtonId::MetronomeBrightPlus) => Some(Action::Character(b'0')),
        (Mode::TextEntry, true, ButtonId::MetronomeTempoMinus) => Some(Action::Character(b'.')),
        (Mode::TextEntry, true, ButtonId::MetronomeBrightMinus) => Some(Action::Character(b' ')),

        _ => None,
    }
}
