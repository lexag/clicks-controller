#![no_std]
#![no_main]

mod menu;

use cortex_m_rt::entry;
use defmt_rtt as _;
use embedded_hal::digital::v2::{InputPin, OutputPin};
use panic_probe as _;
use rp2040_hal as hal;

use bitflags::bitflags;
use embedded_graphics::{
    image::{Image, ImageRaw},
    pixelcolor::BinaryColor,
};
use embedded_graphics::{prelude::Size, Drawable};
use embedded_menu::{
    interaction::{Action, Interaction, Navigation},
    Menu, SelectValue,
};
use hal::{
    clocks::{init_clocks_and_plls, Clock},
    pac,
    sio::Sio,
    watchdog::Watchdog,
};
use ssd1306::mode::DisplayConfig;

#[unsafe(link_section = ".boot2")]
#[used]
pub static BOOT2: [u8; 256] = rp2040_boot2::BOOT_LOADER_W25Q080;

#[entry]
fn main() -> ! {
    let mut pac = pac::Peripherals::take().unwrap();
    let core = pac::CorePeripherals::take().unwrap();
    let mut watchdog = Watchdog::new(pac.WATCHDOG);
    let sio = Sio::new(pac.SIO);

    let external_xtal_freq_hz = 12_000_000u32;
    let clocks = init_clocks_and_plls(
        external_xtal_freq_hz,
        pac.XOSC,
        pac.CLOCKS,
        pac.PLL_SYS,
        pac.PLL_USB,
        &mut pac.RESETS,
        &mut watchdog,
    )
    .ok()
    .unwrap();

    let mut delay = cortex_m::delay::Delay::new(core.SYST, clocks.system_clock.freq().to_Hz());

    let pins = hal::gpio::Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    let sda_pin: hal::gpio::Pin<_, hal::gpio::FunctionI2C, _> = pins.gpio26.reconfigure();
    let scl_pin: hal::gpio::Pin<_, hal::gpio::FunctionI2C, _> = pins.gpio27.reconfigure();

    let mut i2c = hal::I2C::i2c1(
        pac.I2C1,
        sda_pin,
        scl_pin, // Try `not_an_scl_pin` here
        hal::fugit::Rate::<u32, 1, 1>::kHz(400),
        &mut pac.RESETS,
        &clocks.system_clock,
    );

    let interface = ssd1306::I2CDisplayInterface::new(i2c);
    let mut display = ssd1306::Ssd1306::new(
        interface,
        ssd1306::size::DisplaySize128x64,
        ssd1306::rotation::DisplayRotation::Rotate0,
    )
    .into_buffered_graphics_mode();
    display.init().unwrap();

    let mut pin_led_metr = pins.gpio10.into_push_pull_output().into_dyn_pin();
    let mut pin_led_conn = pins.gpio11.into_push_pull_output().into_dyn_pin();
    let mut pin_led_play = pins.gpio12.into_push_pull_output().into_dyn_pin();
    let mut pin_led_vlt = pins.gpio13.into_push_pull_output().into_dyn_pin();

    let mut button_rows = [
        pins.gpio6.into_push_pull_output().into_dyn_pin(),
        pins.gpio7.into_push_pull_output().into_dyn_pin(),
        pins.gpio8.into_push_pull_output().into_dyn_pin(),
    ];
    for pin in &mut button_rows {
        pin.set_high();
    }
    let mut button_columns = [
        pins.gpio2.into_pull_up_input().into_dyn_pin(),
        pins.gpio3.into_pull_up_input().into_dyn_pin(),
        pins.gpio4.into_pull_up_input().into_dyn_pin(),
        pins.gpio5.into_pull_up_input().into_dyn_pin(),
    ];

    let mut menu = Menu::build("Menu")
        .add_item("Brightness", Brightness::High, |_| 1)
        .add_item("mer test", "!", |_| 2)
        .build();

    let mut menu_visible = false;
    let mut last_buttons = Buttons::empty();

    pin_led_metr.set_high();

    loop {
        let mut redraw = false;
        let buttons = button_scan(&mut button_columns, &mut button_rows);
        match buttons.clone() & !last_buttons {
            Buttons::Menu => {
                if !menu_visible {
                    menu_visible = true;
                } else {
                    menu.interact(Interaction::Action(Action::Select));
                }
                redraw = true;
            }
            Buttons::Back => {
                if menu_visible {
                    menu_visible = false;
                    redraw = true;
                }
            }
            Buttons::Next => {
                if menu_visible {
                    menu.interact(Interaction::Navigation(Navigation::Next));
                    redraw = true;
                } else {
                }
            }
            Buttons::Prev => {
                if menu_visible {
                    menu.interact(Interaction::Navigation(Navigation::Previous));
                    redraw = true;
                } else {
                }
            }
            _ => {}
        }
        last_buttons = buttons;

        if redraw {
            if menu_visible {
                menu.update(&display);
                display.clear_buffer();
                menu.draw(&mut display).unwrap();
                display.flush();
            } else {
                display.clear_buffer();
                display.flush();
            }
        }
    }
}

bitflags! {
    #[derive(PartialEq, Clone)]
    pub struct Buttons: u16 {
    const MetStart = 0x01;
    const MetStop = 0x02;
    const Back = 0x04;
    const Menu = 0x08;
    const MetTPlus = 0x10;
    const MetTMinus = 0x20;
    const MetBPlus = 0x40;
    const MetBMinus = 0x80;
    const Next = 0x100;
    const Prev = 0x200;
    const Stop = 0x400;
    const Start = 0x800;
    }
}

#[derive(Copy, Clone, PartialEq, SelectValue)]
enum Brightness {
    Low,
    Medium,
    High,
}

fn button_scan(
    cols: &mut [hal::gpio::Pin<hal::gpio::DynPinId, hal::gpio::FunctionSioInput, hal::gpio::PullUp>;
             4],
    rows: &mut [hal::gpio::Pin<hal::gpio::DynPinId, hal::gpio::FunctionSioOutput, hal::gpio::PullDown>;
             3],
) -> Buttons {
    let mut buttons = Buttons::empty();
    buttons.set(
        Buttons::MetStart,
        button_down(&mut cols[1], &mut rows[0]).is_ok_and(|b| b),
    );
    buttons.set(
        Buttons::MetStop,
        button_down(&mut cols[0], &mut rows[0]).is_ok_and(|b| b),
    );
    buttons.set(
        Buttons::Back,
        button_down(&mut cols[2], &mut rows[0]).is_ok_and(|b| b),
    );
    buttons.set(
        Buttons::Menu,
        button_down(&mut cols[3], &mut rows[0]).is_ok_and(|b| b),
    );
    buttons.set(
        Buttons::MetTPlus,
        button_down(&mut cols[0], &mut rows[1]).is_ok_and(|b| b),
    );
    buttons.set(
        Buttons::MetTMinus,
        button_down(&mut cols[0], &mut rows[2]).is_ok_and(|b| b),
    );
    buttons.set(
        Buttons::MetBPlus,
        button_down(&mut cols[1], &mut rows[1]).is_ok_and(|b| b),
    );
    buttons.set(
        Buttons::MetBMinus,
        button_down(&mut cols[1], &mut rows[2]).is_ok_and(|b| b),
    );
    buttons.set(
        Buttons::Next,
        button_down(&mut cols[2], &mut rows[1]).is_ok_and(|b| b),
    );
    buttons.set(
        Buttons::Prev,
        button_down(&mut cols[3], &mut rows[1]).is_ok_and(|b| b),
    );
    buttons.set(
        Buttons::Stop,
        button_down(&mut cols[2], &mut rows[2]).is_ok_and(|b| b),
    );
    buttons.set(
        Buttons::Start,
        button_down(&mut cols[3], &mut rows[2]).is_ok_and(|b| b),
    );
    buttons
}

fn button_down(
    col: &mut hal::gpio::Pin<hal::gpio::DynPinId, hal::gpio::FunctionSioInput, hal::gpio::PullUp>,
    row: &mut hal::gpio::Pin<
        hal::gpio::DynPinId,
        hal::gpio::FunctionSioOutput,
        hal::gpio::PullDown,
    >,
) -> Result<bool, core::convert::Infallible> {
    row.set_low();
    let b = col.is_low();
    row.set_high();
    return b;
}
