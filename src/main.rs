#![no_std]
#![no_main]

mod menu;

use cortex_m_rt::entry;
use defmt_rtt as _;
use embedded_hal::digital::v2::{InputPin, OutputPin};
use panic_probe as _;
use rp2040_hal::{
    self as hal,
    fugit::{MicrosDurationU32, Rate},
    gpio::{
        bank0::{Gpio10, Gpio25},
        DynFunction, DynPinId, DynPullType, Function, FunctionPwm, FunctionSioOutput, PullDown,
        PullNone,
    },
    timer::{Alarm, Alarm0},
};

use bitflags::bitflags;
use core::{
    cell::RefCell,
    sync::atomic::{AtomicBool, Ordering},
};
use cortex_m::interrupt::Mutex;
use embedded_graphics::{
    image::{Image, ImageRaw},
    mono_font::{
        ascii::{FONT_6X10, FONT_8X13},
        MonoTextStyleBuilder,
    },
    pixelcolor::BinaryColor,
    prelude::Dimensions,
    text::{Text, TextStyleBuilder},
};
use embedded_graphics::{prelude::Size, Drawable};
use embedded_menu::{
    interaction::{Action, Interaction, Navigation},
    Menu, SelectValue,
};
use embedded_time::duration::Microseconds;
use hal::{
    clocks::{init_clocks_and_plls, Clock},
    gpio::Pin,
    pac,
    pac::interrupt,
    sio::Sio,
    watchdog::Watchdog,
};
use ssd1306::{
    mode::{BufferedGraphicsMode, DisplayConfig},
    size::DisplaySize128x64,
};

#[unsafe(link_section = ".boot2")]
#[used]
pub static BOOT2: [u8; 256] = rp2040_boot2::BOOT_LOADER_W25Q080;

type MetrLEDPinType = Pin<Gpio10, FunctionSioOutput, PullDown>;

static G_ALARM0: Mutex<RefCell<Option<Alarm0>>> = Mutex::new(RefCell::new(None));
static G_LED_PIN: Mutex<RefCell<Option<MetrLEDPinType>>> = Mutex::new(RefCell::new(None));
static G_IS_LED_HIGH: AtomicBool = AtomicBool::new(false);
static G_TIMER: Mutex<RefCell<Option<hal::Timer>>> = Mutex::new(RefCell::new(None));

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

    let mut timer = rp2040_hal::Timer::new(pac.TIMER, &mut pac.RESETS, &clocks);
    let mut metr_alarm = timer.alarm_0().unwrap();

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

    let mut pin_led_metr: MetrLEDPinType = pins.gpio10.into_push_pull_output();
    let mut pin_led_conn = pins.gpio11.into_push_pull_output().into_dyn_pin();
    let mut pin_led_play = pins.gpio12.into_push_pull_output().into_dyn_pin();
    let mut pin_led_vlt = pins.gpio13.into_push_pull_output().into_dyn_pin();

    pin_led_metr.set_low();

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

    cortex_m::interrupt::free(|cs| {
        G_ALARM0.borrow(cs).replace(Some(metr_alarm));
        G_LED_PIN.borrow(cs).replace(Some(pin_led_metr));
        G_TIMER.borrow(cs).replace(Some(timer));

        let g_led_pin = &mut G_LED_PIN.borrow(cs).borrow_mut();
        if let Some(led_pin) = g_led_pin.as_mut() {
            // set led on for start
            led_pin.set_high().unwrap();
            G_IS_LED_HIGH.store(true, Ordering::Release);
        }
    });

    #[allow(unsafe_code)]
    unsafe {
        pac::NVIC::unmask(pac::Interrupt::TIMER_IRQ_0);
    }

    let mut menu = Menu::build("Menu")
        .add_item("Brightness", Brightness::High, |_| 1)
        .add_item("mer test", "!", |_| 2)
        .build();

    let mut menu_visible = false;
    let mut last_buttons = Buttons::empty();

    let mut bpm: u32 = 120;
    let mut metr_on = false;

    let mut redraw = true;

    loop {
        let buttons = button_scan(&mut button_columns, &mut button_rows);
        let shift = buttons.contains(Buttons::Back);
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
            Buttons::MetTPlus => {
                bpm = bpm.saturating_add(if shift { 10 } else { 1 });
                redraw = true;
            }
            Buttons::MetTMinus => {
                bpm = bpm.saturating_sub(if shift { 10 } else { 1 });
                redraw = true;
            }
            Buttons::MetStart => {
                metr_on = true;
                cortex_m::interrupt::free(|cs| {
                    let g_alarm0 = &mut G_ALARM0.borrow(cs).borrow_mut();
                    if let Some(alarm0) = g_alarm0.as_mut() {
                        alarm0.schedule(MicrosDurationU32::micros(0)).unwrap();
                        alarm0.enable_interrupt();
                    }
                });
            }
            Buttons::MetStop => {
                metr_on = false;
                cortex_m::interrupt::free(|cs| {
                    let g_alarm0 = &mut G_ALARM0.borrow(cs).borrow_mut();
                    if let Some(alarm0) = g_alarm0.as_mut() {
                        alarm0.clear_interrupt();
                    }
                });
            }
            _ => {}
        }
        last_buttons = buttons;

        let mut is_alarm_finished = false;
        cortex_m::interrupt::free(|cs| {
            let g_alarm0 = &mut G_ALARM0.borrow(cs).borrow_mut();
            if let Some(alarm0) = g_alarm0.as_mut() {
                is_alarm_finished = alarm0.finished();
            }
        });

        if is_alarm_finished {
            cortex_m::interrupt::free(|cs| {
                let g_alarm0 = &mut G_ALARM0.borrow(cs).borrow_mut();
                if let Some(alarm0) = g_alarm0.as_mut() && metr_on {
                    const ON_TIME: MicrosDurationU32 = MicrosDurationU32::micros(30000);
                    if G_IS_LED_HIGH.load(Ordering::Acquire) {
                        alarm0.schedule(ON_TIME).unwrap();
                    } else {
                        alarm0
                            .schedule(
                                MicrosDurationU32::from_rate(Rate::<u32, 1, 60>::from_raw(bpm))
                                    - ON_TIME,
                            )
                            .unwrap();
                    }
                    alarm0.enable_interrupt();
                }
            });
        }

        if redraw {
            redraw = false;
            if menu_visible {
                menu.update(&display);
                display.clear_buffer();
                menu.draw(&mut display).unwrap();
                display.flush();
                pin_led_vlt.set_low();
            } else {
                let mut buf = [0x20u8; 21];
                let s = format_no_std::show(&mut buf, format_args!("{:0>3} BPM", bpm)).unwrap();

                display.clear_buffer();
                let bounding_box = display.bounding_box();
                let character_style = MonoTextStyleBuilder::new()
                    .font(&FONT_6X10)
                    .text_color(BinaryColor::On)
                    .build();
                let left_aligned = TextStyleBuilder::new()
                    .alignment(embedded_graphics::text::Alignment::Left)
                    .baseline(embedded_graphics::text::Baseline::Top)
                    .build();
                Text::with_text_style(s, bounding_box.top_left, character_style, left_aligned)
                    .draw(&mut display);
                display.flush();
                pin_led_vlt.set_high();
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

#[interrupt]
fn TIMER_IRQ_0() {
    static mut LED: Option<MetrLEDPinType> = None;

    if LED.is_none() {
        cortex_m::interrupt::free(|cs| {
            *LED = G_LED_PIN.borrow(cs).take();
        });
    }

    // switch led
    if let Some(led) = LED {
        let is_high = G_IS_LED_HIGH.load(Ordering::Acquire);
        if is_high {
            led.set_low().unwrap();
        } else {
            led.set_high().unwrap();
        }
        G_IS_LED_HIGH.store(!is_high, Ordering::Release);
    }

    cortex_m::interrupt::free(|cs| {
        let g_alarm0 = &mut G_ALARM0.borrow(cs).borrow_mut();
        if let Some(alarm0) = g_alarm0.as_mut() {
            alarm0.clear_interrupt();
        }
    });
}
