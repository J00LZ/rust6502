use crate::device::{CreateError, Device, WriteError};
use olc_pixel_game_engine as olc;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::thread::sleep;
use std::time::{Duration, SystemTime};

pub mod cpu;
mod device;

fn main() -> Result<(), CreateError> {
    println!("rust6502");

    let font = psf::Font::new("./assets/koi8-14.psf").unwrap();
    let keys = Arc::new(Mutex::new(VecDeque::new()));
    let keys_clone = Arc::clone(&keys);
    let vram = Arc::new(Mutex::new(device::Ram::new(0x500, 0x1000)));
    let vram_clone = Arc::clone(&vram);
    let mut vga = device::vga::Vga::new(font, keys_clone, vram_clone);
    let running = Arc::new(AtomicBool::new(true));
    let running_clone = Arc::clone(&running);

    let jh = thread::spawn(move || {
        olc::start("rust6502", &mut vga, 8 * 80, 14 * 25, 4, 4).unwrap();
        running_clone.store(false, Ordering::Release);
    });
    let keyboard = device::vga::Keyboard::new(0x10, keys);
    let ram = device::Ram::new(0x0100, 0x0400);
    let rom = device::Rom::new_file(0x8000, "./code/bin/example")?;
    let kernel = device::Rom::new_file(0xE000, "./code/bin/kernel")?;
    let interrupts = device::Rom::interrupts(0, 0xE000, 0);

    let mapp = &mut device::device_map::DeviceMap::new();
    mapp.add(ram);
    mapp.add(rom);
    mapp.add(vram);
    mapp.add(keyboard);
    mapp.add(interrupts);
    mapp.add(kernel);

    let mut cpu = cpu::CPU::new();
    let mut pins = cpu.pins;
    while running.load(Ordering::Acquire) {
        let now = SystemTime::now();
        pins = cpu.tick(pins);
        let addr = pins.address;
        if pins.rw == cpu::ReadWrite::Read {
            if let Some(e) = mapp.read(addr) {
                pins.data = e
            }
            // println!(
            //     "Reading {:#06X}, data is now: {:#04X}",
            //     pins.address, pins.data
            // );
        } else {
            let res = mapp.write(addr, pins.data);
            // println!(
            //     "Writing {:#06X}, data will be: {:#04X}",
            //     pins.address, pins.data
            // );
            match res {
                Ok(_) => {}
                Err(WriteError::NotWritable) => {}
                Err(WriteError::InvalidAddress) => {}
            }
        }
        if pins.address == 0xFFFF && pins.data == 0xFF {
            break;
        }
        match now.elapsed() {
            Ok(elapsed) => {
                // it prints '2'
                let elapsed = elapsed.as_nanos();
                println!("execution took {}ns!", elapsed);
                if elapsed < 1000 {
                    sleep(Duration::from_nanos((1000 - elapsed) as u64))
                }
            }
            Err(e) => {
                // an error occurred!
                println!("Error: {:?}", e);
            }
        }
    }

    jh.join().unwrap();
    Ok(())
}
