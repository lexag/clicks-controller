use crate::textentry::TextEntryContext;
use common::mem::str::StaticString;

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
    DebugMessage {
        msg: StaticString<32>,
    }, // Extend as needed
}
