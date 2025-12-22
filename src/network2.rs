//! This example implements a TCP echo server on port 1234 and using DHCP.
//! Send it some data, you should see it echoed back and printed in the console.
//!
//! Example written for the [`WIZnet W55RP20-EVB-Pico`](https://docs.wiznet.io/Product/ioNIC/W55RP20/w55rp20-evb-pico) board.
//! Note: the W55RP20 is a single package that contains both a RP2040 and the Wiznet W5500 ethernet
//! controller

use crate::STATE;
use common::mem::network::IpAddress;
use embassy_futures::yield_now;
use embassy_net::Stack;
use embassy_net_wiznet::chip::W5500;
use embassy_net_wiznet::*;
use embassy_rp::gpio::{Input, Output};
use embassy_rp::peripherals::PIO0;
use embassy_rp::spi::Async;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;

pub type SpiType = embassy_rp::pio_programs::spi::Spi<'static, PIO0, 0, Async>;
pub type SpiBusType = embassy_embedded_hal::shared_bus::asynch::spi::SpiDevice<
    'static,
    CriticalSectionRawMutex,
    SpiType,
    Output<'static>,
>;

#[embassy_executor::task]
pub async fn ethernet_task(
    runner: Runner<'static, W5500, SpiBusType, Input<'static>, Output<'static>>,
) -> ! {
    runner.run().await
}

#[embassy_executor::task]
pub async fn net_task(mut runner: embassy_net::Runner<'static, Device<'static>>) -> ! {
    runner.run().await
}

async fn wait_for_config(stack: Stack<'static>) -> embassy_net::StaticConfigV4 {
    loop {
        if let Some(config) = stack.config_v4() {
            return config.clone();
        }
        yield_now().await;
    }
}

#[embassy_executor::task]
pub async fn stack_task(stack: Stack<'static>) {
    let cfg = wait_for_config(stack).await;
    let local_addr = cfg.address.address();

    let mut state = STATE.lock().await;
    state.self_ip = IpAddress::new(cfg.address.address().octets(), 0);

    let mut rx_buffer = [0; 4096];
    let mut tx_buffer = [0; 4096];
    let mut buf = [0; 4096];
    return;
    //loop {
    //    let mut socket = embassy_net::tcp::TcpSocket::new(stack, &mut rx_buffer, &mut tx_buffer);
    //    socket.set_timeout(Some(Duration::from_secs(10)));

    //    led.set_low();
    //    info!("Listening on TCP:1234...");
    //    if let Err(e) = socket.accept(1234).await {
    //        warn!("accept error: {:?}", e);
    //        continue;
    //    }
    //    info!("Received connection from {:?}", socket.remote_endpoint());
    //    led.set_high();

    //    loop {
    //        let n = match socket.read(&mut buf).await {
    //            Ok(0) => {
    //                warn!("read EOF");
    //                break;
    //            }
    //            Ok(n) => n,
    //            Err(e) => {
    //                warn!("{:?}", e);
    //                break;
    //            }
    //        };
    //        info!("rxd {}", core::str::from_utf8(&buf[..n]).unwrap());

    //        if let Err(e) = socket.write_all(&buf[..n]).await {
    //            warn!("write error: {:?}", e);
    //            break;
    //        }
    //    }
    //}
}
