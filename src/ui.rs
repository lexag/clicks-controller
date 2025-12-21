use crate::events::{Action, Mode};
use crate::graphics::GraphicsController;
use crate::state::SystemState;
use crate::{STATE, UI_CH};
use common::event::{Event, EventDescription};
use common::mem::str::StaticString;
use defmt::*;
use embassy_executor::task;
use embedded_graphics::prelude::Point;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ViewState {
    mode: Mode,
    selected_index: u8,
}

#[task]
pub async fn ui_task(mut gc: GraphicsController) {
    let mut rx = UI_CH.receiver();

    let mut state = ViewState {
        mode: Mode::Lock,
        selected_index: 0,
    };

    let gcm = &mut gc;
    redraw_full(&state, gcm).await;

    loop {
        let action = rx.recv().await;
        let mut need_redraw = false;

        match action {
            Action::NextItem => {
                state.selected_index = state.selected_index.wrapping_add(1);
                need_redraw = true;
            }
            Action::PreviousItem => {
                state.selected_index = state.selected_index.wrapping_sub(1);
                need_redraw = true;
            }
            Action::ModeChange(m) => {
                state.mode = m;
                redraw_full(&state, gcm).await;
            }
            _ => {}
        }

        if need_redraw {
            redraw_partial(&state, gcm).await;
        }
    }
}

async fn redraw_full(state: &ViewState, gc: &mut GraphicsController) {
    let mut app_state = STATE.lock().await;
    match state.mode {
        Mode::Lock => gc.logo(),
        Mode::Main => {
            gc.clear();
            draw_main_bpm(gc, &mut app_state);
            draw_main_cue(gc, &mut app_state);
            draw_main_mark(gc, &mut app_state);
            draw_main_bar(gc, &mut app_state);
        }
        _ => {}
    }
    gc.commit();
}

async fn redraw_partial(state: &ViewState, gc: &mut GraphicsController) {}

fn draw_main_bpm(gc: &mut GraphicsController, app: &mut SystemState) {
    let mut buf = [0u8; 3];
    let bpm = app.bpm.read_ref() % 1000;
    let s = format_no_std::show(&mut buf, format_args!("{: >3}", bpm)).unwrap_or_default();
    gc.text_strip(
        s,
        Point::new(0, 0),
        GraphicsController::CHAR_SMALL,
        3,
        GraphicsController::TL_ALIGN,
    );
}

fn draw_main_cue(gc: &mut GraphicsController, app: &mut SystemState) {
    let mut buf = [0u8; 40];
    let cue_idx = app.cue_idx.read_ref();
    let cue = app.cue_metadata.read_ref();
    let s = format_no_std::show(
        &mut buf,
        format_args!("{: >3}:{: <32}", cue_idx, cue.name.str()),
    )
    .unwrap_or_default();
    gc.text_strip(
        s,
        Point::new(24, 0),
        GraphicsController::CHAR_SMALL,
        16,
        GraphicsController::TL_ALIGN,
    );
}

fn draw_main_mark(gc: &mut GraphicsController, app: &mut SystemState) {
    let mark_idx = app.mark_label.read();
    gc.text_strip(
        mark_idx.str(),
        Point::new(0, 11),
        GraphicsController::CHAR_LARGE,
        8,
        GraphicsController::TL_ALIGN,
    );
}

fn draw_main_bar(gc: &mut GraphicsController, app: &mut SystemState) {
    let mut buf = [0u8; 4];
    let s = format_no_std::show(&mut buf, format_args!("{: >4}", app.beat.read().bar_number))
        .unwrap_or_default();
    gc.text_strip(
        s,
        Point::new(0, 33),
        GraphicsController::CHAR_LARGE,
        4,
        GraphicsController::TL_ALIGN,
    );
}
