use crate::device::{CreateError, Device, WriteError};
use olc_pixel_game_engine as olc;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::thread;

pub mod cpu;
mod device;

fn main() -> Result<(), CreateError> {
    println!("rust6502");

    let font = psf::Font::new("./assets/koi8-14.psf").unwrap();
    let keys = Arc::new(Mutex::new(VecDeque::new()));
    let keys_clone = Arc::clone(&keys);
    let vram = Arc::new(Mutex::new(device::Ram::new(0x400, 4000)));
    let vram_clone = Arc::clone(&vram);
    let mut vga = device::vga::VGA::new(font, keys_clone, vram_clone);
    let running = Arc::new(Mutex::new(true));
    let running_clone = Arc::clone(&running);

    let jh = thread::spawn(move || {
        olc::start("rust6502", &mut vga, 8 * 80, 14 * 25, 4, 4).unwrap();
        let mut x = running_clone.lock().unwrap();
        *x = false;
    });
    let keyboard = device::vga::Keyboard::new(0x1400, keys);
    let ram = device::Ram::new(0, 0x0400);
    let rom = device::Rom::new_file(0x8000, "./code/example")?;
    let mapp = &mut device::device_map::DeviceMap::new();
    mapp.add(ram);
    mapp.add(rom);
    mapp.add(vram);
    mapp.add(keyboard);

    let mut cpu = cpu::CPU::new();
    let mut pins = cpu.pins;
    while *running.lock().unwrap() {
        pins = cpu.tick(pins);
        let addr = pins.address;
        if pins.rw == cpu::ReadWrite::Read {
            match mapp.read(addr) {
                Some(e) => pins.data = e,
                None => {}
            }
            println!(
                "Reading {:#06X}, data is now: {:#04X}",
                pins.address, pins.data
            );
        } else {
            let res = mapp.write(addr, pins.data);
            println!(
                "Writing {:#06X}, data will be: {:#04X}",
                pins.address, pins.data
            );
            match res {
                Ok(_) => {}
                Err(WriteError::NotWritable) => {}
                Err(WriteError::InvalidAddress) => {}
            }
        }
        if pins.address == 0xFFFF && pins.data == 0xFF {
            break;
        }
    }

    jh.join().unwrap();
    Ok(())
}
