use crate::{METRONOME_CONTROLLER, STATE};
use bitflags::bitflags;
use common::{beat::Beat, event::EventDescription, mem::str::String8};
use embassy_rp::{
    i2c::{Async, I2c},
    peripherals::{I2C0, I2C1},
};
use embedded_graphics::{
    framebuffer::{buffer_size, Framebuffer},
    image::{Image, ImageRaw},
    mono_font::{
        ascii::{FONT_10X20, FONT_6X10},
        MonoTextStyle, MonoTextStyleBuilder,
    },
    pixelcolor::{
        raw::{LittleEndian, RawU1},
        BinaryColor,
    },
    prelude::Point,
    primitives::{PrimitiveStyleBuilder, Rectangle, StyledDrawable},
    text::{Text, TextStyle, TextStyleBuilder},
};
use embedded_graphics::{prelude::Size, Drawable};
use ssd1306::prelude::I2CInterface;
use ssd1306::{
    mode::{BufferedGraphicsMode, DisplayConfig},
    size::DisplaySize128x64,
};

bitflags! {
    #[derive(PartialEq, Clone)]
    pub struct ScreenElement: u16 {
    const Bpm  = 0x01;
    const Cue  = 0x02;
    const Mark = 0x04;
    const Bar  = 0x08;
    const Beat = 0x10;
    const Menu = 0x20;
    const Logo = 0x40;
    const Main = 0x1F;
    }
}

type I2CType<'a> = I2c<'a, I2C1, Async>;

pub struct GraphicsController<'a> {
    display: ssd1306::Ssd1306<
        I2CInterface<I2CType<'a>>,
        DisplaySize128x64,
        BufferedGraphicsMode<DisplaySize128x64>,
    >,
    draw_area_start: Point,
    draw_area_size: Size,
}

impl GraphicsController<'static> {
    const LOGO_DATA: &[u8; 1024] = include_bytes!("../logo.bin");
    const CHAR_SMALL_WIDTH: u32 = 6;
    const CHAR_SMALL_HEIGHT: u32 = 10;
    const CHAR_LARGE_WIDTH: u32 = 10;
    const CHAR_LARGE_HEIGHT: u32 = 20;

    const SMALL_STYLE: MonoTextStyle<'_, BinaryColor> = MonoTextStyleBuilder::new()
        .font(&FONT_6X10)
        .text_color(BinaryColor::On)
        .build();
    const LARGE_STYLE: MonoTextStyle<'_, BinaryColor> = MonoTextStyleBuilder::new()
        .font(&FONT_10X20)
        .text_color(BinaryColor::On)
        .build();

    const TL_ALIGN: TextStyle = TextStyleBuilder::new()
        .alignment(embedded_graphics::text::Alignment::Left)
        .baseline(embedded_graphics::text::Baseline::Top)
        .build();

    pub fn new(i2c: I2CType<'static>) -> Self {
        let interface = ssd1306::I2CDisplayInterface::new(i2c);
        let mut display = ssd1306::Ssd1306::new(
            interface,
            ssd1306::size::DisplaySize128x64,
            ssd1306::rotation::DisplayRotation::Rotate0,
        )
        .into_buffered_graphics_mode();
        display.init().unwrap();
        Self {
            display,
            draw_area_size: (0, 0).into(),
            draw_area_start: (0, 0).into(),
        }
    }

    fn lower_right(&self) -> Point {
        self.draw_area_start + self.draw_area_size
    }

    fn bounded_clear(&mut self) {
        //let tl_idx = (self.draw_area_start.1 * 128 + self.draw_area_start.0) / 8;
        //let br_idx = tl_idx + (self.draw_area_size.1 * 128 + self.draw_area_size.0) / 8;

        &Rectangle::new(self.draw_area_start, self.draw_area_size).draw_styled(
            &PrimitiveStyleBuilder::new()
                .fill_color(BinaryColor::Off)
                .build(),
            &mut self.display,
        );
        //self.frame_buf.data_mut()[tl_idx as usize..br_idx as usize]
        //    .copy_from_slice(&[0x00; 1024][0..(br_idx as usize - tl_idx as usize)]);
    }

    fn small_text(&mut self, s: &str) {
        Text::with_text_style(s, self.draw_area_start, Self::SMALL_STYLE, Self::TL_ALIGN)
            .draw(&mut self.display);
    }

    fn large_text(&mut self, s: &str) {
        Text::with_text_style(s, self.draw_area_start, Self::LARGE_STYLE, Self::TL_ALIGN)
            .draw(&mut self.display);
    }

    pub async fn redraw_screen_element(&mut self, element: ScreenElement) {
        let mut state_res = STATE.lock().await;
        let state = state_res.as_ref().expect("pls");

        if element.is_empty() {
            self.display.clear_buffer();
            self.display.flush();
            return;
        }
        if element.contains(ScreenElement::Menu) {}
        if element.contains(ScreenElement::Logo) {
            let raw_image = ImageRaw::<BinaryColor>::new(Self::LOGO_DATA, 128);
            Image::new(&raw_image, Point::zero()).draw(&mut self.display);
        }
        if element.contains(ScreenElement::Bpm) {
            self.draw_area_start = Point::new(0, 0);
            self.draw_area_size = Size::new(3 * Self::CHAR_SMALL_WIDTH, Self::CHAR_SMALL_HEIGHT);
            self.bounded_clear();
            let metc = METRONOME_CONTROLLER.lock().await;
            let bpm = metc.as_ref().expect("pls").bpm;
            let mut buf = [0u8; 8];
            let s = format_no_std::show(&mut buf, format_args!("{: >3}", bpm)).unwrap();
            self.small_text(s);
        }
        if element.contains(ScreenElement::Beat) {
            self.draw_area_start = Point::new(
                4 * Self::CHAR_LARGE_WIDTH as i32,
                (Self::CHAR_LARGE_HEIGHT + Self::CHAR_SMALL_HEIGHT + 2) as i32,
            );
            self.draw_area_size = Size::new(40, 20);
            self.bounded_clear();
            let mut beat = Beat::empty();
            if let Some(beat_res) = state.cue.get_beat(state.beat_idx) {
                beat = beat_res;
            }

            for i in 0..12 {
                let xoffs = (i % 4) * 6;
                let yoffs = (i / 4) * 6;
                let pos = self.draw_area_start + Size::new(xoffs, yoffs);
                let current = i + 1 == beat.count as u32;
                Rectangle::new(pos, Size::new(4, 4)).draw_styled(
                    &PrimitiveStyleBuilder::new()
                        .fill_color(if current {
                            BinaryColor::On
                        } else {
                            BinaryColor::Off
                        })
                        .build(),
                    &mut self.display,
                );
                self.display
                    .set_pixel(pos.x as u32 + 1, pos.y as u32 + 1, true);
            }
        }
        if element.contains(ScreenElement::Bar) {
            self.draw_area_start = Point::new(
                0,
                (Self::CHAR_LARGE_HEIGHT + Self::CHAR_SMALL_HEIGHT) as i32,
            );
            self.draw_area_size = Size::new(4 * Self::CHAR_LARGE_WIDTH, Self::CHAR_LARGE_HEIGHT);
            self.bounded_clear();
            let mut beat = Beat::empty();
            if let Some(beat_res) = state.cue.get_beat(state.beat_idx) {
                beat = beat_res;
            }
            let mut buf = [0u8; 16];
            let s = format_no_std::show(&mut buf, format_args!("{: >3}", beat.bar_number)).unwrap();
            self.large_text(s);
        }
        if element.contains(ScreenElement::Mark) {
            self.draw_area_start = Point::new(0, Self::CHAR_SMALL_HEIGHT as i32);
            self.draw_area_size = Size::new(8 * Self::CHAR_LARGE_WIDTH, Self::CHAR_LARGE_HEIGHT);
            self.bounded_clear();
            let mark_name = if let Some(EventDescription::RehearsalMarkEvent { label }) = state
                .cue
                .events
                .get(state.mark_idx)
                .unwrap_or_default()
                .event
            {
                label
            } else {
                String8::new("No Mark")
            };
            let mut buf = [0u8; 8];
            let s = format_no_std::show(&mut buf, format_args!("{}", mark_name.str())).unwrap();
            self.large_text(s);
        }
        if element.contains(ScreenElement::Cue) {
            self.draw_area_start = Point::new(4 * Self::CHAR_SMALL_WIDTH as i32, 0);
            self.draw_area_size = Size::new(16, Self::CHAR_SMALL_HEIGHT);
            self.bounded_clear();
            let mut buf = [0u8; 64];
            let s = format_no_std::show(
                &mut buf,
                format_args!("{: >2}: {}", state.cue_idx, state.cue.metadata.name.str()),
            )
            .unwrap();
            self.small_text(s);
        }
        self.display.flush();
        //let mut buf_rmark = [0x20u8; 8];
        //.unwrap();
        //let mut buf_top = [0x20u8; 64];
        //let s_top = format_no_std::show(
        //    &mut buf_top,
        //    format_args!(
        //        "{: >3} {: >2} {}",
        //        bpm,
        //        cue_idx,
        //        if core_connection {
        //            &str::from_utf8(&cue_name.content).unwrap_or_default()
        //        } else {
        //            "NC"
        //        }
        //    ),
        //)
        //.unwrap();

        //display.clear_buffer();
        //let bounding_box = display.bounding_box();
        //let big_style = MonoTextStyleBuilder::new()
        //    .font(&FONT_10X20)
        //    .text_color(BinaryColor::On)
        //    .build();
        //let centered = TextStyleBuilder::new()
        //    .alignment(embedded_graphics::text::Alignment::Center)
        //    .baseline(embedded_graphics::text::Baseline::Bottom)
        //    .build();
        //Text::with_text_style(
        //    s_rmark,
        //    bounding_box.center() + Point::new(0, 14),
        //    big_style,
        //    centered,
        //)
        //.draw(&mut display);
        //.draw(&mut display);
        //Text::with_text_style(s_top, bounding_box.top_left, character_style, top_line)
        //    .draw(&mut display);
        //display.flush();
        //pin_led_vlt.set_high();
    }
}
