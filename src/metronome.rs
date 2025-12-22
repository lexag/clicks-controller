use crate::{events::Action, led::LED, LED_CH, METR_CH, UI_CH};
use embassy_futures::select::{select, Either};
use embassy_time::{Duration, Instant, Timer};

#[embassy_executor::task]
pub async fn metronome_task() {
    let mut bpm = 120;
    let mut last_blip = Instant::now();
    UI_CH.send(Action::NewBPM(bpm as u64)).await;

    loop {
        'stopped: loop {
            let msg = METR_CH.receive().await;
            match msg {
                Action::MetronomeStart => {
                    break 'stopped;
                }
                Action::MetronomeTempoTap => {
                    last_blip = Instant::now();
                    break 'stopped;
                }
                Action::MetronomeAddTempo(t) => {
                    bpm += t;
                    UI_CH.send(Action::NewBPM(bpm as u64)).await;
                }
                Action::MetronomeSetTempo(t) => {
                    bpm = t;
                    UI_CH.send(Action::NewBPM(bpm as u64)).await;
                }
                Action::NewBeatData(data) => bpm = data.tempo() as i64,
                _ => {}
            }
        }

        'running: loop {
            let mut ticker =
                embassy_time::Ticker::every(Duration::from_micros(60000000 / bpm.max(1) as u64));
            'constant_tempo: loop {
                LED_CH.try_send(Action::LEDBlip(LED::Metronome));
                'wait_blip: loop {
                    match select(ticker.next(), METR_CH.receive()).await {
                        Either::First(_) => break 'wait_blip,
                        Either::Second(msg) => match msg {
                            Action::MetronomeStart => {
                                ticker.reset();
                                break 'wait_blip;
                            }
                            Action::MetronomeStop => break 'running,
                            Action::MetronomeTempoTap => {
                                let now = Instant::now();
                                bpm = (bpm + 60000000 / (now - last_blip).as_micros() as i64) / 2;
                                last_blip = now;
                                UI_CH.send(Action::NewBPM(bpm as u64)).await;
                                break 'constant_tempo;
                            }
                            Action::NewTransportData(data) => {
                                if data.running {
                                    break 'running;
                                }
                            }
                            Action::MetronomeAddTempo(t) => {
                                bpm += t;
                                UI_CH.send(Action::NewBPM(bpm as u64)).await;
                                break 'constant_tempo;
                            }
                            Action::MetronomeSetTempo(t) => {
                                bpm = t;
                                UI_CH.send(Action::NewBPM(bpm as u64)).await;
                                break 'constant_tempo;
                            }
                            Action::NewBeatData(data) => {
                                bpm = data.tempo() as i64;
                                break 'constant_tempo;
                            }
                            _ => {}
                        },
                    }
                }
            }
        }
    }
}
