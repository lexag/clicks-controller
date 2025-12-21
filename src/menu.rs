use crate::{
    events::{Action, Mode},
    state::SystemState,
    textentry::TextEntryContext,
};
use common::mem::str::StaticString;

#[derive(Clone, Copy)]
pub struct MenuItem {
    pub text: StaticString<32>,
    pub value: fn(SystemState) -> StaticString<32>,
    pub exec: fn(SystemState) -> Option<Action>,
}

const MENU_SIZE: usize = 8;

pub fn items() -> [MenuItem; MENU_SIZE] {
    [
        MenuItem {
            text: StaticString::new("Close Menu"),
            value: |_| StaticString::empty(),
            exec: |_| Some(Action::ModeChange(Mode::Main)),
        },
        MenuItem {
            text: StaticString::new("Core port"),
            value: |state| {
                let mut buf = [0u8; 8];
                let s =
                    format_no_std::show(&mut buf, format_args!("{:>5}", state.core_ip.peek().port))
                        .unwrap_or_default();
                StaticString::new(s)
            },
            exec: |_| {
                Some(Action::TextEntryStart {
                    ctx: TextEntryContext::CorePort,
                    initial_value: StaticString::empty(),
                })
            },
        },
        MenuItem {
            text: StaticString::new("IP"),
            value: |state| state.core_ip.peek().str_from_octets(),
            exec: |_| None,
        },
        MenuItem {
            text: StaticString::new("Menu"),
            value: |_| StaticString::empty(),
            exec: |_| None,
        },
        MenuItem {
            text: StaticString::new("Menu"),
            value: |_| StaticString::empty(),
            exec: |_| None,
        },
        MenuItem {
            text: StaticString::new("Menu"),
            value: |_| StaticString::empty(),
            exec: |_| None,
        },
        MenuItem {
            text: StaticString::new("Menu"),
            value: |_| StaticString::empty(),
            exec: |_| None,
        },
        MenuItem {
            text: StaticString::new("Menu"),
            value: |_| StaticString::empty(),
            exec: |_| None,
        },
    ]
}

pub fn get_items_following_idx<const N: usize>(idx: usize) -> [Option<MenuItem>; N] {
    let n = N.min(MENU_SIZE - idx);
    let mut ret = [const { None }; N];
    let items = items();
    for (i, item) in ret.iter_mut().enumerate().take(n) {
        *item = Some(items[idx + i]);
    }
    ret
}

pub fn get_item(idx: usize) -> MenuItem {
    items()[idx]
}
