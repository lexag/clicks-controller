use crate::ui::{debug, debug_now};
use bitflags::bitflags;
use embassy_rp::{
    i2c::{Async, I2c},
    peripherals::I2C1,
};
use embedded_graphics::{
    image::{Image, ImageRaw},
    mono_font::{
        ascii::{FONT_10X20, FONT_6X10},
        MonoTextStyle, MonoTextStyleBuilder,
    },
    pixelcolor::BinaryColor,
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
    const IpSelect = 0x80;
    const Main = 0x1F;
    }
}

type I2CType = I2c<'static, I2C1, Async>;

pub struct GraphicsController {
    display: ssd1306::Ssd1306<
        I2CInterface<I2CType>,
        DisplaySize128x64,
        BufferedGraphicsMode<DisplaySize128x64>,
    >,
}

pub struct FontData<'a> {
    pub width: u32,
    pub height: u32,
    pub style: MonoTextStyle<'a, BinaryColor>,
}

impl GraphicsController {
    const LOGO_DATA: &[u8; 1024] = include_bytes!("../logo.bin");
    pub const CHAR_LARGE: FontData<'_> = FontData {
        width: 10,
        height: 20,
        style: MonoTextStyleBuilder::new()
            .font(&FONT_10X20)
            .text_color(BinaryColor::On)
            .build(),
    };
    pub const CHAR_SMALL: FontData<'_> = FontData {
        width: 6,
        height: 10,
        style: MonoTextStyleBuilder::new()
            .font(&FONT_6X10)
            .text_color(BinaryColor::On)
            .build(),
    };

    pub const TL_ALIGN: TextStyle = TextStyleBuilder::new()
        .alignment(embedded_graphics::text::Alignment::Left)
        .baseline(embedded_graphics::text::Baseline::Top)
        .build();

    pub const TR_ALIGN: TextStyle = TextStyleBuilder::new()
        .alignment(embedded_graphics::text::Alignment::Right)
        .baseline(embedded_graphics::text::Baseline::Top)
        .build();

    pub fn new(i2c: I2CType) -> Self {
        let interface = ssd1306::I2CDisplayInterface::new(i2c);
        let mut display = ssd1306::Ssd1306::new(
            interface,
            ssd1306::size::DisplaySize128x64,
            ssd1306::rotation::DisplayRotation::Rotate0,
        )
        .into_buffered_graphics_mode();
        display.init().unwrap();
        Self { display }
    }

    pub fn text_strip(
        &mut self,
        s: &str,
        origin: Point,
        font: FontData,
        len: usize,
        align: TextStyle,
    ) {
        self.bounded_clear(origin, Size::new(len as u32 * font.width, font.height));
        Text::with_text_style(s, origin, font.style, align).draw(&mut self.display);
    }

    pub fn bounded_clear(&mut self, origin: Point, size: Size) {
        self.rect(origin, size, Some(BinaryColor::Off), None);
    }

    pub fn bounded_fill(&mut self, origin: Point, size: Size) {
        self.rect(origin, size, Some(BinaryColor::On), None);
    }

    pub fn rect(
        &mut self,
        origin: Point,
        size: Size,
        fill: Option<BinaryColor>,
        stroke: Option<BinaryColor>,
    ) {
        let mut style = PrimitiveStyleBuilder::new();
        if let Some(fill_c) = fill {
            style = style.fill_color(fill_c);
        }
        if let Some(stroke_c) = stroke {
            style = style
                .stroke_color(stroke_c)
                .stroke_width(1)
                .stroke_alignment(embedded_graphics::primitives::StrokeAlignment::Inside);
        }

        if let Err(err) =
            &Rectangle::new(origin, size).draw_styled(&style.build(), &mut self.display)
        {
            let mut buf = [0u8; 128];
            let s = format_no_std::show(&mut buf, format_args!("{:?}", err)).unwrap_or_default();
            debug_now(s);
        }
    }

    pub fn x6_dot(&mut self, origin: Point, width: u32) {
        if !(width < 6 && width % 2 == 0) {
            return;
        }
        self.bounded_clear(origin, Size::new_equal(6));
        self.bounded_fill(
            origin + Size::new_equal((6 - width) / 2),
            Size::new_equal(width),
        );
    }

    pub fn list_item(&mut self, label: &str, value: Option<&str>, origin: Point, highlight: bool) {
        //self.rect(origin, Size::new(120, 14), Some(BinaryColor::On), None);
        //self.rect(
        //    origin + Size::new(1, 1),
        //    Size::new(120, 12),
        //    Some(BinaryColor::Off),
        //    None,
        //);
        if highlight {
            self.text_strip(
                ">",
                origin + Size::new(0, 2),
                Self::CHAR_SMALL,
                1,
                Self::TL_ALIGN,
            );
        }
        self.text_strip(
            label,
            origin + Size::new(10, 2),
            Self::CHAR_SMALL,
            label.len().min(19),
            Self::TL_ALIGN,
        );
        if let Some(right_text) = value {
            self.text_strip(
                right_text,
                origin + Size::new(120, 2),
                Self::CHAR_SMALL,
                right_text.len(),
                Self::TR_ALIGN,
            );
        }
    }

    pub fn logo(&mut self) {
        let raw_image = ImageRaw::<BinaryColor>::new(Self::LOGO_DATA, 128);
        Image::new(&raw_image, Point::zero()).draw(&mut self.display);
    }

    pub fn commit(&mut self) {
        self.display.flush();
    }

    pub fn clear(&mut self) {
        self.display.clear_buffer();
    }

    //pub async fn redraw_screen_element(&mut self, element: ScreenElement) {
    //    let mut state_res = STATE.lock().await;
    //    let state = state_res.as_ref().expect("pls");

    //    if element.is_empty() {
    //        self.display.clear_buffer();
    //        self.display.flush();
    //        return;
    //    }

    //    if element.contains(ScreenElement::IpSelect) && let FSM::IpSelect(step) = state.fsm {
    //            self.display.clear_buffer();
    //            let mut buf = [0u8; 64];
    //            self.draw_area_start = Point::new(0, 0);
    //            let s = format_no_std::show(
    //                &mut buf,
    //                format_args!(
    //                    "{: >3}.{: >3}.{: >3}.{: >3}:{: >5}",
    //                    state.core_ip.addr[0],
    //                    state.core_ip.addr[1],
    //                    state.core_ip.addr[2],
    //                    state.core_ip.addr[3],
    //                    state.core_ip.port
    //                ),
    //            )
    //            .unwrap();
    //            self.small_text(s);
    //            let mut buf = [0x20u8; 64];
    //            buf[match step {
    //                0 => 0,
    //                1 => 1,
    //                2 => 2,
    //                3 => 4,
    //                4 => 5,
    //                5 => 6,
    //                6 => 8,
    //                7 => 9,
    //                8 => 10,
    //                9 => 12,
    //                10 => 13,
    //                11 => 14,
    //                12 => 16,
    //                13 => 17,
    //                14 => 18,
    //                15 => 19,
    //                16 => 20,
    //                _ => 32,
    //            }] = 94;
    //            let s = &str::from_utf8(&buf).expect("pls");
    //            self.draw_area_start = Point::new(0, Self::CHAR_SMALL_HEIGHT as i32 + 5);
    //            self.small_text(s);
    //    }
    //    if element.contains(ScreenElement::Menu) {}
    //    if element.contains(ScreenElement::Logo) {
    //    }
    //    if element.contains(ScreenElement::Bpm) {
    //        self.draw_area_start = Point::new(0, 0);
    //        self.draw_area_size = Size::new(3 * Self::CHAR_SMALL_WIDTH, Self::CHAR_SMALL_HEIGHT);
    //        self.bounded_clear();
    //        let metc = METRONOME_CONTROLLER.lock().await;
    //        let bpm = metc.as_ref().expect("pls").bpm;
    //        let mut buf = [0u8; 8];
    //        let s = format_no_std::show(&mut buf, format_args!("{: >3}", bpm)).unwrap();
    //        self.small_text(s);
    //    }
    //    if element.contains(ScreenElement::Beat) {
    //        self.draw_area_start = Point::new(
    //            4 * Self::CHAR_LARGE_WIDTH as i32,
    //            (Self::CHAR_LARGE_HEIGHT + Self::CHAR_SMALL_HEIGHT + 2) as i32,
    //        );
    //        self.draw_area_size = Size::new(40, 20);
    //        self.bounded_clear();
    //        let mut beat = Beat::empty();
    //        if let Some(beat_res) = state.cue.get_beat(state.beat_idx) {
    //            beat = beat_res;
    //        }

    //        for i in 0..12 {
    //            let xoffs = (i % 4) * 6;
    //            let yoffs = (i / 4) * 6;
    //            let pos = self.draw_area_start + Size::new(xoffs, yoffs);
    //            let current = i + 1 == beat.count as u32;
    //            Rectangle::new(pos, Size::new(4, 4)).draw_styled(
    //                &PrimitiveStyleBuilder::new()
    //                    .fill_color(if current {
    //                        BinaryColor::On
    //                    } else {
    //                        BinaryColor::Off
    //                    })
    //                    .build(),
    //                &mut self.display,
    //            );
    //            self.display
    //                .set_pixel(pos.x as u32 + 1, pos.y as u32 + 1, true);
    //        }
    //    }
    //    if element.contains(ScreenElement::Bar) {
    //        self.draw_area_start = Point::new(
    //            0,
    //            (Self::CHAR_LARGE_HEIGHT + Self::CHAR_SMALL_HEIGHT) as i32,
    //        );
    //        self.draw_area_size = Size::new(4 * Self::CHAR_LARGE_WIDTH, Self::CHAR_LARGE_HEIGHT);
    //        self.bounded_clear();
    //        let mut beat = Beat::empty();
    //        if let Some(beat_res) = state.cue.get_beat(state.beat_idx) {
    //            beat = beat_res;
    //        }
    //        let mut buf = [0u8; 16];
    //        let s = format_no_std::show(&mut buf, format_args!("{: >3}", beat.bar_number)).unwrap();
    //        self.large_text(s);
    //    }
    //    if element.contains(ScreenElement::Mark) {
    //        self.draw_area_start = Point::new(0, Self::CHAR_SMALL_HEIGHT as i32);
    //        self.draw_area_size = Size::new(8 * Self::CHAR_LARGE_WIDTH, Self::CHAR_LARGE_HEIGHT);
    //        self.bounded_clear();
    //        let mark_name = if let Some(EventDescription::RehearsalMarkEvent { label }) = state
    //            .cue
    //            .events
    //            .get(state.mark_idx)
    //            .unwrap_or_default()
    //            .event
    //        {
    //            label
    //        } else {
    //            String8::new("No Mark")
    //        };
    //        let mut buf = [0u8; 8];
    //        let s = format_no_std::show(&mut buf, format_args!("{}", mark_name.str())).unwrap();
    //        self.large_text(s);
    //    }
    //    if element.contains(ScreenElement::Cue) {
    //        self.draw_area_start = Point::new(4 * Self::CHAR_SMALL_WIDTH as i32, 0);
    //        self.draw_area_size = Size::new(16, Self::CHAR_SMALL_HEIGHT);
    //        self.bounded_clear();
    //        let mut buf = [0u8; 64];
    //        let s = format_no_std::show(
    //            &mut buf,
    //            format_args!("{: >2}: {}", state.cue_idx, state.cue.metadata.name.str()),
    //        )
    //        .unwrap();
    //        self.small_text(s);
    //    }
    //    self.display.flush();
    //    //let mut buf_rmark = [0x20u8; 8];
    //    //.unwrap();
    //    //let mut buf_top = [0x20u8; 64];
    //    //let s_top = format_no_std::show(
    //    //    &mut buf_top,
    //    //    format_args!(
    //    //        "{: >3} {: >2} {}",
    //    //        bpm,
    //    //        cue_idx,
    //    //        if core_connection {
    //    //            &str::from_utf8(&cue_name.content).unwrap_or_default()
    //    //        } else {
    //    //            "NC"
    //    //        }
    //    //    ),
    //    //)
    //    //.unwrap();

    //    //display.clear_buffer();
    //    //let bounding_box = display.bounding_box();
    //    //let big_style = MonoTextStyleBuilder::new()
    //    //    .font(&FONT_10X20)
    //    //    .text_color(BinaryColor::On)
    //    //    .build();
    //    //let centered = TextStyleBuilder::new()
    //    //    .alignment(embedded_graphics::text::Alignment::Center)
    //    //    .baseline(embedded_graphics::text::Baseline::Bottom)
    //    //    .build();
    //    //Text::with_text_style(
    //    //    s_rmark,
    //    //    bounding_box.center() + Point::new(0, 14),
    //    //    big_style,
    //    //    centered,
    //    //)
    //    //.draw(&mut display);
    //    //.draw(&mut display);
    //    //Text::with_text_style(s_top, bounding_box.top_left, character_style, top_line)
    //    //    .draw(&mut display);
    //    //display.flush();
    //    //pin_led_vlt.set_high();
    //}
}
