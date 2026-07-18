use alloc::boxed::Box;
use alloc::vec;
use alloc::vec::Vec;
use bitflags::bitflags;
use log::{info, warn};
use crate::device::cpu::IoPort;
use crate::device::pci::{pci_bus, Command};
use crate::device::pic::{Irq, PIC};
use crate::interrupt::dispatcher::{IntVectors, InterruptVector};
use crate::interrupt::isr::ISR;
use crate::library::once::Once;
use crate::library::spinlock::Spinlock;
use crate::panic;

const REG_MAC: u16     = 0x00;
const REG_RBSTART: u16 = 0x30; // 4 Bytes (32-Bit)
const REG_CMD: u16     = 0x37; // 1 Byte (8-Bit)
const REG_CAPR: u16    = 0x38; // 2 Bytes (16-Bit)
const REG_IMR: u16     = 0x3C; // 2 Bytes (16-Bit)
const REG_ISR: u16     = 0x3E; // 2 Bytes (16-Bit)
const REG_RCR: u16     = 0x44; // 4 Bytes (32-Bit)
const REG_CONFIG_1: u16 = 0x52;

const BUFFER_SIZE: usize = 8192 + 1500 + 16;

const QUEUE_CAPACITY: usize = 64;

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct CommandReg: u8 {
        const BUFFER_EMPTY = 1 << 0;
        const TX_ENABLE = 1 << 2;
        const RX_ENABLE = 1 << 3;
        const RESET = 1 << 4;
    }
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct InterruptReg: u16 {
        const RECEIVE_OK = 1 << 0;
        const TRANSMIT_OK = 1 << 2;
        const RX_OVERFLOW = 1 << 4;
    }
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct ReceiveConfig: u32 {
        const ACCEPT_ALL_PACKETS = 1 << 0;
        const ACCEPT_PHYSICAL_MATCH = 1 << 1;
        const ACCEPT_MULTICAST = 1 << 2;
        const ACCEPT_BROADCAST = 1 << 3;
        const WRAP = 1 << 7;
    }
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct ReceiveStatus: u16 {
        /// Receive OK: When set, indicates that a good packet is received.
        const ROK  = 1 << 0;
        /// Frame Alignment Error: When set, indicates that a frame alignment error occurred on this received packet.
        const FAE  = 1 << 1;
        /// CRC Error: When set, indicates that a CRC error occurred on the received packet.
        const CRC  = 1 << 2;
        /// Long Packet: Set to 1 indicates that the size of the received packet exceeds 4k bytes.
        const LONG = 1 << 3;
        /// Runt Packet Received: Set to 1 indicates that the received packet length is smaller than 64 bytes.
        const RUNT = 1 << 4;
        /// Invalid Symbol Error: (100BASE-TX only) An invalid symbol was encountered during the reception of this packet.
        const ISE  = 1 << 5;

        // Bits 6 - 12 are Reserved.

        /// Broadcast Address Received: Set to 1 indicates that a broadcast packet is received.
        const BAR  = 1 << 13;
        /// Physical Address Matched: Set to 1 indicates that the destination address of this packet
        /// matches the value written in ID registers.
        const PAM  = 1 << 14;
        /// Multicast Address Received: Set to 1 indicates that a multicast packet is received.
        const MAR  = 1 << 15;
    }
}

static RTL8139: Once<Spinlock<Rtl8139>> = Once::new();

#[derive(Copy, Clone)]
pub struct EthernetFrame {
    pub data: [u8; 2048],
    pub length: usize,
}

impl EthernetFrame {
    const fn empty() -> EthernetFrame {
        EthernetFrame {
            data: [0; 2048],
            length: 0,
        }
    }
}

pub struct PacketQueue {
    buffer: Vec<EthernetFrame>,
    head: usize,
    tail: usize,
    size: usize,
    dropped_packets: usize,
}

impl PacketQueue {
    pub fn new() -> PacketQueue {
        PacketQueue {
            buffer: vec![EthernetFrame::empty(); QUEUE_CAPACITY],
            head: 0,
            tail: 0,
            size: 0,
            dropped_packets: 0,
        }
    }

    pub fn push(&mut self, packet_data: &[u8]) {
        if self.size == QUEUE_CAPACITY {
            self.dropped_packets += 1;
            warn!("Packet queue is full, dropping packet. Total dropped packets: {}", self.dropped_packets);

            return;
        }

        if packet_data.len() > 2048 {
            warn!("Ethernet frame too large, dropping packet: {} bytes", packet_data.len());
            self.dropped_packets += 1;

            return;
        }

        // Copy the packet data into the buffer
        let frame = &mut self.buffer[self.head];
        frame.length = packet_data.len();
        frame.data[..packet_data.len()].copy_from_slice(packet_data);

        self.head = (self.head + 1) % QUEUE_CAPACITY;
        self.size += 1;
        info!("Packet received and added to queue. Queue size: {}", self.size);
    }

    pub fn pop(&mut self) -> Option<EthernetFrame> {
        if self.size == 0 {
            return None; // Queue is empty
        }

        let frame = self.buffer[self.tail];

        self.tail = (self.tail + 1) % QUEUE_CAPACITY;
        self.size -= 1;

        Some(frame)
    }
}

struct Rtl8139 {
    io_base: u16,
    rx_buffer: Vec<u8>,
    packet_queue: PacketQueue,
    rx_read_offset: usize,
}

impl Rtl8139 {
    pub fn new(io_base: u16) -> Rtl8139 {
        let rx_buffer = vec![0u8; BUFFER_SIZE];
        let packet_queue = PacketQueue::new();
        let rx_read_offset = 0;

        Rtl8139 { io_base , rx_buffer, packet_queue, rx_read_offset }
    }

    pub fn write_command(&self, cmd: u8) {
        unsafe {
            IoPort::new(self.io_base + REG_CMD).outb(cmd);
        }
    }

    pub fn read_command(&self) -> u8 {
        unsafe {
            IoPort::new(self.io_base + REG_CMD).inb()
        }
    }

    pub fn read_interrupt_status(&self) -> u16 {
        unsafe {
            IoPort::new(self.io_base + REG_ISR).inw()
        }
    }

    pub fn power_on(&self) {
        // Send 0x00 to the CONFIG_1 register
        unsafe {
            IoPort::new(self.io_base + REG_CONFIG_1).outb(0x00);
        }
    }

    pub fn software_reset(&self) {
        // Remove garbage left in the buffers or registers
        // Write 1 to the RESET bit
        self.write_command(CommandReg::RESET.bits());

        // Wait until the RST bit is cleared
        while self.read_command() & CommandReg::RESET.bits() != 0 { }
    }

    pub fn init_rx_buffer(&self) {
        // Write buffer memory location to the RBSTART register
        unsafe {
            IoPort::new(self.io_base + REG_RBSTART).outdw(self.rx_buffer.as_ptr() as u32);
        }
    }

    pub fn allow_interrupts(&self) {
        let mask = InterruptReg::RECEIVE_OK | InterruptReg::TRANSMIT_OK | InterruptReg::RX_OVERFLOW;

        unsafe {
            IoPort::new(self.io_base + REG_IMR).outw(mask.bits());
        }
    }

    pub fn configure_receive_buffer(&self) {
        let config = ReceiveConfig::ACCEPT_ALL_PACKETS
            | ReceiveConfig::ACCEPT_PHYSICAL_MATCH
            | ReceiveConfig::ACCEPT_MULTICAST
            | ReceiveConfig::ACCEPT_BROADCAST
            | ReceiveConfig::WRAP;

        unsafe {
            IoPort::new(self.io_base + REG_RCR).outdw(config.bits());
        }
    }

    pub fn enable_rx_tx(&self) {
        self.write_command(CommandReg::RX_ENABLE.bits() | CommandReg::TX_ENABLE.bits());
    }

    pub fn acknowledge_interrupt(&self, status: u16) {
        unsafe {
            // Write the status register to clear the interrupt
            IoPort::new(self.io_base + REG_ISR).outw(status);
        }
    }

    pub fn read_capr(&self) -> u16 {
        unsafe {
            IoPort::new(self.io_base + REG_CAPR).inw()
        }
    }

    pub fn write_capr(&self, capr: u16) {
        unsafe {
            IoPort::new(self.io_base + REG_CAPR).outw(capr);
        }
    }

    pub fn rx_buffer_empty(&self) -> bool {
        self.read_command() & CommandReg::BUFFER_EMPTY.bits() != 0
    }

    pub fn process_rx_buffer(&mut self) {
        while !self.rx_buffer_empty() {
            let offset = self.rx_read_offset;

            // Read 16-bit status + 16-bit packet length
            let header = u32::from_le_bytes([
                    self.rx_buffer[offset],
                    self.rx_buffer[offset + 1],
                    self.rx_buffer[offset + 2],
                    self.rx_buffer[offset + 3]
            ]);

            let status = ReceiveStatus::from_bits_truncate(header as u16);
            let length = (header >> 16) as usize;

            // Check if the packet is valid and copy it to the packet queue
            if status.contains(ReceiveStatus::ROK) {
                // The length includes the packet data and a 4-byte CRC
                let packet_len = length.saturating_sub(4);
                let data_start = offset + 4;
                let data_end = data_start + packet_len;

                if data_end <= self.rx_buffer.len() {
                    let packet_data = &self.rx_buffer[data_start..data_start + packet_len];
                    self.packet_queue.push(packet_data);
                } else {
                    warn!("Hardware fault: packet length exceeds buffer size.");
                }
            }

            // Packets are always aligned to 4-byte boundaries
            // Next offset is: current_offset + 4-byte header + packet_length aligned to 4-bytes
            self.rx_read_offset = (offset + 4 + length + 3) & !3;

            // Wrap around at the end of the buffer with modulo (don't start at 0) since the RTL8139
            // still writes the overflowing bytes to the start of the buffer (even with WRAP=1)
            self.rx_read_offset %= 8192;

            // RTL8139 hardware quirk: write the offset minus 16 to CAPR!
            let capr_val = (self.rx_read_offset as u16).wrapping_sub(16);
            self.write_capr(capr_val);
        }
    }
}

fn init_rtl8139(io_base: u16) {
    RTL8139.init(|| Spinlock::new(Rtl8139::new(io_base)));

    let rtl8139 = RTL8139.get().unwrap().lock();

    rtl8139.power_on();
    rtl8139.software_reset();
    rtl8139.init_rx_buffer();
    rtl8139.allow_interrupts();
    rtl8139.configure_receive_buffer();
    rtl8139.enable_rx_tx();
}

struct Rtl8139ISR;

impl ISR for Rtl8139ISR {
    fn trigger(&self) {
        let mut rtl8139 = RTL8139.get().unwrap().lock();

        let status = rtl8139.read_interrupt_status();
        let flags = InterruptReg::from_bits_truncate(status);

        info!("RTL8139 Interrupt! Status: {:?}", flags);

        if flags.contains(InterruptReg::RECEIVE_OK) || flags.contains(InterruptReg::RX_OVERFLOW) {
            rtl8139.process_rx_buffer();
        }

        if flags.contains(InterruptReg::TRANSMIT_OK) {
            info!("Packet sent!");
        }

        rtl8139.acknowledge_interrupt(status);
    }
}

pub fn plugin() {
    let rtl8139 = pci_bus().iter().find(|device| {
        device.read_vendor_id() == 0x10ec && device.read_device_id() == 0x8139
    });

    let Some (rtl8139) = rtl8139 else { return };
    info!("Found RTL8139 device!");

    // Enable I/O access AND PCI Bus Mastering (DMA)
    rtl8139.write_command(rtl8139.read_command() | Command::IoEnable as u16 | Command::BusMasterEnable as u16);

    let bar0 = rtl8139.read_bar(0);
    if bar0 & 0x1 == 0 {
        // Memory Mapped I/O not supported
        panic!("RTL8139 MMIO is not supported");
    }

    let io_base = (bar0 & 0xfffc) as u16;
    init_rtl8139(io_base);

    let irq_num = rtl8139.read_interrupt_line();
    let irq_enum = match irq_num {
        9  => Irq::Free1,
        10 => Irq::Free2,
        11 => Irq::Free3,
        _  => panic!("Unsupported PCI IRQ: {}", irq_num),
    };
    PIC.lock().allow(irq_enum);

    let vector_num = (irq_num + 32) as usize;
    IntVectors::register_dynamic(vector_num, Box::new(Rtl8139ISR));
}



