use crate::{events::Action, ACTION_UPSTREAM};
use common::mem::str::StaticString;
use embassy_rp::{
    peripherals::{SPI0, SPI1},
    spi::{self, Blocking},
};
use embassy_time::Timer;

const BLOCK_COMMON_REGISTER: u8 = 0x00;
const BLOCK_SOCKET_0_REGISTER: u8 = 0x01;
const BLOCK_SOCKET_0_TXBUFFER: u8 = 0x02;
const BLOCK_SOCKET_0_RXBUFFER: u8 = 0x03;
const BLOCK_SOCKET_1_REGISTER: u8 = 0x05;
const BLOCK_SOCKET_1_TXBUFFER: u8 = 0x06;
const BLOCK_SOCKET_1_RXBUFFER: u8 = 0x07;
const BLOCK_SOCKET_2_REGISTER: u8 = 0x09;
const BLOCK_SOCKET_2_TXBUFFER: u8 = 0x0a;
const BLOCK_SOCKET_2_RXBUFFER: u8 = 0x0b;
const BLOCK_SOCKET_3_REGISTER: u8 = 0x0d;
const BLOCK_SOCKET_3_TXBUFFER: u8 = 0x0e;
const BLOCK_SOCKET_3_RXBUFFER: u8 = 0x0f;
const BLOCK_SOCKET_4_REGISTER: u8 = 0x11;
const BLOCK_SOCKET_4_TXBUFFER: u8 = 0x12;
const BLOCK_SOCKET_4_RXBUFFER: u8 = 0x13;
const BLOCK_SOCKET_5_REGISTER: u8 = 0x15;
const BLOCK_SOCKET_5_TXBUFFER: u8 = 0x16;
const BLOCK_SOCKET_5_RXBUFFER: u8 = 0x17;
const BLOCK_SOCKET_6_REGISTER: u8 = 0x19;
const BLOCK_SOCKET_6_TXBUFFER: u8 = 0x1a;
const BLOCK_SOCKET_6_RXBUFFER: u8 = 0x1b;
const BLOCK_SOCKET_7_REGISTER: u8 = 0x1d;
const BLOCK_SOCKET_7_TXBUFFER: u8 = 0x1e;
const BLOCK_SOCKET_7_RXBUFFER: u8 = 0x1f;

const COM_ADDR_MODE: u16 = 0x0000;
const COM_ADDR_GATEWAY_ADDR: u16 = 0x0001;
const COM_ADDR_SUBNET_MASK_ADDR: u16 = 0x0005;
const COM_ADDR_HARDWARE_ADDR: u16 = 0x0009;
const COM_ADDR_IP_ADDR: u16 = 0x000F;

const SOCK_ADDR_MODE: u16 = 0x0000;
const SOCK_ADDR_COMMAND: u16 = 0x0001;
const SOCK_ADDR_INTERRUPT: u16 = 0x0002;
const SOCK_ADDR_STATUS: u16 = 0x0003;
const SOCK_ADDR_SOURCE_PORT: u16 = 0x0004;
const SOCK_ADDR_DEST_HARDWARE_ADDR: u16 = 0x0006;
const SOCK_ADDR_DEST_IP_ADDR: u16 = 0x000C;
const SOCK_ADDR_DEST_PORT: u16 = 0x0010;
const SOCK_ADDR_MAX_SEG_SIZE: u16 = 0x0012;

const SOCK_COMMAND_OPEN: u8 = 0x01;
const SOCK_COMMAND_LISTEN: u8 = 0x02;
const SOCK_COMMAND_CONNECT: u8 = 0x04;
const SOCK_COMMAND_DISCON: u8 = 0x08;
const SOCK_COMMAND_CLOSE: u8 = 0x10;
const SOCK_COMMAND_SEND: u8 = 0x20;
const SOCK_COMMAND_SEND_MAC: u8 = 0x40;
const SOCK_COMMAND_SEND_KEEP: u8 = 0x80;

const MAC_ADDRESS: [u8; 6] = [0x02, 0x00, 0x34, 0x00, 0x00, 0x00];
const SOURCE_IP_ADDRESS: [u8; 4] = [192, 168, 1, 72];
const SOURCE_PORT: [u8; 2] = [0x13, 0x88];
const CORE_IP_ADDRESS: [u8; 4] = [192, 168, 1, 72];
const CORE_PORT: [u8; 2] = [0x1f, 0x91];
const GATEWAY_ADDRESS: [u8; 4] = [192, 168, 0, 1];

#[embassy_executor::task]
pub async fn ethernet_task(mut spi: spi::Spi<'static, SPI0, Blocking>) {
    SpiW::runtime_tests().await;
    let mut d = SpiW {
        spi,
        read_buf: [0u8; SpiW::MAX_DATA_SIZE],
    };
    // Setup device
    d.write(BLOCK_COMMON_REGISTER, COM_ADDR_MODE, &[0b10100000])
        .await;
    d.write(
        BLOCK_COMMON_REGISTER,
        COM_ADDR_GATEWAY_ADDR,
        &GATEWAY_ADDRESS,
    )
    .await;
    d.write(BLOCK_COMMON_REGISTER, COM_ADDR_HARDWARE_ADDR, &MAC_ADDRESS)
        .await;
    d.write(BLOCK_COMMON_REGISTER, COM_ADDR_IP_ADDR, &SOURCE_IP_ADDRESS)
        .await;

    // Setup socket #1
    d.write(BLOCK_SOCKET_1_REGISTER, SOCK_ADDR_MODE, &[0b00000010])
        .await;
    d.write(BLOCK_SOCKET_1_REGISTER, SOCK_ADDR_SOURCE_PORT, &SOURCE_PORT)
        .await;

    // Read back socket port to see if setting did anything
    debug("let's read").await;
    let res = d.read(BLOCK_SOCKET_1_REGISTER, SOCK_ADDR_SOURCE_PORT).await;
    debug("i read").await;
    let mut buf = [0u8; 256];
    let s = format_no_std::show(
        &mut buf,
        format_args!(
            "r: {:X} {:X} {:X} {:X} {:X} {:X} {:X} {:X}",
            res[0], res[1], res[2], res[3], res[4], res[5], res[6], res[7]
        ),
    )
    .unwrap_or("fmt failed");
    debug(s).await;

    // Connect to core ip
    d.write(
        BLOCK_SOCKET_1_REGISTER,
        SOCK_ADDR_DEST_IP_ADDR,
        &CORE_IP_ADDRESS,
    )
    .await;
    d.write(BLOCK_SOCKET_1_REGISTER, SOCK_ADDR_DEST_PORT, &CORE_PORT)
        .await;
    d.write(
        BLOCK_SOCKET_1_REGISTER,
        SOCK_ADDR_COMMAND,
        &[SOCK_COMMAND_OPEN],
    )
    .await;
    Timer::after_millis(40).await;
    d.write(
        BLOCK_SOCKET_1_REGISTER,
        SOCK_ADDR_COMMAND,
        &[SOCK_COMMAND_CONNECT],
    )
    .await;
}

struct SpiW {
    pub spi: spi::Spi<'static, SPI0, Blocking>,
    pub read_buf: [u8; Self::MAX_DATA_SIZE],
}

impl SpiW {
    const MAX_DATA_SIZE: usize = 8;

    async fn write(&mut self, block: u8, addr: u16, data: &[u8]) {
        let mut frame = [0u8; Self::MAX_DATA_SIZE + 3];
        Self::stamp_frame_header(&mut frame, block, addr, true);
        frame[3..data.len() + 3].copy_from_slice(data);
        let res = self.spi.blocking_write(&frame[0..data.len()]);
    }

    fn stamp_frame_header(frame: &mut [u8], block: u8, addr: u16, write: bool) {
        frame[0] = (addr >> 8) as u8;
        frame[1] = (addr & 0x00FF) as u8;
        frame[2] = block << 3 | (write as u8) << 2;
    }

    async fn read(&mut self, block: u8, addr: u16) -> &[u8] {
        let mut out = [0u8; Self::MAX_DATA_SIZE];
        let mut frame = [0u8; 3];
        Self::stamp_frame_header(&mut frame, block, addr, false);
        let _ = self.spi.blocking_transfer(&mut out, &frame);
        self.read_buf = out;
        &self.read_buf
    }

    async fn runtime_tests() {
        let mut frame = [0u8; 3];
        SpiW::stamp_frame_header(&mut frame, BLOCK_SOCKET_1_TXBUFFER, 0x0040, true);
        if frame != [0b00000000, 0b01000000, 0b00110100] {
            debug("W5500 test failed").await;
            debug("frame_header_write").await;
        }

        let mut frame = [0u8; 3];
        SpiW::stamp_frame_header(&mut frame, BLOCK_SOCKET_3_RXBUFFER, 0x0100, false);
        if frame != [0b00000001, 0b00000000, 0b01111000] {
            debug("W5500 test failed").await;
            debug("frame_header_read").await;
        }

        debug("tests passed").await;
    }
}
