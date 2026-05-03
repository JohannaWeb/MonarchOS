extern crate alloc;

use alloc::string::String;
use core::arch::asm;

pub struct Terminal {
    addr: *mut u32,
    width: usize,
    height: usize,
    pitch: usize,
    cursor_x: usize,
    cursor_y: usize,
    scale: usize,
    shift: bool,
}

impl Terminal {
    pub fn new(addr: *mut u32, width: usize, height: usize, pitch: usize, scale: usize) -> Self {
        Self {
            addr,
            width,
            height,
            pitch,
            cursor_x: 0,
            cursor_y: 0,
            scale,
            shift: false,
        }
    }

    pub fn clear(&mut self) {
        for y in 0..self.height {
            for x in 0..self.width {
                unsafe {
                    self.addr
                        .add(y * (self.pitch / 4) + x)
                        .write_volatile(0xFF_000000);
                }
            }
        }
        self.cursor_x = 0;
        self.cursor_y = 0;
    }

    pub fn print_char(&mut self, c: char) {
        if c == '\n' {
            self.cursor_x = 0;
            self.cursor_y += 8 * self.scale;
            return;
        }

        if c == '\x08' {
            // Backspace
            if self.cursor_x >= 8 * self.scale {
                self.cursor_x -= 8 * self.scale;
                // Clear character
                for sy in 0..8 * self.scale {
                    for sx in 0..8 * self.scale {
                        let px = self.cursor_x + sx;
                        let py = self.cursor_y + sy;
                        if px < self.width && py < self.height {
                            unsafe {
                                self.addr
                                    .add(py * (self.pitch / 4) + px)
                                    .write_volatile(0xFF_000000);
                            }
                        }
                    }
                }
            } else if self.cursor_y >= 8 * self.scale {
                // simple backspace wrapping
                self.cursor_y -= 8 * self.scale;
                self.cursor_x = self.width - 8 * self.scale;
            }
            return;
        }

        if (c as u32) < 128 {
            let glyph = font8x8::legacy::BASIC_LEGACY[c as usize];
            for row_idx in 0..8 {
                let row = glyph[row_idx];
                for bit_idx in 0..8 {
                    if (row >> bit_idx) & 1 == 1 {
                        for sy in 0..self.scale {
                            for sx in 0..self.scale {
                                let px = self.cursor_x + bit_idx * self.scale + sx;
                                let py = self.cursor_y + row_idx * self.scale + sy;
                                if px < self.width && py < self.height {
                                    unsafe {
                                        self.addr
                                            .add(py * (self.pitch / 4) + px)
                                            .write_volatile(0xFF_00FF00); // Green text!
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        self.cursor_x += 8 * self.scale;
        if self.cursor_x + 8 * self.scale > self.width {
            self.cursor_x = 0;
            self.cursor_y += 8 * self.scale;
        }
        if self.cursor_y + 8 * self.scale > self.height {
            // Scroll not implemented, just wrap to top for now
            self.clear();
        }
    }

    pub fn print(&mut self, s: &str) {
        for c in s.chars() {
            self.print_char(c);
        }
    }

    unsafe fn inb(port: u16) -> u8 {
        let mut value: u8;
        asm!("in al, dx", out("al") value, in("dx") port, options(nomem, nostack, preserves_flags));
        value
    }

    pub fn get_char(&mut self) -> char {
        const SCANCODE_TO_ASCII: [u8; 122] = [
            0, 27, b'1', b'2', b'3', b'4', b'5', b'6', b'7', b'8', b'9', b'0', b'-', b'=', 8,
            b'\t', b'q', b'w', b'e', b'r', b't', b'y', b'u', b'i', b'o', b'p', b'[', b']', b'\n',
            0, b'a', b's', b'd', b'f', b'g', b'h', b'j', b'k', b'l', b';', b'\'', b'`', 0, b'\\',
            b'z', b'x', b'c', b'v', b'b', b'n', b'm', b',', b'.', b'/', 0, b'*', 0, b' ', 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0,
        ];
        const SCANCODE_TO_ASCII_SHIFT: [u8; 122] = [
            0, 27, b'!', b'@', b'#', b'$', b'%', b'^', b'&', b'*', b'(', b')', b'_', b'+', 8,
            b'\t', b'Q', b'W', b'E', b'R', b'T', b'Y', b'U', b'I', b'O', b'P', b'{', b'}', b'\n',
            0, b'A', b'S', b'D', b'F', b'G', b'H', b'J', b'K', b'L', b':', b'"', b'~', 0, b'|',
            b'Z', b'X', b'C', b'V', b'B', b'N', b'M', b'<', b'>', b'?', 0, b'*', 0, b' ', 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0,
        ];

        let mut last_scancode = 0;

        loop {
            unsafe {
                if Self::inb(0x64) & 1 == 1 {
                    let scancode = Self::inb(0x60);
                    if scancode == last_scancode {
                        continue;
                    }
                    last_scancode = scancode;

                    if scancode == 0x2A || scancode == 0x36 {
                        self.shift = true;
                    } else if scancode == 0xAA || scancode == 0xB6 {
                        self.shift = false;
                    } else if scancode < 122 {
                        let c = if self.shift {
                            SCANCODE_TO_ASCII_SHIFT[scancode as usize]
                        } else {
                            SCANCODE_TO_ASCII[scancode as usize]
                        };
                        if c != 0 {
                            return c as char;
                        }
                    }
                }
            }
        }
    }

    pub fn run_shell(&mut self) {
        self.clear();
        self.print("Welcome to MonarchOS!\n");
        let mut input = String::new();

        loop {
            self.print("> ");
            input.clear();
            loop {
                let c = self.get_char();
                if c == '\n' {
                    self.print_char('\n');
                    break;
                } else if c == '\x08' {
                    if !input.is_empty() {
                        input.pop();
                        self.print_char(c);
                    }
                } else {
                    input.push(c);
                    self.print_char(c);
                }
            }

            if input == "help" {
                self.print("Available commands:\n  help  - Show this help message\n  echo  - Echo back input\n  clear - Clear the screen\n  reboot- Reboot the system\n");
            } else if input.starts_with("echo ") {
                self.print(&input[5..]);
                self.print("\n");
            } else if input == "clear" {
                self.clear();
            } else if input == "reboot" {
                self.print("Rebooting...\n");
                unsafe {
                    let mut good = 0x02;
                    while good & 0x02 != 0 {
                        good = Self::inb(0x64);
                    }
                    asm!("out dx, al", in("dx") 0x64_u16, in("al") 0xFE_u8, options(nomem, nostack, preserves_flags));
                }
            } else if !input.is_empty() {
                self.print("Unknown command: ");
                self.print(&input);
                self.print("\n");
            }
        }
    }
}
