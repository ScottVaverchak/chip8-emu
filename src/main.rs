const FBWIDTH: usize = 64;
const FBHEIGHT: usize = 32;

struct Framebuffer { 
    memory: [bool; FBWIDTH * FBHEIGHT],
}

impl Framebuffer { 
    fn new() -> Self { 
        Self { 
            memory: [false; FBWIDTH * FBHEIGHT],
        }
    }

    pub fn clear(&mut self) { 
        self.memory.fill(false);
    }

    fn xy(&mut self, x: usize, y: usize, value: bool) { 
        assert!(x < FBWIDTH, "X is larger than allowed FBWIDTH");
        assert!(y < FBHEIGHT, "Y is larger than allows FBHEIGHT");

        self.memory[y * FBWIDTH + x] ^= value;
    }

    fn get_xy(&self, x: usize, y: usize) -> bool { 
        self.memory[(y * FBWIDTH) + x]
    }

    fn dump(&self) { 
        for y in 0..FBHEIGHT - 1 { 
            for x in 0..FBWIDTH - 1 { 
                let pixel = self.get_xy(x, y);
                
                match pixel { 
                    true => print!("#"),
                    false => print!(" ")
                }
            }

            println!();
        }
    }
}

struct Chip8Emulator { 
    // @TODO(svavs): Probably want to break the emulator up into smaller
    //               units - Memory, Registers, etc...
    
    // Registers
    registers: [u8; 16],
    index_register: u16,

    // @TODO(svavs): Using a Vec<T>, but I think this should be its own 
    //               impl sooner than later. This will require a lot 
    //               of handholding
    // Stack
    stack: Vec<u16>,
    stack_pointer: u8, 

    // Timers
    delay_timer: u8, 
    sound_timer: u8,

    // Framebuffer
    framebuffer: Framebuffer, //[bool; 64 * 32],

    // PC 
    program_counter: u16,

    // Memory
    memory: [u8; 4096],

}

impl Chip8Emulator { 
    fn new() -> Self { 
        Self { 
            registers: [0; 16],
            index_register: 0, 
            stack: Vec::new(),
            stack_pointer: 0,

            delay_timer: 0,
            sound_timer: 0,

            framebuffer: Framebuffer::new(),
            program_counter: 0, 
            
            memory: [0; 4096]
        }
    }

    fn load_program(&mut self, program: Vec<u8>) { 
        println!("Program Length: {prog_len}", prog_len = program.len());

        let mut pointer = 0x200;
        for data in program { 
            self.memory[pointer] = data;
            pointer += 1;
        }

        self.program_counter = 0x200;
    }

    fn run(&mut self) { 
        loop {

            match self.step() { 
                Err(err) => { 
                    eprintln!("{err}");
                    return;
                },
                _ => {},
            }
        }
    }

    fn run_for(&mut self, count: usize) { 
        for _ in 0..count {
            match self.step() { 
                Err(err) => { 
                    eprintln!("{err}");
                    return;
                },
                _ => {},
            }
        }
    }

    fn step(&mut self) -> Result<(), &'static str> { 
        if self.program_counter as usize >= self.memory.len() { 
            return Err("Program counter ran past top of memory.");
        }

        let b0 = self.memory[self.program_counter as usize];
        let b1 = self.memory[self.program_counter as usize + 1];
        self.program_counter += 2;
        
        // println!("0x{:02x}{:02x}", b0, b1);

        let inst = b0 >> 4;

        match inst { 
            0x0 => { 
                match (b0, b1) {
                    (0x00, 0xE0) => self.framebuffer.clear(),
                    (0x00, 0xEE) => { 
                        let Some(pc) = self.stack.pop() else { 
                            eprintln!("Stack underflow!");
                            std::process::exit(0);
                        };
                        self.program_counter = pc;
                        self.stack_pointer -= 1;
                    },
                    (_, _) => println!("SYS (Ignored)"),
                };

            },
            0x1 => {
                let lb = b0 & 0xF;
                let addr = ((lb as u16) << 8) | (b1 as u16);
                self.program_counter = addr;
            },
            0x2 => { 
                self.stack.push(self.program_counter);
                self.stack_pointer += 1;

                let lb = b0 & 0xF;
                let addr = ((lb as u16) << 8) | (b1 as u16);
                self.program_counter = addr;
            },
            0x3 => { 
                let lb = b0 & 0xF;

                if self.registers[lb as usize] == b1 { 
                    self.program_counter += 2;
                }
                
            },
            0x4 => {
                let lb = b0 & 0xF;

                if self.registers[lb as usize] != b1 { 
                    self.program_counter += 2;
                }
            },
            0x5 => { 
                let x = b0 & 0xF;
                let y = b1 >> 4;

                if self.registers[x as usize] == self.registers[y as usize] { 
                    self.program_counter += 2;
                }

            },
            0x6 => { 
                let x = b0 & 0xF;
                self.registers[x as usize] = b1;
            },
            0x7 => { 
                let x = b0 & 0xF;
                self.registers[x as usize] += b1;
            },
            0x8 => {
                let inins = b1 & 0xF;
                let x = (b0 & 0xF) as usize;
                let y = (b1 >> 4) as usize;

                match inins { 
                    0x0 => self.registers[x] = self.registers[y],
                    0x1 => self.registers[x] |= self.registers[y],
                    0x2 => self.registers[x] &= self.registers[y],
                    0x3 => self.registers[x] ^= self.registers[y],
                    0x4 => {
                        let output = self.registers[x] as u16 + self.registers[y] as u16;

                        self.registers[0xF] = if output > 255 { 1 } else { 0 };
                        self.registers[x] = (output & 0xFF) as u8;
                    },
                    0x5 => {
                        let vx = self.registers[x];
                        let vy = self.registers[y];

                        self.registers[0xF] = if vx > vy { 1 } else { 0 };
                        self.registers[x] = vx - vy;
                    },
                    0x6 => {
                        self.registers[x] = self.registers[x] >> 1;
                        self.registers[0xF] = if self.registers[x] & 1 == 1 { 1 } else { 0 };
                    },
                    0x7 => {
                        let vx = self.registers[x];
                        let vy = self.registers[y];

                        self.registers[0xF] = if vy > vx { 1 } else { 0 };
                        self.registers[x] = vy - vx;
                    },
                    0xE => { 
                        self.registers[x] = self.registers[x] << 1;
                        self.registers[0xF] = if self.registers[x] >> 7 == 1 { 1 } else { 0 };
                    },
                    _ => {},

                };
            },
            0x9 => { 
                let x = b0 & 0xF;
                let y = b1 >> 4;

                if self.registers[x as usize] != self.registers[y as usize] { 
                    self.program_counter += 2;
                }
            },
            0xA =>  { 
                let lb = b0 & 0xF;
                let addr = ((lb as u16) << 8) | (b1 as u16);
                self.index_register = addr;

            },
            0xB => { 
                let lb = b0 & 0xF;
                let addr = ((lb as u16) << 8) | (b1 as u16);
                let v0 = self.registers[0x0];
                self.program_counter = addr + v0 as u16;
            },
            0xC => {
                unimplemented!("INST C");
            },
            0xD => { 
                let x = b0 & 0xF;
                let y = b1 >> 4;
                let n = b1 & 0xF;

                let vx = self.registers[x as usize] & 63;
                let vy = self.registers[y as usize] & 31;

                self.registers[0xF] = 0;

                for y in 0..n { 
                    if vy + y > 31 { break; }                  

                    let data = self.memory[(self.index_register + y as u16) as usize];

                    for x in 0..8 { 
                        if (data & (0x80 >> x)) != 0 { 
                            if vx + x > 63 { 
                                break;
                            }

                            if self.framebuffer.get_xy((vx + x) as usize, (vy + y) as usize) { 
                                self.registers[0xF] = 1;
                            }

                            self.framebuffer.xy((vx + x) as usize, (vy + y) as usize, true);
                        }

                    }
                }
                
            },
            0xE => println!("INST E"),
            0xF => println!("INST F"),
            _ => println!(":)")
        };

        Ok(())
    }

}


fn main() {
    let Ok(program) = std::fs::read("./1-chip8-logo.ch8") else { return; };

    let mut emu = Chip8Emulator::new();
    emu.load_program(program);

    emu.run_for(50);

    emu.framebuffer.dump();
}
