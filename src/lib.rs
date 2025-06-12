//! 6502 embeddable emulator.
//!
//! Provides a struct that simulates the 6502 processor.
//! You can control how it reads and writes by providing closures in the new() function.

#![no_std]

pub const BASE_STACK: u16 = 0x100;

#[repr(u8)]
pub enum Flags {
    CARRY = 0x01,
    ZERO = 0x02,
    INTERUPT = 0x04,
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
        }
    }

    pub fn read(&mut self, address: u16) -> u8 {
        (self.read_closure)(self, address)
    }

    pub fn write(&mut self, address: u16, value: u8) {
        (self.write_closure)(self, address, value)
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
