use super::carry_borrow::{AddCarry, ShiftOverflow, SubBorrow};
use super::insts::Chip8Inst;
use super::{Chip8Machine, DISPLAY_COLS, DISPLAY_ROWS, FONT_BASE};
use std::sync::atomic::Ordering;
use termion::event::Key;
use termion::input::TermRead;
use std::io::stdin;

impl Chip8Machine {
    fn get_keydown(&self) -> Option<u8> {
        let mut keys = stdin().keys();
        let k = keys.nth(0).unwrap().unwrap();
        return match k {
            Key::Char('1') => Some(0x1),
            Key::Char('2') => Some(0x2),
            Key::Char('3') => Some(0x3),
            Key::Char('4') => Some(0xc),
            Key::Char('q') => Some(0x4),
            Key::Char('w') => Some(0x5),
            Key::Char('e') => Some(0x6),
            Key::Char('r') => Some(0xd),
            Key::Char('a') => Some(0x7),
            Key::Char('s') => Some(0x8),
            Key::Char('d') => Some(0x9),
            Key::Char('f') => Some(0xe),
            Key::Char('z') => Some(0xa),
            Key::Char('x') => Some(0x0),
            Key::Char('c') => Some(0xb),
            Key::Char('v') => Some(0xf),
            _ => None,
        };
    }

    pub(super) fn execute(&mut self, inst: Chip8Inst) {
        match inst {
            Chip8Inst::MachineInst => (),
            Chip8Inst::ClearScreen => {
                for mut row in self.display {
                    row.fill(false);
                }
                self.print_display();
            }
            Chip8Inst::SubCall(n) => {
                self.stack.push(self.prog_counter);
                self.prog_counter = n;
            }
            Chip8Inst::SubReturn => match self.stack.pop() {
                Some(n) => self.prog_counter = n,
                None => panic!("Tried to pop an empty stack"),
            },
            Chip8Inst::Jump(n) => self.prog_counter = n,
            Chip8Inst::SetIndex(n) => self.index_reg = n,
            Chip8Inst::AddIndex(x) => {
                self.index_reg += self.registers[x] as usize;
                if self.index_reg >= 0x1000 {
                    self.registers[0xf] = 1;
                }
            }
            Chip8Inst::RegSet(x, n) => self.registers[x] = n,
            Chip8Inst::RegAddNoCarry(x, n) => {
                let m = self.registers[x];
                self.registers[x] = u8::add_no_carry(m, n);
            }
            Chip8Inst::SkipEqConst(x, n) => {
                if self.registers[x] == n {
                    self.prog_counter += 2;
                }
            }
            Chip8Inst::SkipNeqConst(x, n) => {
                if self.registers[x] != n {
                    self.prog_counter += 2;
                }
            }
            Chip8Inst::SkipEqReg(x, y) => {
                if self.registers[x] == self.registers[y] {
                    self.prog_counter += 2;
                }
            }
            Chip8Inst::SkipNeqReg(x, y) => {
                if self.registers[x] != self.registers[y] {
                    self.prog_counter += 2;
                }
            }
            Chip8Inst::Assign(x, y) => self.registers[x] = self.registers[y],
            Chip8Inst::BinOr(x, y) => self.registers[x] |= self.registers[y],
            Chip8Inst::BinAnd(x, y) => self.registers[x] &= self.registers[y],
            Chip8Inst::BinXor(x, y) => self.registers[x] ^= self.registers[y],
            Chip8Inst::ArithAdd(x, y) => {
                let (sum, carry) = u8::add_carry(self.registers[x], self.registers[y]);
                self.registers[x] = sum;
                self.registers[0xf] = if carry { 1 } else { 0 }
            }
            Chip8Inst::ArithSub(x, y) => {
                let (diff, borrow) = u8::sub_borrow(self.registers[x], self.registers[y]);
                self.registers[x] = diff;
                self.registers[0xf] = if borrow { 1 } else { 0 }
            }
            Chip8Inst::ArithSubReverse(x, y) => {
                let (diff, borrow) = u8::sub_borrow(self.registers[y], self.registers[x]);
                self.registers[x] = diff;
                self.registers[0xf] = if borrow { 1 } else { 0 }
            }
            Chip8Inst::ReadDelay(x) => {
                let n = self.delay_timer.load(Ordering::Acquire);
                self.registers[x] = n;
            }
            Chip8Inst::SetDelay(x) => {
                self.delay_timer.store(self.registers[x], Ordering::Release);
            }
            Chip8Inst::SetSound(x) => {
                self.sound_timer.store(self.registers[x], Ordering::Release);
            }
            Chip8Inst::Display(x_reg, y_reg, n) => {
                let mut x = (self.registers[x_reg] & 63) as usize;
                let mut y = (self.registers[y_reg] & 31) as usize;
                self.registers[0xf] = 0;

                for i in 0..n {
                    let b = self.memory[self.index_reg + i as usize];
                    let bs = [
                        b & 0x80,
                        b & 0x40,
                        b & 0x20,
                        b & 0x10,
                        b & 0x8,
                        b & 0x4,
                        b & 0x2,
                        b & 0x1,
                    ];
                    for j in 0..8 {
                        if self.display[y - 1][x - 1] && bs[j] != 0 {
                            self.display[y - 1][x - 1] = false;
                        } else if !self.display[y - 1][x - 1] && bs[j] != 0 {
                            self.display[y - 1][x - 1] = true;
                        }

                        x += 1;
                        if x - 2 >= DISPLAY_COLS {
                            break;
                        }
                    }
                    y += 1;
                    if y - 2 >= DISPLAY_ROWS {
                        break;
                    }
                    x = (self.registers[x_reg] & 63) as usize;
                }
                self.print_display();
            }
            Chip8Inst::Random(x, n) => {
                let r = rand::random::<u8>();
                self.registers[x] = n & r;
            }
            Chip8Inst::ShiftLeft(x, y) => {
                let n = self.registers[y];
                let (n1, overflow) = u8::shift_left(n, 1);
                self.registers[x] = n1;
                self.registers[0xf] = if overflow { 1 } else { 0 };
            }
            Chip8Inst::ShiftRight(x, y) => {
                let n = self.registers[y];
                let (n1, underflow) = u8::shift_right(n, 1);
                self.registers[x] = n1;
                self.registers[0xf] = if underflow { 1 } else { 0 };
            }
            Chip8Inst::SkipEqKey(x) => match self.get_keydown() {
                None => (),
                Some(k) => {
                    if self.registers[x] == k {
                        self.prog_counter += 2;
                    }
                }
            },
            Chip8Inst::SkipNeqKey(x) => match self.get_keydown() {
                None => (),
                Some(k) => {
                    if self.registers[x] != k {
                        self.prog_counter += 2;
                    }
                }
            },
            Chip8Inst::GetKey(x) => loop {
                match self.get_keydown() {
                    None => (),
                    Some(k1) => {
                        self.registers[x] = k1;
                        break;
                    }
                }
            },
            Chip8Inst::LoadFont(x) => {
                let c = self.registers[x];
                self.index_reg = FONT_BASE + c as usize;
            }
            Chip8Inst::BCDConvert(x) => {
                let n = self.registers[x];
                self.memory[self.index_reg] = n / 100;
                self.memory[self.index_reg + 1] = (n % 100) / 10;
                self.memory[self.index_reg + 2] = n % 10;
            }
            Chip8Inst::StoreMem(x) => {
                for i in 0..x + 1 {
                    self.memory[self.index_reg + i] = self.registers[i];
                }
            }
            Chip8Inst::LoadMem(x) => {
                for i in 0..x + 1 {
                    self.registers[i] = self.memory[self.index_reg + i]
                }
            }
        }
    }
}
