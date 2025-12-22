use common::{
    beat::Beat,
    cue::CueMetadata,
    mem::{network::IpAddress, str::StaticString},
};

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

    pub fn peek(&self) -> T {
        self.value.clone()
    }
}

#[derive(Clone, Default)]
pub struct SystemState {
    pub cue_metadata: CueMetadata,
    pub cue_idx: u16,
    pub beat_idx: u16,
    pub beat: Beat,
    pub mark_label: StaticString<8>,
    pub bpm: u16,
    pub core_ip: IpAddress,
    pub self_ip: IpAddress,
}

impl SystemState {
    pub const fn new() -> Self {
        Self {
            beat: Beat::empty(),
            cue_metadata: CueMetadata::const_default(),
            beat_idx: 0,
            cue_idx: 0,
            mark_label: StaticString::empty(),
            core_ip: IpAddress {
                port: 8081,
                addr: [192, 168, 1, 135],
            },
            self_ip: IpAddress {
                port: 0,
                addr: [0, 0, 0, 0],
            },
            bpm: 120,
        }
    }
}
