use crate::device::{CreateError, Device, WriteError};

pub mod cpu;
mod device;

fn main() -> Result<(), CreateError> {
    println!("rust6502");

    let ram = device::Ram::new(0, 0x0400);
    let rom = device::Rom::new_file(0x8000, "./example")?;
    let mapp = &mut device::device_map::DeviceMap::new();
    mapp.add(ram);
    mapp.add(rom);

    let mut cpu = cpu::CPU::new();
    let mut pins = cpu.pins;
    // return Ok(());
    for _ in 0..0x30 {
        pins = cpu.tick(pins);
        // println!("Addr = {}, data = {}", pins.address, pins.data);
        let addr = pins.address;
        if pins.rw == cpu::ReadWrite::Read {
            match mapp.read(addr) {
                Some(e) => pins.data = e,
                None => {}
            }
            println!("Reading {:#06X}, data is now: {:#04X}", pins.address, pins.data);
        } else {
            let res = mapp.write(addr, pins.data);
            println!("Writing {:#06X}, data will be: {:#04X}", pins.address, pins.data);
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
    Ok(())
}
