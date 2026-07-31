/*
 * Contains demos for the PCI bus scan and reading the MAC address of a RTL8139 Ethernet card.
 *
 * Author: Fabian Ruhland, Heinrich Heine University Duesseldorf, 2026-04-02
 * License: GPLv3
 */
use crate::device::pci::{pci_bus};
use crate::device::{pit, rtl8139};
use crate::device::terminal::terminal;
use crate::library::input;
use crate::thread::scheduler::scheduler;
use crate::thread::thread::Thread;

pub fn print_pci_devices() {
    terminal().lock().clear();
    println!("PCI Demo:\n");

    for device in pci_bus().iter() {
        println!("Found PCI device {:04x}:{:04x}", device.read_vendor_id(), device.read_device_id());
    }

    println!("\nPress 'Enter' to exit...");
    input::wait_for_return();
}

pub fn rtl8139_demo() {
    terminal().lock().clear();
    println!("RTL8139 Demo:\n");

    let receive_loop = Thread::new(test_receive_loop);
    let send_loop = Thread::new(test_send_loop);
    scheduler().ready(receive_loop);
    scheduler().ready(send_loop);
    scheduler().schedule();
}

fn test_receive_loop() {
    println!("Waiting for packets...");

    loop {
        if let Some(frame) = rtl8139::receive_packet() {

            let dest = &frame.data[0..6];
            let src = &frame.data[6..12];
            let ethertype = (frame.data[12] as u16) << 8 | (frame.data[13] as u16);

            println!(
                "Packet [{} bytes] | Dest: {:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X} | Src: {:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X} | Type: 0x{:04X}",
                frame.length,
                dest[0], dest[1], dest[2], dest[3], dest[4], dest[5],
                src[0], src[1], src[2], src[3], src[4], src[5],
                ethertype
            );
        }

        pit::wait(100);
    }
}

fn test_send_loop() {
    loop {
        send_test_packet();
        pit::wait(1000);
    }
}

fn send_test_packet() {
    let mut frame = [0u8; 64];

    // Destination MAC (Broadcast)
    frame[0..6].copy_from_slice(&[0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF]);

    // Source MAC (52:54:00:12:34:56)
    frame[6..12].copy_from_slice(&[0x52, 0x54, 0x00, 0x12, 0x34, 0x56]);

    // EtherType (0x88B5 is reserved for local experimental use)
    frame[12] = 0x88;
    frame[13] = 0xB5;

    let payload = b"Hello from HeineOS!";
    frame[14..(14 + payload.len())].copy_from_slice(payload);

    println!("Sending test packet...");
    rtl8139::send_packet(&frame);
}
