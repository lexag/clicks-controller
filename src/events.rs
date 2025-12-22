use crate::{led::LED, textentry::TextEntryContext};
use common::{
    beat::Beat,
    cue::CueMetadata,
    local::status::TransportState,
    mem::str::StaticString,
    protocol::{message::SmallMessage, request::Request},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ButtonId {
    MetronomeStart,
    MetronomeStop,
    Shift,
    Menu,
    MetronomeTempoPlus,
    MetronomeTempoMinus,
    MetronomeBrightPlus,
    MetronomeBrightMinus,
    Next,
    Previous,
    Stop,
    Start,
}

#[derive(Clone, Copy, Debug)]
pub struct ButtonEvent {
    pub id: ButtonId,
    pub pressed: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    TextEntry,
    Menu,
    Main,
    Lock,
}

#[derive(Clone, Copy)]
pub enum Action {
    NextItem,
    PreviousItem,
    SelectItem,
    Character(u8),
    NextCue,
    PreviousCue,
    ModeChange(Mode),
    TextEntryStart {
        ctx: TextEntryContext,
        initial_value: StaticString<32>,
    },
    TextEntryUpdate {
        ctx: TextEntryContext,
        value: StaticString<32>,
    },
    TextEntryComplete {
        ctx: TextEntryContext,
        value: StaticString<32>,
    },
    Confirm,
    Backspace,
    SeekCheckpoint,
    ForceRedraw,
    NewBeatData(Beat),
    NewBPM(u64),
    NewCueData(u16, CueMetadata),
    NewTransportData(TransportState),
    NewLabelData(StaticString<8>),
    DebugMessage {
        msg: StaticString<32>,
    },
    ReloadConnection,
    GainConnection,
    LoseConnection,
    MessageFromCore(SmallMessage),
    RequestToCore(Request),
    LEDSet(LED, bool),
    LEDToggle(LED),
    LEDBlip(LED),
    MetronomeAddTempo(i64),
    MetronomeSetTempo(i64),
    MetronomeStop,
    MetronomeStart,
    MetronomeTempoTap,
}
