use crate::{
    events::{Action, Mode},
    ACTION_SRC, ACTION_UPSTREAM, MODE_SIGNAL, STATE, UX_CH,
};
use common::mem::{network::IpAddress, str::StaticString};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TextEntryContext {
    Unknown,
    CoreIPv4,
    CorePort,
}

#[embassy_executor::task]
pub async fn text_entry_task() {
    let rx = UX_CH.receiver();
    let tx = ACTION_UPSTREAM.sender(); // publish updates to UI

    loop {
        while MODE_SIGNAL.wait().await != crate::events::Mode::TextEntry {}

        let mut buffer = StaticString::<32>::new("");
        let mut cursor = 0;
        let mut edit_context = TextEntryContext::Unknown;

        loop {
            let action = rx.recv().await;
            match action {
                Action::TextEntryStart { ctx, initial_value } => {
                    edit_context = ctx;
                    buffer = initial_value;
                    cursor = buffer.len()
                }
                Action::Character(digit) if cursor < 31 => {
                    buffer.set_char(cursor, digit);
                    let _ = tx.try_send(Action::TextEntryUpdate {
                        ctx: edit_context,
                        value: buffer,
                    });
                    cursor += 1;
                }
                Action::Backspace if cursor > 0 => {
                    buffer.set_char(cursor - 1, 0x00);
                    let _ = tx.try_send(Action::TextEntryUpdate {
                        ctx: edit_context,
                        value: buffer,
                    });
                    cursor -= 1;
                }
                Action::Confirm => {
                    let mut system = STATE.lock().await;
                    match edit_context {
                        TextEntryContext::CorePort => {
                            system.core_ip.port = buffer.str().parse().unwrap_or_default();
                        }
                        TextEntryContext::CoreIPv4 => {
                            system.core_ip =
                                IpAddress::from_str_and_port(buffer.str(), system.core_ip.port)
                                    .unwrap_or_default();
                        }
                        _ => {}
                    }
                    tx.try_send(Action::ModeChange(Mode::Menu));
                    break;
                }
                _ => {}
            }
        }
    }
}
