use alloc::boxed::Box;
use alloc::vec;
use alloc::vec::Vec;
use bitflags::bitflags;
use log::info;
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
const REG_IMR: u16     = 0x3C; // 2 Bytes (16-Bit)
const REG_ISR: u16     = 0x3E; // 2 Bytes (16-Bit)
const REG_RCR: u16     = 0x44; // 4 Bytes (32-Bit)
const REG_CONFIG_1: u16 = 0x52;

const BUFFER_SIZE: usize = 8192 + 1500 + 16;

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

static RTL8139: Once<Spinlock<Rtl8139>> = Once::new();

struct Rtl8139 {
    io_base: u16,
    rx_buffer: Vec<u8>,
}

impl Rtl8139 {
    pub fn new(io_base: u16) -> Rtl8139 {
        let rx_buffer = vec![0u8; BUFFER_SIZE];

        Rtl8139 { io_base , rx_buffer }
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
        while CommandReg::from_bits_truncate(self.read_command()).contains(CommandReg::RESET) { }
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
        let rtl8139 = RTL8139.get().unwrap().lock();

        let status = rtl8139.read_interrupt_status();
        let flags = InterruptReg::from_bits_truncate(status);

        info!("RTL8139 Interrupt! Status: {:?}", flags);

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



