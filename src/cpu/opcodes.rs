use crate::cpu::opcodes::AddressingMode::*;
use crate::cpu::opcodes::Opcode::*;

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum AddressingMode {
    Acc,
    Abs,
    AbsX,
    AbsY,
    Imm,
    Impl,
    Ind,
    XInd,
    IndY,
    Rel,
    Zpg,
    ZpgX,
    ZpgY,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Opcode {
    // add with carry
    ADC,
    // and (with accumulator)
    AND,
    // arithmetic shift left
    ASL,
    // branch on carry clear
    BCC,
    // branch on carry set
    BCS,
    // branch on equal (zero set)
    BEQ,
    // bit test
    BIT,
    // branch on minus (negative set)
    BMI,
    // branch on not equal (zero clear)
    BNE,
    // branch on plus (negative clear)
    BPL,
    // break / interrupt
    BRK,
    // branch on overflow clear
    BVC,
    // branch on overflow set
    BVS,
    // clear carry
    CLC,
    // clear decimal
    CLD,
    // clear interrupt disable
    CLI,
    // clear overflow
    CLV,
    // compare (with accumulator)
    CMP,
    // compare with X
    CPX,
    // compare with Y
    CPY,
    // decrement
    DEC,
    // decrement X
    DEX,
    // decrement Y
    DEY,
    // exclusive or (with accumulator)
    EOR,
    // increment
    INC,
    // increment X
    INX,
    // increment Y
    INY,
    // jump
    JMP,
    // jump subroutine
    JSR,
    // load accumulator
    LDA,
    // load X
    LDX,
    // load Y
    LDY,
    // logical shift right
    LSR,
    // no operation
    NOP,
    // or with accumulator
    ORA,
    // push accumulator
    PHA,
    // push processor status (SR)
    PHP,
    // pull accumulator
    PLA,
    // pull processor status (SR)
    PLP,
    // rotate left
    ROL,
    // rotate right
    ROR,
    // return from interrupt
    RTI,
    // return from subroutine
    RTS,
    // subtract with carry
    SBC,
    // set carry
    SEC,
    // set decimal
    SED,
    // set interrupt disable
    SEI,
    // store accumulator
    STA,
    // store X
    STX,
    // store Y
    STY,
    // transfer accumulator to X
    TAX,
    // transfer accumulator to Y
    TAY,
    // transfer stack pointer to X
    TSX,
    // transfer X to accumulator
    TXA,
    // transfer X to stack pointer
    TXS,
    // transfer Y to accumulator
    TYA,
    //Here be illegal opcodes
    // AND oper + LSR
    ALR,
    // AND oper + set C as ASL
    ANC,
    // AND oper + set C as ROL
    ANC2,
    // * AND X + AND oper
    ANE,
    // AND oper + ROR
    ARR,
    // DEC oper + CMP oper
    DCP,
    // INC oper + SBC oper
    ISC,
    // LDA/TSX oper
    LAS,
    // LDA oper + LDX oper
    LAX,
    // Store * AND oper in A and X
    LXA,
    // ROL oper + AND oper
    RLA,
    // ROR oper + ADC oper
    RRA,
    // A AND X -> M
    SAX,
    // CMP and DEX at once, sets flags like CMP
    SBX,
    // Stores A AND X AND (high-byte of addr. + 1) at addr.
    SHA,
    // Stores X AND (high-byte of addr. + 1) at addr.
    SHX,
    // Stores Y AND (high-byte of addr. + 1) at addr.
    SHY,
    // ASL oper + ORA oper
    SLO,
    // LSR oper + EOR oper
    SRE,
    // Puts A AND X in SP and stores A AND X AND (high-byte of addr. + 1) at addr.
    TAS,
    // effectively same as normal SBC immediate, instr. E9.
    USBC,
    // These instructions freeze the CPU.
    JAM,
}

#[derive(Debug, Eq, PartialEq)]
pub struct Instruction {
    pub opcode: Opcode,
    pub mode: AddressingMode,
    pub cycles: u16,
}

fn ins(opcode: Opcode, mode: AddressingMode, cycles: u16) -> Instruction {
    Instruction {
        opcode,
        mode,
        cycles,
    }
}
#[cfg(test)]
mod test {
    use crate::cpu::opcodes::Instruction;

    #[test]
    #[cfg(test)]
    fn yeet() {
        assert_eq!(
            Instruction::from_byte(0x00),
            super::ins(super::BRK, super::Impl, 7)
        );
        assert_eq!(
            Instruction::from_byte(0x64),
            super::ins(super::NOP, super::Zpg, 7)
        );
    }
}

impl Instruction {
    pub fn from_byte(data: u8) -> Instruction {
        panic!("help?")
    }
}
