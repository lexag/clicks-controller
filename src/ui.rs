use crate::events::{Action, Mode};
use crate::graphics::GraphicsController;
use crate::menu::{self};
use crate::state::SystemState;
use crate::{ACTION_UPSTREAM, STATE, UI_CH};
use common::mem::str::StaticString;
use embassy_executor::task;
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::{Point, Size};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ViewState {
    mode: Mode,
    selected_index: usize,
    text: StaticString<32>,
}

#[task]
pub async fn ui_task(mut gc: GraphicsController) {
    let mut rx = UI_CH.receiver();

    let mut state = ViewState {
        mode: Mode::Lock,
        selected_index: 0,
        text: StaticString::new("Unused text"),
    };

    let gcm = &mut gc;
    redraw_full(&state, gcm).await;

    loop {
        let action = rx.recv().await;
        let mut need_redraw = false;

        match action {
            Action::NextItem => {
                state.selected_index = state.selected_index.saturating_add(1);
                need_redraw = true;
            }
            Action::PreviousItem => {
                state.selected_index = state.selected_index.saturating_sub(1);
                need_redraw = true;
            }
            Action::SelectItem => {
                let app_state = STATE.lock().await;
                if let Some(action) = (menu::get_item(state.selected_index).exec)(app_state.clone())
                {
                    drop(app_state);
                    ACTION_UPSTREAM.send(action).await;
                }
            }
            Action::ModeChange(m) => {
                state.mode = m;
                redraw_full(&state, gcm).await;
            }
            Action::TextEntryStart { ctx, initial_value } => {
                state.text = initial_value;
                redraw_full(&state, gcm).await;
            }
            Action::TextEntryUpdate { ctx, value } => {
                state.text = value;
                need_redraw = true;
            }
            Action::DebugMessage { msg } => {
                draw_debug_message(gcm, msg);
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
    gc.clear();
    match state.mode {
        Mode::Lock => gc.logo(),
        Mode::Main => {
            draw_main_bpm(gc, &mut app_state);
            draw_main_cue(gc, &mut app_state);
            draw_main_mark(gc, &mut app_state);
            draw_main_bar(gc, &mut app_state);
        }
        Mode::Menu => {
            draw_menu(gc, &mut app_state, state.selected_index);
        }
        Mode::TextEntry => {
            draw_textentry(gc, &mut app_state, state.text);
        }
        _ => {}
    }
    gc.commit();
}

async fn redraw_partial(state: &ViewState, gc: &mut GraphicsController) {
    if state.mode == Mode::Menu {
        redraw_full(state, gc).await
    } else if state.mode == Mode::TextEntry {
        let mut app_state = STATE.lock().await;
        draw_textentry(gc, &mut app_state, state.text);
    }
}

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

fn draw_menu(gc: &mut GraphicsController, app: &mut SystemState, start_idx: usize) {
    const NUM_ITEMS: i32 = 4;
    const MARGIN: i32 = 3;
    const ITEM_HEIGHT: i32 = (64 - (NUM_ITEMS + 1) * MARGIN) / NUM_ITEMS;
    for (i, item) in menu::get_items_following_idx::<4>(start_idx)
        .iter()
        .flatten()
        .enumerate()
    {
        let offset_y = MARGIN + i as i32 * (ITEM_HEIGHT + MARGIN);
        let origin = Point::new(MARGIN, offset_y);
        gc.list_item(
            item.text.str(),
            Some((item.value)(app.clone()).str()),
            origin,
            i == 0,
        );
    }
    gc.commit();
}

fn draw_textentry(gc: &mut GraphicsController, app: &mut SystemState, val: StaticString<32>) {
    const ORIGIN: Point = Point::new(10, 32 - GraphicsController::CHAR_SMALL.height as i32 / 2);
    gc.rect(
        ORIGIN + Size::new(0, GraphicsController::CHAR_SMALL.height + 1),
        Size::new(108, 2),
        Some(BinaryColor::On),
        None,
    );

    gc.clear();

    gc.text_strip(
        "Edit Value",
        ORIGIN - Size::new(0, 20),
        GraphicsController::CHAR_SMALL,
        10,
        GraphicsController::TL_ALIGN,
    );

    gc.text_strip(
        "^",
        ORIGIN
            + Size::new(
                val.len() as u32 * GraphicsController::CHAR_SMALL.width,
                GraphicsController::CHAR_SMALL.height + 2,
            ),
        GraphicsController::CHAR_SMALL,
        10,
        GraphicsController::TL_ALIGN,
    );

    gc.text_strip(
        val.str(),
        ORIGIN,
        GraphicsController::CHAR_SMALL,
        val.len(),
        GraphicsController::TL_ALIGN,
    );

    gc.commit();
}

fn draw_debug_message(gc: &mut GraphicsController, msg: StaticString<32>) {
    gc.clear();
    gc.text_strip(
        msg.str(),
        Point::new(0, 0),
        GraphicsController::CHAR_SMALL,
        msg.len(),
        GraphicsController::TL_ALIGN,
    );

    gc.commit();
}
