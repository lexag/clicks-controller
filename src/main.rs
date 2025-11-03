#![no_std]
#![no_main]

use cortex_m_rt::entry;
use defmt_rtt as _;
use embedded_hal::digital::v2::{InputPin, OutputPin};
use panic_probe as _;
use rp2040_hal as hal;

use hal::{
    clocks::{init_clocks_and_plls, Clock},
    pac,
    sio::Sio,
    watchdog::Watchdog,
};

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

    let mut pin_led_metr = pins.gpio10.into_push_pull_output().into_dyn_pin();
    let mut pin_led_conn = pins.gpio11.into_push_pull_output().into_dyn_pin();
    let mut pin_led_play = pins.gpio12.into_push_pull_output().into_dyn_pin();
    let mut pin_led_vlt = pins.gpio13.into_push_pull_output().into_dyn_pin();

    let mut pin_button_row_1 = pins.gpio6.into_pull_up_input().into_dyn_pin();
    let mut pin_button_row_2 = pins.gpio7.into_pull_up_input().into_dyn_pin();
    let mut pin_button_row_3 = pins.gpio8.into_pull_up_input().into_dyn_pin();

    let mut pin_button_col_1 = pins.gpio2.into_push_pull_output().into_dyn_pin();
    let mut pin_button_col_2 = pins.gpio3.into_push_pull_output().into_dyn_pin();
    let mut pin_button_col_3 = pins.gpio4.into_push_pull_output().into_dyn_pin();
    let mut pin_button_col_4 = pins.gpio5.into_push_pull_output().into_dyn_pin();

    loop {
        if button_down(&mut pin_button_col_3, &pin_button_row_2).expect("") {
            pin_led_vlt.set_high().unwrap();
        } else {
            pin_led_vlt.set_low().unwrap();
        }
        pin_led_conn.set_high().unwrap();
        delay.delay_ms(50);
        pin_led_conn.set_low().unwrap();
        delay.delay_ms(50);
    }
}

enum Button {
    MetStart,
    MetStop,
    Back,
    Menu,
    MetTPlus,
    MetTMinus,
    MetBPlus,
    MetBMinus,
    Next,
    Prev,
    Stop,
    Start,
}

fn button_down(
    col: &mut hal::gpio::Pin<
        hal::gpio::DynPinId,
        hal::gpio::FunctionSioOutput,
        hal::gpio::PullDown,
    >,
    row: &hal::gpio::Pin<hal::gpio::DynPinId, hal::gpio::FunctionSioInput, hal::gpio::PullUp>,
) -> Result<bool, core::convert::Infallible> {
    col.set_high();
    let b = row.is_high();
    col.set_low();
    return b;
}
