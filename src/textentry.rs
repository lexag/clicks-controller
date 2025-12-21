use crate::{events::Action, ACTION_SRC, MODE_SIGNAL, UX_CH};
use common::mem::str::StaticString;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TextEntryContext {
    Unknown,
    CoreIPv4,
    CorePort,
}

#[embassy_executor::task]
pub async fn text_entry_task() {
    let rx = UX_CH.receiver();
    let tx = ACTION_SRC.sender(); // publish updates to UI

    loop {
        while MODE_SIGNAL.wait().await != crate::events::Mode::TextEntry {}

        let mut buffer = StaticString::<32>::new("");
        let mut buffer_before = buffer.clone();
        let mut cursor = 0;
        let mut edit_context = TextEntryContext::Unknown;

        loop {
            let action = rx.recv().await;
            match action {
                Action::TextEntryStart { ctx, initial_value } => {
                    edit_context = ctx;
                    buffer = initial_value;
                    buffer_before = initial_value;
                }
                Action::Character(digit) if cursor < 31 => {
                    buffer.set_char(cursor, digit + 0x30);
                    let _ = tx.try_send(Action::TextEntryUpdate {
                        ctx: edit_context,
                        value: buffer.clone(),
                    });
                    cursor += 1;
                }
                Action::Backspace if cursor > 0 => {
                    buffer.set_char(cursor, 0x00);
                    let _ = tx.try_send(Action::TextEntryUpdate {
                        ctx: edit_context,
                        value: buffer.clone(),
                    });
                    cursor -= 1;
                }
                Action::Escape => {
                    buffer = buffer_before;
                    let _ = tx.try_send(Action::TextEntryUpdate {
                        ctx: edit_context,
                        value: buffer.clone(),
                    });
                    break;
                }
                Action::Confirm => {
                    let _ = tx.try_send(Action::TextEntryComplete {
                        ctx: edit_context,
                        value: buffer.clone(),
                    });
                    break;
                }
                _ => {}
            }
        }
    }
}
