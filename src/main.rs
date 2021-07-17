use crate::device::{CreateError, Device, WriteError};
use std::borrow::{Borrow, BorrowMut};
use std::error::Error;

pub mod cpu;
pub mod device;

fn main() -> Result<(), CreateError> {
    println!("rust6502");

    let mut ram = device::Ram::new(0, 0x0400);
    let mut rom = device::Rom::new_file(0x8000, "./blink")?;
    let mut mapp = &mut device::device_map::DeviceMap::new();
    {
        mapp.add(&ram);
        mapp.add(&rom);
    }
    let mut cpu = cpu::CPU::new();
    let mut pins = cpu.pins;
    loop {
        pins = cpu.tick(pins);
        println!("Addr = {}, data = {}", pins.address, pins.data);
        let addr = pins.address;
        if pins.rw == cpu::ReadWrite::Read {
            match mapp.read(addr) {
                Some(e) => pins.data = e,
                None => {}
            }
        } else {
            let res = mapp.write(addr, pins.data);
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
