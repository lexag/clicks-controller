//! This example implements a TCP echo server on port 1234 and using DHCP.
//! Send it some data, you should see it echoed back and printed in the console.
//!
//! Example written for the [`WIZnet W55RP20-EVB-Pico`](https://docs.wiznet.io/Product/ioNIC/W55RP20/w55rp20-evb-pico) board.
//! Note: the W55RP20 is a single package that contains both a RP2040 and the Wiznet W5500 ethernet
//! controller

use crate::events::Action;
use crate::led::LED;
use crate::ui::debug;
use crate::{ACTION_UPSTREAM, CONTROL_CH, STATE};
use common::mem::network::{IpAddress, SubscriberInfo};
use common::mem::str::StaticString;
use common::mem::typeflags::MessageType;
use common::protocol::message::SmallMessage;
use common::protocol::request::Request;
use core::net::Ipv4Addr;
use embassy_futures::select::{select, Either};
use embassy_futures::yield_now;
use embassy_net::udp::{PacketMetadata, SendError, UdpMetadata, UdpSocket};
use embassy_net::{IpEndpoint, Stack};
use embassy_net_wiznet::chip::W5500;
use embassy_net_wiznet::*;
use embassy_rp::gpio::{Input, Output};
use embassy_rp::peripherals::PIO0;
use embassy_rp::spi::Async;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_time::{Duration, Timer, WithTimeout};

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
    loop {
        let cfg = wait_for_config(stack).await;

        let mut state = STATE.lock().await;
        state.self_ip = IpAddress::new(cfg.address.address().octets(), 1234);
        let self_ip = state.self_ip;
        let endpoint = IpEndpoint::new(
            embassy_net::IpAddress::Ipv4(Ipv4Addr::new(
                state.core_ip.addr[0],
                state.core_ip.addr[1],
                state.core_ip.addr[2],
                state.core_ip.addr[3],
            )),
            state.core_ip.port,
        );
        drop(state);
        ACTION_UPSTREAM.send(Action::ForceRedraw).await;

        let mut rx_buffer = [0; 4096];
        let mut tx_buffer = [0; 4096];
        let mut rx_meta = [PacketMetadata::EMPTY; 16];
        let mut tx_meta = [PacketMetadata::EMPTY; 16];

        let mut buf = [0; 4096];

        let mut socket = UdpSocket::new(
            stack,
            &mut rx_meta,
            &mut rx_buffer,
            &mut tx_meta,
            &mut tx_buffer,
        );
        socket.bind(1234).unwrap();

        ACTION_UPSTREAM.send(Action::LEDBlip(LED::Connection)).await;
        if send_request(
            Request::Subscribe(SubscriberInfo {
                identifier: StaticString::new("ClicKS Hardware Controller"),
                address: self_ip,
                message_kinds: MessageType::Heartbeat
                    | MessageType::BeatData
                    | MessageType::ShutdownOccured
                    | MessageType::EventOccured,
                last_contact: 0,
            }),
            endpoint,
            &socket,
        )
        .await
        .is_err()
        {
            loop {
                if let Action::ReloadConnection = CONTROL_CH.receive().await {
                    break;
                }
            }
        } else {
            let _ = send_request(Request::Ping, endpoint, &socket).await;
            loop {
                buf.fill(0);
                // Handle network receives
                if let Ok(Ok((n, _ep))) = socket
                    .recv_from(&mut buf)
                    .with_timeout(Duration::from_millis(5))
                    .await
                {
                    let res = postcard::from_bytes(&buf[1..41]);
                    if let Ok(msg) = res {
                        receive_message(msg).await;
                    }
                }

                // Handle action receives
                if let Ok(action) = CONTROL_CH
                    .receive()
                    .with_timeout(Duration::from_millis(5))
                    .await
                {
                    match action {
                        Action::RequestToCore(request) => {
                            if let Err(err) = send_request(request, endpoint, &socket).await {
                                break;
                            }
                        }
                        Action::ReloadConnection => {
                            break;
                        }
                        _ => {}
                    }
                }
            }
        }

        ACTION_UPSTREAM
            .send(Action::LEDSet(LED::Connection, false))
            .await;
        socket.close();
    }
}

async fn receive_message(msg: SmallMessage) {
    match msg {
        SmallMessage::TransportData(data) => {
            if !data.running || data.ltc.f < 2 {
                ACTION_UPSTREAM.send(Action::NewTransportData(data)).await;
            }
        }
        SmallMessage::BeatData(data) => {
            ACTION_UPSTREAM.send(Action::NewBeatData(data.beat)).await;
        }
        SmallMessage::ShutdownOccured => {
            ACTION_UPSTREAM
                .send(Action::LEDSet(LED::Connection, false))
                .await;
        }
        SmallMessage::Heartbeat(data) => {
            ACTION_UPSTREAM
                .send(Action::LEDSet(LED::Connection, true))
                .await;
        }
        _ => {}
    }
}

async fn send_request(
    req: Request,
    endpoint: IpEndpoint,
    socket: &UdpSocket<'_>,
) -> Result<(), SendError> {
    let mut buf = [0; 4096];
    if let Ok(send_buf) = postcard::to_slice(&req, &mut buf) {
        socket.send_to(send_buf, endpoint).await
    } else {
        Err(SendError::PacketTooLarge)
    }
}
