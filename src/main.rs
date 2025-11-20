#![no_std]
#![no_main]

mod buttons;
mod events;
mod fsm;
mod graphics;
mod led;
mod menu;
mod metronome;
mod network2;
mod state;
mod textentry;
mod translator;
mod ui;

use defmt_rtt as _;
use panic_probe as _;

use crate::{
    buttons::{button_scanner_task, ButtonScanner},
    events::{Action, ButtonEvent, Mode},
    fsm::FSM,
    graphics::{GraphicsController, ScreenElement},
    led::{LEDController, LED},
    metronome::MetronomeController,
    state::SystemState,
    translator::input_translator_task,
    ui::ui_task,
};
use embassy_embedded_hal::shared_bus::asynch::spi::SpiDevice;
use embassy_executor::Spawner;
use embassy_net_wiznet::{chip::W5500, State};
use embassy_rp::{
    bind_interrupts,
    config::Config,
    gpio::{Input, Level, Output, Pull},
    i2c::{self, I2c, InterruptHandler},
    peripherals::{I2C1, SPI0},
    spi::{self, Async},
    spinlock_mutex::SpinlockRawMutex,
};
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, mutex::Mutex};
use embassy_time::Timer;

use embassy_sync::channel::Channel;
use embassy_sync::signal::Signal;

// Button events from scanner → translator
pub static BUTTON_CH: Channel<CriticalSectionRawMutex, ButtonEvent, 8> = Channel::new();

// Translator publishes actions here
pub static ACTION_SRC: Channel<CriticalSectionRawMutex, Action, 8> = Channel::new();

// Fan‑out destinations (subscribers)
pub static CONTROL_CH: Channel<CriticalSectionRawMutex, Action, 8> = Channel::new();
pub static UI_CH: Channel<CriticalSectionRawMutex, Action, 8> = Channel::new();
pub static UX_CH: Channel<CriticalSectionRawMutex, Action, 8> = Channel::new();
pub static LED_CH: Channel<CriticalSectionRawMutex, Action, 8> = Channel::new();

// Signal for latest mode
pub static MODE_SIGNAL: Signal<CriticalSectionRawMutex, Mode> = Signal::new();

static STATE: Mutex<CriticalSectionRawMutex, SystemState> = Mutex::new(SystemState::new());

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

    //let spi_mutex: Mutex<SpinlockRawMutex<1>, spi::Spi<'_, SPI0, Async>> = Mutex::new(spi_bus);

    //let spi_device = SpiDevice::new(&spi_mutex, cs);

    //let mut wiznet_state: embassy_net_wiznet::State<2, 2> = State::new();
    //embassy_net_wiznet::new::<
    //    2,
    //    2,
    //    W5500,
    //    embassy_embedded_hal::shared_bus::asynch::spi::SpiDevice<
    //        '_,
    //        SpinlockRawMutex<1>,
    //        embassy_rp::spi::Spi<'_, SPI0, embassy_rp::spi::Async>,
    //        Output<'_>,
    //    >,
    //    Input<'_>,
    //    Output<'_>,
    //>([0x0; 6], &mut wiznet_state, spi_device, int, rst);

    //let mut ledc = LED_CONTROLLER.lock().await;
    //ledc.replace(LEDController::new(p.PIN_10, p.PIN_11, p.PIN_12, p.PIN_13));

    //let mut metc = METRONOME_CONTROLLER.lock().await;
    //metc.replace(MetronomeController::new());

    MODE_SIGNAL.signal(Mode::Lock);
    spawner
        .spawn(button_scanner_task(ButtonScanner::new(
            p.PIN_2, p.PIN_3, p.PIN_4, p.PIN_5, p.PIN_6, p.PIN_7, p.PIN_8,
        )))
        .unwrap();
    spawner.spawn(input_translator_task()).unwrap();
    spawner.spawn(action_fanout_task()).unwrap();

    let i2c_config = i2c::Config::default();
    let i2c = I2c::new_async(p.I2C1, p.PIN_27, p.PIN_26, Irqs, i2c_config);
    spawner
        .spawn(ui_task(GraphicsController::new(i2c)))
        .unwrap();

    ACTION_SRC.send(Action::ModeChange(Mode::Lock)).await;
    Timer::after_secs(1).await;
    ACTION_SRC.send(Action::ModeChange(Mode::Main)).await;

    let mut o = Output::new(p.PIN_10.reborrow(), Level::Low);
    loop {
        o.toggle();
        Timer::after_millis(500).await;
    }
}

const BLINK_TIME_US: u64 = 50000;

//#[embassy_executor::task]
//pub async fn init_metronome() {
//    {
//        let mut ledc = LED_CONTROLLER.lock().await;
//        ledc.as_mut().expect("pls").set(LED::Connection, true);
//    }
//    loop {
//        let (bpm, enabled) = {
//            let mut metc = METRONOME_CONTROLLER.lock().await;
//            let metc = metc.as_mut().expect("pls");
//            (metc.bpm, metc.enabled)
//        };
//
//        if enabled {
//            {
//                let mut ledc = LED_CONTROLLER.lock().await;
//                ledc.as_mut()
//                    .expect("pls")
//                    .flash(LED::Metronome, BLINK_TIME_US)
//                    .await;
//            }
//            Timer::after_micros(60_000_000 / bpm as u64 - BLINK_TIME_US).await;
//        } else {
//            Timer::after_micros(1000).await;
//        }
//    }
//}

//#[embassy_executor::task]
//pub async fn init_buttons() {
//    let mut last_buttons = Buttons::empty();
//    loop {
//        let buttons = {
//            let mut btnc = BUTTON_CONTROLLER.lock().await;
//            btnc.as_mut().expect("pls").button_scan()
//        };
//        let mut fsm = {
//            let state = STATE.lock().await;
//            state.as_ref().expect("pls").fsm.clone()
//        };
//
//        let shift = buttons.contains(Buttons::Back);
//        let button_change = buttons.clone() & !last_buttons;
//
//        match fsm {
//            FSM::Main => match button_change {
//                Buttons::Menu => {
//                    fsm = FSM::IpSelect(0);
//                }
//                Buttons::Next => {
//                    let mut state = STATE.lock().await;
//                    let idx = state.as_ref().expect("pls").cue_idx;
//                    state.as_mut().expect("pls").cue_idx = idx.saturating_add(1);
//                }
//                Buttons::Prev => {
//                    let mut state = STATE.lock().await;
//                    let idx = state.as_ref().expect("pls").cue_idx;
//                    state.as_mut().expect("pls").cue_idx =
//                        if shift { 0 } else { idx.saturating_sub(1) };
//                }
//                Buttons::MetTPlus => {
//                    let mut metc = METRONOME_CONTROLLER.lock().await;
//                    metc.as_mut()
//                        .expect("pls")
//                        .change_bpm(if shift { 10 } else { 1 });
//                }
//                Buttons::MetTMinus => {
//                    let mut metc = METRONOME_CONTROLLER.lock().await;
//                    metc.as_mut()
//                        .expect("pls")
//                        .change_bpm(if shift { -10 } else { -1 });
//                }
//                Buttons::MetStart => {
//                    let mut metc = METRONOME_CONTROLLER.lock().await;
//                    metc.as_mut().expect("pls").enabled = true
//                }
//                Buttons::MetStop => {
//                    let mut metc = METRONOME_CONTROLLER.lock().await;
//                    metc.as_mut().expect("pls").enabled = false
//                }
//                _ => {}
//            },
//            FSM::Menu => match button_change {
//                Buttons::Menu => {}
//                Buttons::Back => {
//                    fsm = FSM::Main;
//                }
//                Buttons::Next => {}
//                Buttons::Prev => {}
//                _ => {}
//            },
//            FSM::IpSelect(step) => match button_change {
//                Buttons::Start => {
//                    fsm = if step == 3 {
//                        FSM::PortSelect
//                    } else {
//                        let mut state = STATE.lock().await;
//                        state.as_mut().expect("pls").core_ip.addr[step + 1] = 0;
//                        FSM::IpSelect(step + 1)
//                    };
//                }
//                Buttons::Stop => {
//                    fsm = if step == 0 {
//                        FSM::Main
//                    } else {
//                        let mut state = STATE.lock().await;
//                        if state.as_ref().expect("pls").core_ip.addr[step] != 0 {
//                            state.as_mut().expect("pls").core_ip.addr[step] = 0;
//                        }
//                        FSM::IpSelect(step - 1)
//                    }
//                }
//                Buttons::MetStop
//                | Buttons::MetStart
//                | Buttons::Back
//                | Buttons::Menu
//                | Buttons::MetTPlus
//                | Buttons::MetBPlus
//                | Buttons::MetTMinus
//                | Buttons::MetBMinus
//                | Buttons::Prev
//                | Buttons::Next => {
//                    let numerical = ButtonScanner::numerical(button_change.clone());
//
//                    {
//                        let mut state = STATE.lock().await;
//                        let octet = state.as_ref().expect("pls").core_ip.addr[step];
//                        state.as_mut().expect("pls").core_ip.addr[step] =
//                            octet.saturating_mul(10).saturating_add(numerical as u8);
//                    };
//                }
//                _ => {}
//            },
//            FSM::PortSelect => {}
//        }
//
//        {
//            let mut state = STATE.lock().await;
//            state.as_mut().expect("pls").fsm = fsm.clone();
//        }
//
//        redraw(fsm).await;
//
//        last_buttons = buttons;
//
//        Timer::after_micros(50000).await;
//    }
//}

#[embassy_executor::task]
pub async fn action_fanout_task() {
    let mut rx = ACTION_SRC.receiver();
    let tx_playback = CONTROL_CH.sender();
    let tx_ui = UI_CH.sender();
    let tx_ux = UX_CH.sender();
    let tx_led = LED_CH.sender();

    loop {
        let action = rx.receive().await;

        // Non‑blocking sends so fanout isn't held up by slow consumer
        let _ = tx_playback.try_send(action);
        let _ = tx_ui.try_send(action);
        let _ = tx_ux.try_send(action);
        let _ = tx_led.try_send(action);
    }
}
