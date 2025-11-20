use crate::state::SystemState;
use common::mem::{network::IpAddress, str::String32};

pub enum MenuItemType {
    Label {
        text: String32,
    },
    Value {
        text: fn(SystemState) -> String32,
        value: fn(SystemState) -> u16,
        max: u16,
        min: u16,
        exec: fn(SystemState),
    },
    IpEdit {
        text: fn(SystemState) -> String32,
        ip: fn(SystemState) -> IpAddress,
        exec: fn(SystemState),
    },
    Toggle {
        text: fn(SystemState) -> String32,
        val: fn(SystemState) -> bool,
        exec: fn(SystemState),
    },
}

pub struct Menu {
    items: [MenuItemType; 8],
}
