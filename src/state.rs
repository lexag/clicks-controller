use crate::fsm::FSM;
use common::{cue::Cue, event::EventTable, mem::network::IpAddress};

#[derive(Clone, Copy, Default)]
pub struct TrackedValue<T> {
    pub value: T,
    pub dirty: bool,
}

impl<T: Clone> TrackedValue<T> {
    pub const fn new(val: T) -> Self {
        Self {
            value: val,
            dirty: false,
        }
    }

    pub fn set(&mut self, val: T) {
        self.value = val;
        self.dirty = true;
    }

    pub fn read(&mut self) -> T {
        self.dirty = false;
        self.value.clone()
    }

    pub fn read_ref(&mut self) -> &T {
        self.dirty = false;
        &self.value
    }

    pub fn read_dirty(&mut self) -> Option<&T> {
        if self.dirty {
            self.dirty = false;
            return Some(&self.value);
        } else {
            None
        }
    }
}

#[derive(Clone, Default)]
pub struct SystemState {
    pub cue: TrackedValue<Cue>,
    pub cue_idx: TrackedValue<u16>,
    pub beat_idx: TrackedValue<u16>,
    pub mark_idx: TrackedValue<u8>,
    pub core_ip: TrackedValue<IpAddress>,
    pub bpm: TrackedValue<u16>,
}

impl SystemState {
    pub const fn new() -> Self {
        Self {
            cue: TrackedValue::new(Cue::empty()),
            beat_idx: TrackedValue::new(0),
            cue_idx: TrackedValue::new(0),
            mark_idx: TrackedValue::new(255),
            core_ip: TrackedValue::new(IpAddress {
                port: 0,
                addr: [0, 0, 0, 0],
            }),
            bpm: TrackedValue::new(120),
        }
    }
}
