use common::{cue::Cue, event::EventTable};

#[derive(Clone)]
pub struct SystemState {
    pub cue: Cue,
    pub cue_idx: u16,
    pub beat_idx: u16,
    pub mark_idx: u8,
}

impl SystemState {
    pub const fn new() -> Self {
        Self {
            cue: Cue::empty(),
            beat_idx: 0,
            cue_idx: 0,
            mark_idx: 255,
        }
    }
}
