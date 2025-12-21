use crate::state::SystemState;
use common::mem::{network::IpAddress, str::StaticString};

pub enum MenuItemType {
    Label {
        text: StaticString<32>,
    },
    Value {
        text: fn(SystemState) -> StaticString<32>,
        value: fn(SystemState) -> u16,
        max: u16,
        min: u16,
        exec: fn(SystemState),
    },
    IpEdit {
        text: fn(SystemState) -> StaticString<32>,
        ip: fn(SystemState) -> IpAddress,
        exec: fn(SystemState),
    },
    Toggle {
        text: fn(SystemState) -> StaticString<32>,
        val: fn(SystemState) -> bool,
        exec: fn(SystemState),
    },
}

pub struct Menu {
    items: [MenuItemType; 8],
}
