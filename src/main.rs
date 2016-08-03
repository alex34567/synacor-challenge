use std::io;
use std::fs::File;
use std::io::prelude::*;
use std::fmt;

struct Synacor {
    registers: [u16; 8],
    memory: [u16; 0x1FFFFF],
    stack: Vec<u16>,
    program_counter: u16,
    stdin: std::io::Stdin,
}

enum SynacorErr {
    Halted,
    BadRegister,
    StackUnderflow,
    BadOptcode,
    InputErr(io::Error),
}

impl fmt::Display for SynacorErr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            SynacorErr::Halted => write!(f, "The synacor halted."),
            SynacorErr::BadRegister => write!(f, "The synacor accessed a bad register."),
            SynacorErr::StackUnderflow => write!(f, "The synacor's stack underflowed."),
            SynacorErr::BadOptcode => write!(f, "The synacor's optcode is not implemented."),
            SynacorErr::InputErr(ref err) => write!(f, "{}", err),
        }
    }
}

impl Synacor {
    fn new() -> Synacor {
        Synacor {
            registers: [0; 8],
            memory: [0; 0x1FFFFF],
            stack: Vec::new(),
            program_counter: 0,
            stdin: io::stdin(),
        }
    }
    fn read_word_code(&mut self) -> u16 {
        self.program_counter += 1;
        self.memory[self.program_counter as usize - 1]
    }
    fn read_word_data(&mut self, location: u16) -> Result<u16, SynacorErr> {
        if location < 32768 {
            Ok(location)
        } else {
            let register = location % 32768;
            if register > 8 {
                Err(SynacorErr::BadRegister)
            } else {
                Ok(self.registers[register as usize])
            }
        }
    }
    fn read_bytes_into_ram(&mut self, bytes: &[u8]) {
        for i in bytes.iter().enumerate().zip(bytes.iter().skip(1)) {
            let ((mut index, byte1), byte2) = i;
            if index % 2 == 1 {
                continue;
            }
            index /= 2;
            let mut word = *byte2 as u16;
            word <<= 8;
            word |= *byte1 as u16;
            self.memory[index] = word;
        }
    }
    fn write_word_data(&mut self, location: u16, word: u16) -> Result<(), SynacorErr> {
        if location < 32768 {
            Ok(())
        } else {
            let register = location % 32768;
            if register > 8 {
                Err(SynacorErr::BadRegister)
            } else {
                self.registers[register as usize] = word;
                Ok(())
            }
        }
    }
    fn run_optcode(&mut self) -> Result<(), SynacorErr> {
        match self.read_word_code() {
            0 => Err(SynacorErr::Halted),
            1 => {
                let write_reg = self.read_word_code();
                let word_loc = self.read_word_code();
                let word = try!(self.read_word_data(word_loc));
                self.write_word_data(write_reg, word)
            }
            2 => {
                let location = self.read_word_code();
                let word = try!(self.read_word_data(location));
                self.stack.push(word);
                Ok(())
            }
            3 => {
                if let Some(word) = self.stack.pop() {
                    let location = self.read_word_code();
                    try!(self.write_word_data(location, word));
                    Ok(())
                } else {
                    Err(SynacorErr::StackUnderflow)
                }
            }
            4 => {
                let location_a = self.read_word_code();
                let location_b = self.read_word_code();
                let location_c = self.read_word_code();
                if try!(self.read_word_data(location_b)) == try!(self.read_word_data(location_c)) {
                    self.write_word_data(location_a, 1)
                } else {
                    self.write_word_data(location_a, 0)
                }
            }
            5 => {
                let location_a = self.read_word_code();
                let location_b = self.read_word_code();
                let location_c = self.read_word_code();
                if try!(self.read_word_data(location_b)) > try!(self.read_word_data(location_c)) {
                    self.write_word_data(location_a, 1)
                } else {
                    self.write_word_data(location_a, 0)
                }
            }
            6 => {
                let location = self.read_word_code();
                let jump = try!(self.read_word_data(location));
                self.program_counter = jump;
                Ok(())
            }
            7 => {
                let test_loc = self.read_word_code();
                let test = try!(self.read_word_data(test_loc));
                let jump_loc = self.read_word_code();
                if test != 0 {
                    let jump = try!(self.read_word_data(jump_loc));
                    self.program_counter = jump;
                    Ok(())
                } else {
                    Ok(())
                }
            }
            8 => {
                let test_loc = self.read_word_code();
                let test = try!(self.read_word_data(test_loc));
                let jump_loc = self.read_word_code();
                if test == 0 {
                    let jump = try!(self.read_word_data(jump_loc));
                    self.program_counter = jump;
                    Ok(())
                } else {
                    Ok(())
                }
            }
            9 => {
                let location_a = self.read_word_code();
                let location_b = self.read_word_code();
                let location_c = self.read_word_code();
                let b = try!(self.read_word_data(location_b));
                let c = try!(self.read_word_data(location_c));
                let mut sum = b.wrapping_add(c);
                sum %= 32768;
                self.write_word_data(location_a, sum)
            }
            10 => {
                let location_a = self.read_word_code();
                let location_b = self.read_word_code();
                let location_c = self.read_word_code();
                let b = try!(self.read_word_data(location_b));
                let c = try!(self.read_word_data(location_c));
                let mut prod = b.wrapping_mul(c);
                prod %= 32768;
                self.write_word_data(location_a, prod)
            }
            11 => {
                let location_a = self.read_word_code();
                let location_b = self.read_word_code();
                let location_c = self.read_word_code();
                let b = try!(self.read_word_data(location_b));
                let c = try!(self.read_word_data(location_c));
                let mut rem = b % c;
                rem %= 32768;
                self.write_word_data(location_a, rem)
            }
            12 => {
                let location_a = self.read_word_code();
                let location_b = self.read_word_code();
                let location_c = self.read_word_code();
                let b = try!(self.read_word_data(location_b));
                let c = try!(self.read_word_data(location_c));
                let and = b & c;
                self.write_word_data(location_a, and)
            }
            13 => {
                let location_a = self.read_word_code();
                let location_b = self.read_word_code();
                let location_c = self.read_word_code();
                let b = try!(self.read_word_data(location_b));
                let c = try!(self.read_word_data(location_c));
                let or = b | c;
                self.write_word_data(location_a, or)
            }
            14 => {
                let location_a = self.read_word_code();
                let location_b = self.read_word_code();
                let b = try!(self.read_word_data(location_b));
                let b_inv = b ^ 0x7FFF;
                self.write_word_data(location_a, b_inv)
            }
            15 => {
                let location_a = self.read_word_code();
                let location_b = self.read_word_code();
                let b = try!(self.read_word_data(location_b));
                let a = self.memory[b as usize];
                self.write_word_data(location_a, a)
            }
            16 => {
                let location_a = self.read_word_code();
                let location_b = self.read_word_code();
                let a = try!(self.read_word_data(location_a));
                let b = try!(self.read_word_data(location_b));
                self.memory[a as usize] = b;
                Ok(())
            }
            17 => {
                let location_a = self.read_word_code();
                let a = try!(self.read_word_data(location_a));
                self.stack.push(self.program_counter);
                self.program_counter = a;
                Ok(())
            }
            18 => {
                if let Some(jump) = self.stack.pop() {
                    self.program_counter = jump;
                    Ok(())
                } else {
                    Err(SynacorErr::StackUnderflow)
                }
            }
            19 => {
                let location = self.read_word_code();
                let char = try!(self.read_word_data(location)) as u8 as char;
                print!("{}", char);
                Ok(())
            }
            20 => {
                ;
                let location_a = self.read_word_code();
                let mut char_buf = [0; 1];
                if let Err(err) = self.stdin.lock().read(&mut char_buf) {
                    return Err(SynacorErr::InputErr(err))
                }
                let char16 = char_buf[0] as u16;
                self.write_word_data(location_a, char16)
            }
            21 => Ok(()),
            _ => Err(SynacorErr::BadOptcode),
        }
    }
}

fn main() {
    let mut input_file = File::open("challenge.bin").unwrap();
    let mut input_bytes = Vec::new();
    input_file.read_to_end(&mut input_bytes).unwrap();
    let mut synacor = Synacor::new();
    synacor.read_bytes_into_ram(&input_bytes);
    loop {
        if let Err(error) = synacor.run_optcode() {
            println!("{}", error);
            break;
        }
    }
}
