pub mod cpu;
pub mod device;

fn main() {
    println!("rust6502");
    let mut mem = [0u8; 65536];
    mem[0] = 0xA9;
    mem[1] = 0x01;
    mem[2] = 0x8D;
    mem[3] = 0x00;
    mem[4] = 0x10;

    let mut cpu = cpu::CPU::new();
    let mut pins = cpu.pins;
    loop {
        pins = cpu.tick(pins);
        println!("Addr = {}, data = {}", pins.address, pins.data);
        let addr = pins.address;
        if pins.rw == cpu::ReadWrite::Read {
            pins.data = mem[addr as usize]
        } else {
            mem[addr as usize] = pins.data
        }
        if pins.address == 0xFFFF && pins.data == 0xFF {
            break;
        }
    }
    println!("mem[0x1000] = {}", mem[0x1000]);
}
