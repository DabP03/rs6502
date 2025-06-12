//! 6502 embeddable emulator.
//!
//! Provides a struct that simulates the 6502 processor.
//! You can control how it reads and writes by providing closures in the new() function.

#![no_std]

pub const BASE_STACK: u16 = 0x100;

#[repr(u8)]
pub enum Flag {
    CARRY = 0x01,
    ZERO = 0x02,
    INTERRUPT = 0x04,
    DECIMAL = 0x08,
    BREAK = 0x10,
    CONSTANT = 0x20,
    OVERFLOW = 0x40,
    SIGN = 0x80,
}

pub struct Rs6502<'a> {
    pub pc: u16,
    pub sp: u8,
    pub a: u8,
    pub x: u8,
    pub y: u8,
    pub status: u8,
    read_closure: &'a dyn Fn(&mut Rs6502, u16) -> u8,
    write_closure: &'a dyn Fn(&mut Rs6502, u16, u8),
    // helper variables
    instructions: u32,
    clock_ticks: u32,
    clock_goal: u32,
    old_pc: u16,
    ea: u16,
    rel_addr: u16,
    value: u16,
    result: u16,
    opcode: u8,
    old_status: u8,
}

impl<'a> Rs6502<'a> {
    pub fn new(
        read: &'a dyn Fn(&mut Rs6502, u16) -> u8,
        write: &'a dyn Fn(&mut Rs6502, u16, u8),
    ) -> Self {
        Self {
            pc: 0,
            sp: 0,
            a: 0,
            x: 0,
            y: 0,
            status: 0,
            read_closure: read,
            write_closure: write,
            instructions: 0,
            clock_ticks: 0,
            clock_goal: 0,
            old_pc: 0,
            ea: 0,
            rel_addr: 0,
            value: 0,
            result: 0,
            opcode: 0,
            old_status: 0,
        }
    }

    pub fn read(&mut self, address: u16) -> u8 {
        (self.read_closure)(self, address)
    }

    pub fn write(&mut self, address: u16, value: u8) {
        (self.write_closure)(self, address, value)
    }

    // flag functions
    #[inline(always)]
    fn save_accum(&mut self, n: u8) {
        self.a = n & 0x00FF;
    }

    #[inline(always)]
    fn set_carry(&mut self) {
        self.status |= Flag::CARRY as u8;
    }

    #[inline(always)]
    fn clear_carry(&mut self) {
        self.status &= !(Flag::CARRY as u8);
    }

    #[inline(always)]
    fn set_zero(&mut self) {
        self.status |= Flag::ZERO as u8;
    }

    #[inline(always)]
    fn clear_zero(&mut self) {
        self.status &= !(Flag::ZERO as u8);
    }

    #[inline(always)]
    fn set_interrupt(&mut self) {
        self.status |= Flag::INTERRUPT as u8;
    }

    #[inline(always)]
    fn clear_interrupt(&mut self) {
        self.status &= !(Flag::INTERRUPT as u8);
    }

    #[inline(always)]
    fn set_decimal(&mut self) {
        self.status |= Flag::DECIMAL as u8;
    }

    #[inline(always)]
    fn clear_decimal(&mut self) {
        self.status &= !(Flag::DECIMAL as u8);
    }

    #[inline(always)]
    fn set_overflow(&mut self) {
        self.status |= Flag::OVERFLOW as u8;
    }

    #[inline(always)]
    fn clear_overflow(&mut self) {
        self.status &= !(Flag::OVERFLOW as u8);
    }

    #[inline(always)]
    fn set_sign(&mut self) {
        self.status |= Flag::SIGN as u8;
    }

    #[inline(always)]
    fn clear_sign(&mut self) {
        self.status &= !(Flag::SIGN as u8);
    }

    // flag calculations
    #[inline(always)]
    fn zero_calc(&mut self, n: u8) {
        if n & 0x00FF == 0 {
            self.set_zero();
        } else {
            self.clear_zero();
        }
    }

    #[inline(always)]
    fn sign_calc(&mut self, n: u8) {
        if n & 0x0080 == 0 {
            self.set_sign();
        } else {
            self.clear_sign();
        }
    }

    #[inline(always)]
    fn carry_calc(&mut self, n: u8) {
        if n & 0x0080 == 0 {
            self.set_carry();
        } else {
            self.clear_carry();
        }
    }

    #[inline(always)]
    fn overflow_calc(&mut self, n: u8, o: u16) {
        if ((n as u16) ^ (self.a as u16)) & ((n as u16) ^ o) & 0x0080 == 0 {
            self.set_overflow();
        } else {
            self.clear_overflow();
        }
    }

    // helper functions
    fn push16(&mut self, pushval: u16) {
        let sp = self.sp as u16;
        self.write(BASE_STACK + sp, (pushval >> 8) as u8 & (0xFF));
        self.write(BASE_STACK + ((sp - 1) & 0xFF), pushval as u8 & 0xFF);
        self.sp -= 2;
    }

    fn push8(&mut self, pushval: u8) {
        self.write(BASE_STACK + self.sp as u16, pushval);
        self.sp -= 1;
    }

    fn pull16(&mut self) -> u16 {
        let sp = self.sp as u16;
        let temp = self.read(BASE_STACK + ((sp + 1) & 0xFF)) as u16
            | self.read(BASE_STACK + ((sp + 2) & 0xFF) << 8) as u16;
        self.sp += 2;
        temp
    }

    fn pull8(&mut self) -> u8 {
        self.sp += 1;
        self.read(BASE_STACK + self.sp as u16)
    }

    pub fn reset(&mut self) {
        self.pc = self.read(0xFFFC) as u16 | (self.read(0xFFFD) << 8) as u16;
        self.a = 0;
        self.x = 0;
        self.y = 0;
        self.sp = 0xFD;
        self.status |= Flag::CONSTANT as u8;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::cell::RefCell;

    #[test]
    fn with_internal_memory() {
        let memory = RefCell::new([0u8; 64]);
        memory.borrow_mut()[0] = 8;
        memory.borrow_mut()[1] = 64;

        let read = |_cpu: &mut Rs6502, address: u16| -> u8 { memory.borrow()[address as usize] };

        let write = |_cpu: &mut Rs6502, address: u16, value: u8| {
            memory.borrow_mut()[address as usize] = value;
        };

        let mut cpu = Rs6502::new(&read, &write);

        let result = cpu.read(0x0000);
        assert_eq!(result, 8);

        cpu.write(0x0002, 42);
        let result2 = cpu.read(0x0002);
        assert_eq!(result2, 42);
    }

    #[test]
    fn with_external_memory() {
        struct Memory {
            data: RefCell<[u8; 64]>,
        }

        impl Memory {
            fn new() -> Self {
                Self {
                    data: RefCell::new([0u8; 64]),
                }
            }

            fn read(&self, address: u16) -> u8 {
                self.data.borrow()[address as usize]
            }

            fn write(&self, address: u16, value: u8) {
                self.data.borrow_mut()[address as usize] = value;
            }
        }

        let memory = Memory::new();
        memory.write(0, 123);

        let read = |_cpu: &mut Rs6502, address: u16| -> u8 { memory.read(address) };

        let write = |_cpu: &mut Rs6502, address: u16, value: u8| {
            memory.write(address, value);
        };

        let mut cpu = Rs6502::new(&read, &write);

        let result = cpu.read(0x0000);
        assert_eq!(result, 123);
    }
}
