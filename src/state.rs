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
    pub cue_metadata: TrackedValue<CueMetadata>,
    pub cue_idx: TrackedValue<u16>,
    pub beat_idx: TrackedValue<u16>,
    pub beat: TrackedValue<Beat>,
    pub mark_label: TrackedValue<StaticString<8>>,
    pub bpm: TrackedValue<u16>,
    pub core_ip: IpAddress,
}

impl SystemState {
    pub const fn new() -> Self {
        Self {
            beat: TrackedValue::new(Beat::empty()),
            cue_metadata: TrackedValue::new(CueMetadata::const_default()),
            beat_idx: TrackedValue::new(0),
            cue_idx: TrackedValue::new(0),
            mark_label: TrackedValue::new(StaticString::empty()),
            core_ip: IpAddress {
                port: 0,
                addr: [0, 0, 0, 0],
            },
            bpm: TrackedValue::new(120),
        }
    }
}
