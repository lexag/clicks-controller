#![no_std]
#![no_main]

mod buttons;
mod events;
mod fsm;
mod graphics;
mod led;
mod menu;
mod metronome;
//mod network;
mod network2;
//mod spicks;
mod state;
mod textentry;
mod translator;
mod ui;

use defmt_rtt as _;
use panic_probe as _;

use alloc_cortex_m::CortexMHeap;

#[global_allocator]
static ALLOCATOR: CortexMHeap = CortexMHeap::empty();

use crate::{
    buttons::{button_scanner_task, ButtonScanner},
    events::{Action, ButtonEvent, Mode},
    graphics::GraphicsController,
    state::SystemState,
    textentry::text_entry_task,
    translator::input_translator_task,
    ui::ui_task,
};
use embassy_executor::Spawner;
use embassy_rp::{
    bind_interrupts,
    clocks::RoscRng,
    config::Config,
    gpio::{Input, Level, Output, Pull},
    i2c::{self, I2c, InterruptHandler},
    peripherals::{I2C1, PIO0},
    pio,
    spi::{self, Async, Spi},
};
use embassy_sync::{
    blocking_mutex::{
        raw::{CriticalSectionRawMutex, NoopRawMutex},
        CriticalSectionMutex, NoopMutex,
    },
    mutex::Mutex,
};
//use embassy_time::{Delay, Timer};

use embassy_sync::channel::Channel;
use embassy_sync::signal::Signal;
//use embedded_hal_bus::spi::ExclusiveDevice;
use common::mem::str::StaticString;
use embassy_time::Timer;
use static_cell::StaticCell;

use embassy_net::{Stack, StackResources};
use embassy_net_wiznet::chip::W5500;
use embassy_net_wiznet::*;

// Button events from scanner → translator
pub static BUTTON_CH: Channel<CriticalSectionRawMutex, ButtonEvent, 8> = Channel::new();

// Translator publishes actions here
pub static ACTION_SRC: Channel<CriticalSectionRawMutex, Action, 8> = Channel::new();
// Everyone else publishes actions here
pub static ACTION_UPSTREAM: Channel<CriticalSectionRawMutex, Action, 8> = Channel::new();

// Fan‑out destinations (subscribers)
pub static CONTROL_CH: Channel<CriticalSectionRawMutex, Action, 8> = Channel::new();
pub static UI_CH: Channel<CriticalSectionRawMutex, Action, 8> = Channel::new();
pub static UX_CH: Channel<CriticalSectionRawMutex, Action, 8> = Channel::new();
pub static LED_CH: Channel<CriticalSectionRawMutex, Action, 8> = Channel::new();

// Signal for latest mode
pub static MODE_SIGNAL: Signal<CriticalSectionRawMutex, Mode> = Signal::new();

static STATE: Mutex<CriticalSectionRawMutex, SystemState> = Mutex::new(SystemState::new());

static SPI_MUTEX: StaticCell<Mutex<CriticalSectionRawMutex, network2::SpiType>> = StaticCell::new();

bind_interrupts!(struct Irqs {
    I2C1_IRQ => InterruptHandler<I2C1>;
    PIO0_IRQ_0 => pio::InterruptHandler<PIO0>;
});

#[embassy_executor::main]
async fn main(spawner: Spawner) -> () {
    // Init heap on global_allocator
    #[allow(static_mut_refs)]
    unsafe {
        const HEAP_SIZE: usize = 1024 * 8;
        static mut HEAP_MEM: [u8; HEAP_SIZE] = [0; HEAP_SIZE];
        ALLOCATOR.init(HEAP_MEM.as_ptr() as usize, HEAP_SIZE)
    };

    let mut config = Config::default();
    let mut p = embassy_rp::init(config);

    //let mut spi_config = spi::Config::default();
    let mut rng = RoscRng;
    let mut led = Output::new(p.PIN_19, Level::Low);

    // The W55RP20 uses a PIO unit for SPI communication, once the SPI bus has been formed using a
    // PIO statemachine everything else is generally unchanged from the other examples that use the W5500
    let mosi = p.PIN_23;
    let miso = p.PIN_22;
    let clk = p.PIN_21;

    let pio::Pio {
        mut common, sm0, ..
    } = pio::Pio::new(p.PIO0, Irqs);

    // Construct an SPI driver backed by a PIO state machine
    let mut spi_cfg = spi::Config::default();
    spi_cfg.frequency = 12_500_000; // The PIO SPI program is much less stable than the actual SPI
                                    // peripheral, use higher speeds at your peril
    let spi = embassy_rp::pio_programs::spi::Spi::new(
        &mut common,
        sm0,
        clk,
        mosi,
        miso,
        p.DMA_CH0,
        p.DMA_CH1,
        spi_cfg,
    );

    // Further control pins
    let cs = Output::new(p.PIN_20, Level::High);
    let w5500_int = Input::new(p.PIN_24, Pull::Up);
    let w5500_reset = Output::new(p.PIN_25, Level::High);

    let mac_addr = [0x02, 0x00, 0x00, 0x00, 0x00, 0x00];
    static NET_STATE: StaticCell<State<8, 8>> = StaticCell::new();
    let state = NET_STATE.init(State::<8, 8>::new());

    let spimutex: &'static Mutex<CriticalSectionRawMutex, network2::SpiType> =
        SPI_MUTEX.init(Mutex::new(spi));

    let spidev = embassy_embedded_hal::shared_bus::asynch::spi::SpiDevice::new(spimutex, cs);

    let (device, w55_runner) = embassy_net_wiznet::new::<
        8,
        8,
        W5500,
        network2::SpiBusType,
        Input<'static>,
        Output<'static>,
    >(mac_addr, state, spidev, w5500_int, w5500_reset)
    .await
    .unwrap();

    // Generate random seed
    let seed = rng.next_u64();

    let _ = spawner.spawn(network2::ethernet_task(w55_runner));

    // Init network stack
    static RESOURCES: StaticCell<StackResources<3>> = StaticCell::new();
    let (stack, netstack_runner) = embassy_net::new(
        device,
        embassy_net::Config::dhcpv4(Default::default()),
        RESOURCES.init(StackResources::new()),
        seed,
    );

    // Launch network task
    let _ = spawner.spawn(network2::net_task(netstack_runner));

    // Launch network handler task
    let _ = spawner.spawn(network2::stack_task(stack));

    //spi_config.frequency = 20_000_000;

    //let sclk = p.PIN_18;
    //let miso = p.PIN_16;
    //let mosi = p.PIN_19;
    //let cs = Output::new(p.PIN_17, Level::High);
    //let int = Input::new(p.PIN_21, Pull::Up);
    //let rst = Output::new(p.PIN_20, Level::Low);

    //let spi = spi::Spi::new_blocking(p.SPI0, sclk, mosi, miso, spi_config);

    //static SPI_BUS: StaticCell<Mutex<NoopRawMutex, Spi<'static, SPI0, Async>>> = StaticCell::new();

    //let spi_bus = SPI_BUS.init(Mutex::new(spi));
    //let spi_dev = embassy_embedded_hal::shared_bus::asynch::spi::SpiDevice::new(spi_bus, cs);
    //let spi_mutex: Mutex<SpinlockRawMutex<1>, spi::Spi<'_, SPI0, Async>> = Mutex::new(spi_bus);

    //let spi_device = SpiDevice::new(&spi_mutex, cs);

    //let mut wiznet_state: embassy_net_wiznet::State<2, 2> = State::new();
    //spawner.spawn(embassy_net_wiznet::new::<
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
    //>([0x0; 6], &mut wiznet_state, spi_device, int, rst));

    //let mut ledc = LED_CONTROLLER.lock().await;
    //ledc.replace(LEDController::new(p.PIN_10, p.PIN_11, p.PIN_12, p.PIN_13));

    //let mut metc = METRONOME_CONTROLLER.lock().await;
    //metc.replace(MetronomeController::new());

    MODE_SIGNAL.signal(Mode::Lock);

    let i2c_config = i2c::Config::default();
    let i2c = I2c::new_async(p.I2C1, p.PIN_27, p.PIN_26, Irqs, i2c_config);

    let mut o = Output::new(p.PIN_10.reborrow(), Level::Low);

    //   let _ = spawner.spawn(network2::ethernet_task(spi));
    let _ = spawner.spawn(ui_task(GraphicsController::new(i2c)));
    let _ = spawner.spawn(input_translator_task());
    let _ = spawner.spawn(text_entry_task());
    let _ = spawner.spawn(action_fanout_task());
    let _ = spawner.spawn(button_scanner_task(ButtonScanner::new(
        p.PIN_2, p.PIN_3, p.PIN_4, p.PIN_5, p.PIN_6, p.PIN_7, p.PIN_8,
    )));

    loop {
        Timer::after_millis(250).await;
        o.toggle();
    }
}

//const BLINK_TIME_US: u64 = 50000;

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
