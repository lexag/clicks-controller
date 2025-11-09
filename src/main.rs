#![no_std]
#![no_main]

mod buttons;
mod graphics;
mod led;
mod menu;
mod metronome;
mod network2;
mod state;

use defmt_rtt as _;
use panic_probe as _;

use crate::{
    buttons::{ButtonController, Buttons},
    graphics::{GraphicsController, ScreenElement},
    led::{LEDController, LED},
    metronome::MetronomeController,
    state::SystemState,
};
use embassy_embedded_hal::shared_bus::asynch::spi::SpiDevice;
use embassy_executor::Spawner;
use embassy_net_wiznet::{chip::W5500, State};
use embassy_rp::{
    bind_interrupts,
    config::Config,
    gpio::{Input, Level, Output, Pull},
    i2c::{self, I2c, InterruptHandler},
    peripherals::{I2C0, I2C1, SPI0},
    spi::{self, Async},
    spinlock_mutex::SpinlockRawMutex,
};
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, mutex::Mutex};
use embassy_time::Timer;
use embedded_menu::SelectValue;

static STATE: Mutex<CriticalSectionRawMutex, Option<SystemState>> =
    Mutex::new(Some(SystemState::new()));
static LED_CONTROLLER: Mutex<CriticalSectionRawMutex, Option<LEDController>> = Mutex::new(None);
static BUTTON_CONTROLLER: Mutex<CriticalSectionRawMutex, Option<ButtonController>> =
    Mutex::new(None);
static METRONOME_CONTROLLER: Mutex<CriticalSectionRawMutex, Option<MetronomeController>> =
    Mutex::new(None);
static GRAPHICS_CONTROLLER: Mutex<CriticalSectionRawMutex, Option<GraphicsController>> =
    Mutex::new(None);

bind_interrupts!(struct Irqs {
    I2C1_IRQ => InterruptHandler<I2C1>;
});

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let mut config = Config::default();
    let mut p = embassy_rp::init(config);

    let mut spi_config = spi::Config::default();
    spi_config.frequency = 20_000_000;

    let sclk = p.PIN_18.reborrow();
    let miso = p.PIN_16.reborrow();
    let mosi = p.PIN_19.reborrow();
    let cs = Output::new(p.PIN_17.reborrow(), Level::High);
    let int = Input::new(p.PIN_21.reborrow(), Pull::Up);
    let rst = Output::new(p.PIN_20.reborrow(), Level::Low);

    let spi_bus = spi::Spi::new(
        p.SPI0.reborrow(),
        sclk,
        mosi,
        miso,
        p.DMA_CH0.reborrow(),
        p.DMA_CH1.reborrow(),
        spi_config,
    );

    let spi_mutex: Mutex<SpinlockRawMutex<1>, spi::Spi<'_, SPI0, Async>> = Mutex::new(spi_bus);

    let spi_device = SpiDevice::new(&spi_mutex, cs);

    let mut wiznet_state: embassy_net_wiznet::State<2, 2> = State::new();
    embassy_net_wiznet::new::<
        2,
        2,
        W5500,
        embassy_embedded_hal::shared_bus::asynch::spi::SpiDevice<
            '_,
            SpinlockRawMutex<1>,
            embassy_rp::spi::Spi<'_, SPI0, embassy_rp::spi::Async>,
            Output<'_>,
        >,
        Input<'_>,
        Output<'_>,
    >([0x0; 6], &mut wiznet_state, spi_device, int, rst);

    let i2c_config = i2c::Config::default();
    let i2c = I2c::new_async(p.I2C1, p.PIN_27, p.PIN_26, Irqs, i2c_config);

    {
        let mut btnc = BUTTON_CONTROLLER.lock().await;
        btnc.replace(ButtonController::new(
            p.PIN_2, p.PIN_3, p.PIN_4, p.PIN_5, p.PIN_6, p.PIN_7, p.PIN_8,
        ));

        let mut ledc = LED_CONTROLLER.lock().await;
        ledc.replace(LEDController::new(p.PIN_10, p.PIN_11, p.PIN_12, p.PIN_13));

        let mut metc = METRONOME_CONTROLLER.lock().await;
        metc.replace(MetronomeController::new());

        let mut gfxc = GRAPHICS_CONTROLLER.lock().await;
        gfxc.replace(GraphicsController::new(i2c));
    }

    redraw(ScreenElement::Logo).await;
    Timer::after_secs(1).await;
    redraw(ScreenElement::empty()).await;
    redraw(ScreenElement::Main).await;

    debug_flash().await;

    spawner.spawn(init_metronome()).unwrap();
    spawner.spawn(init_buttons()).unwrap();

    loop {
        Timer::after_secs(1).await;
    }

    //let mut pin_led_metr: MetrLEDPinType = pins.gpio10.into_push_pull_output();
    //let mut pin_led_conn = pins.gpio11.into_push_pull_output().into_dyn_pin();
    //let mut pin_led_play = pins.gpio12.into_push_pull_output().into_dyn_pin();
    //let mut pin_led_vlt = pins.gpio13.into_push_pull_output().into_dyn_pin();

    //pin_led_metr.set_low();

    //cortex_m::interrupt::free(|cs| {
    //    G_ALARM0.borrow(cs).replace(Some(metr_alarm));
    //    G_LED_PIN.borrow(cs).replace(Some(pin_led_metr));
    //    G_TIMER.borrow(cs).replace(Some(timer));

    //    let g_led_pin = &mut G_LED_PIN.borrow(cs).borrow_mut();
    //    if let Some(led_pin) = g_led_pin.as_mut() {
    //        // set led on for start
    //        led_pin.set_high().unwrap();
    //        G_IS_LED_HIGH.store(true, Ordering::Release);
    //    }
    //});

    //#[allow(unsafe_code)]
    //unsafe {
    //    pac::NVIC::unmask(pac::Interrupt::TIMER_IRQ_0);
    //}

    //

    //let mut menu = Menu::build("Menu")
    //    .add_item("Brightness", Brightness::High, |_| 1)
    //    .add_item("mer test", "!", |_| 2)
    //    .build();

    //let mut menu_visible = false;
    //let mut last_buttons = Buttons::empty();

    //let mut metr_on = false;
}

#[derive(Copy, Clone, PartialEq, SelectValue)]
enum Brightness {
    Low,
    Medium,
    High,
}

//#[interrupt]
//fn TIMER_IRQ_0() {
//    static mut LED: Option<MetrLEDPinType> = None;
//
//    if LED.is_none() {
//        cortex_m::interrupt::free(|cs| {
//            *LED = G_LED_PIN.borrow(cs).take();
//        });
//    }
//
//    // switch led
//    if let Some(led) = LED {
//        let is_high = G_IS_LED_HIGH.load(Ordering::Acquire);
//        if is_high {
//            led.set_low().unwrap();
//        } else {
//            led.set_high().unwrap();
//        }
//        G_IS_LED_HIGH.store(!is_high, Ordering::Release);
//    }
//
//    cortex_m::interrupt::free(|cs| {
//        let g_alarm0 = &mut G_ALARM0.borrow(cs).borrow_mut();
//        if let Some(alarm0) = g_alarm0.as_mut() {
//            alarm0.clear_interrupt();
//        }
//    });
//}
//
//

pub async fn debug_flash() {
    let mut ledc = LED_CONTROLLER.lock().await;
    ledc.as_mut()
        .expect("pls")
        .flash(LED::Connection, 500000)
        .await;
}

const BLINK_TIME_US: u64 = 50000;

#[embassy_executor::task]
pub async fn init_metronome() {
    {
        let mut ledc = LED_CONTROLLER.lock().await;
        ledc.as_mut().expect("pls").set(LED::Connection, true);
    }
    loop {
        let (bpm, enabled) = {
            let mut metc = METRONOME_CONTROLLER.lock().await;
            let metc = metc.as_mut().expect("pls");
            (metc.bpm, metc.enabled)
        };

        if enabled {
            {
                let mut ledc = LED_CONTROLLER.lock().await;
                ledc.as_mut()
                    .expect("pls")
                    .flash(LED::Metronome, BLINK_TIME_US)
                    .await;
            }
            Timer::after_micros(60_000_000 / bpm as u64 - BLINK_TIME_US).await;
        } else {
            Timer::after_micros(1000).await;
        }
    }
}
pub async fn redraw(element: ScreenElement) {
    let mut gfxc = GRAPHICS_CONTROLLER.lock().await;
    gfxc.as_mut()
        .expect("pls")
        .redraw_screen_element(element)
        .await;
}

#[embassy_executor::task]
pub async fn init_buttons() {
    let mut last_buttons = Buttons::empty();
    let mut menu_visible = false;
    loop {
        let buttons = {
            let mut btnc = BUTTON_CONTROLLER.lock().await;
            btnc.as_mut().expect("pls").button_scan()
        };

        let shift = buttons.contains(Buttons::Back);
        match buttons.clone() & !last_buttons {
            Buttons::Menu => {
                if !menu_visible {
                    menu_visible = true;
                } else {
                    //menu.interact(Interaction::Action(Action::Select));
                }
                redraw(ScreenElement::Menu).await;
            }
            Buttons::Back => {
                if menu_visible {
                    menu_visible = false;
                    redraw(ScreenElement::Main).await;
                }
            }
            Buttons::Next => {
                if menu_visible {
                    //menu.interact(Interaction::Navigation(Navigation::Next));
                    redraw(ScreenElement::Menu).await;
                } else {
                    {
                        let mut state = STATE.lock().await;
                        let idx = state.as_ref().expect("pls").cue_idx;
                        state.as_mut().expect("pls").cue_idx = idx.saturating_add(1);
                    }
                    redraw(ScreenElement::Cue).await;
                }
            }
            Buttons::Prev => {
                if menu_visible {
                    //menu.interact(Interaction::Navigation(Navigation::Next));
                    redraw(ScreenElement::Menu).await;
                } else {
                    {
                        let mut state = STATE.lock().await;
                        let idx = state.as_ref().expect("pls").cue_idx;
                        state.as_mut().expect("pls").cue_idx =
                            if shift { 0 } else { idx.saturating_sub(1) };
                    }
                    redraw(ScreenElement::Cue).await;
                }
            }
            Buttons::MetTPlus => {
                {
                    let mut metc = METRONOME_CONTROLLER.lock().await;
                    metc.as_mut()
                        .expect("pls")
                        .change_bpm(if shift { 10 } else { 1 });
                }
                redraw(ScreenElement::Bpm).await;
            }
            Buttons::MetTMinus => {
                {
                    let mut metc = METRONOME_CONTROLLER.lock().await;
                    metc.as_mut()
                        .expect("pls")
                        .change_bpm(if shift { -10 } else { -1 });
                }
                redraw(ScreenElement::Bpm).await;
            }
            Buttons::MetStart => {
                let mut metc = METRONOME_CONTROLLER.lock().await;
                metc.as_mut().expect("pls").enabled = true
            }
            Buttons::MetStop => {
                let mut metc = METRONOME_CONTROLLER.lock().await;
                metc.as_mut().expect("pls").enabled = false
            }
            _ => {}
        }
        last_buttons = buttons;

        Timer::after_micros(50000).await;
    }
}
