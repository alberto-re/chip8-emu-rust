pub mod display;
mod sprites;

use display::Display;
use rand::Rng;
use sprites::FONT_SPRITES;
use sprites::FONT_SPRITES_MEM_ADDR;
use sprites::FONT_SPRITE_LEN;

const RAM_SIZE: usize = 4096;

pub struct Chip8 {
    pub display: Display,
    pub ram: [u8; RAM_SIZE],
    pub pc: u16,
    stack: Vec<u16>,
    reg_i: u16,
    reg_v: [u8; 16],
    delay_timer: u8,
    sound_timer: u8,
    keyboard: [bool; 16],
    paused: bool,
    store_keypress_in_reg: u8,
}

impl Chip8 {
    pub fn new() -> Self {
        let mut emu = Self {
            display: Display::new(),
            ram: [0; RAM_SIZE],
            pc: 0,
            stack: Vec::new(),
            reg_i: 0,
            reg_v: [0; 16],
            delay_timer: 0,
            sound_timer: 0,
            keyboard: [false; 16],
            paused: false,
            store_keypress_in_reg: 0,
        };
        emu.load_sprites();
        emu
    }

    fn load_sprites(&mut self) {
        for (index, sprite) in FONT_SPRITES.iter().enumerate() {
            let start_addr = FONT_SPRITES_MEM_ADDR + (index * sprite.len());
            let end_addr = start_addr + sprite.len();
            self.ram[start_addr..end_addr].copy_from_slice(FONT_SPRITES[index].as_slice());
        }
    }

    fn pause_until_keypress(&mut self, reg: u8) {
        self.paused = true;
        self.store_keypress_in_reg = reg;
    }

    pub fn beep(&mut self) -> bool {
        self.sound_timer > 0
    }

    pub fn key_pressed(&mut self, key: u8, state: bool) {
        if self.paused {
            self.reg_v[self.store_keypress_in_reg as usize] = key;
            self.paused = false;
        }
        self.keyboard[key as usize] = state;
    }

    pub fn load(&mut self, data: &[u8]) {
        let start: usize = 0x200;
        let end = start + data.len();
        self.ram[start..end].copy_from_slice(data);
        self.pc = 0x200;
    }

    pub fn fetch_execute(&mut self) {
        if self.paused {
            return;
        };
        let opcode = self.fetch();
        self.execute(opcode);
    }

    pub fn dec_timers(&mut self) {
        if self.delay_timer > 0 {
            self.delay_timer -= 1;
        }
        if self.sound_timer > 0 {
            self.sound_timer -= 1;
        }
    }

    fn fetch(&mut self) -> u16 {
        let hbyte = self.ram[self.pc as usize] as u16;
        let lbyte = self.ram[(self.pc + 1) as usize] as u16;
        let opcode = (hbyte << 8) | lbyte;
        self.pc += 2;
        opcode
    }

    fn execute(&mut self, opcode: u16) {
        let digits = (
            opcode >> 12,
            (opcode & 0x0F00) >> 8,
            (opcode & 0x00F0) >> 4,
            opcode & 0x000F,
        );
        match digits {
            // 00E0 - Clear the display
            (0x0, 0x0, 0xE, 0x0) => {
                self.display.clear();
            }
            // 00EE - Return from a subroutine
            (0x0, 0x0, 0xE, 0xE) => {
                self.pc = self.stack.pop().unwrap();
            }
            // 1nnn - Jump to location nnn
            (0x1, _, _, _) => {
                let nnn = opcode & 0x0FFF;
                self.pc = nnn;
            }
            // 2nnn - Call subroutine at nnn
            (0x2, _, _, _) => {
                let nnn = opcode & 0x0FFF;
                self.stack.push(self.pc);
                self.pc = nnn;
            }
            // 3xkk - Skip next instruction if Vx = kk
            (0x3, x, _, _) => {
                let nn = (opcode & 0x00FF) as u8;
                let vx = self.reg_v[x as usize];
                if vx == nn {
                    self.pc += 2;
                }
            }
            // 4xkk - Skip next instruction if Vx != kk
            (0x4, x, _, _) => {
                let nn = (opcode & 0x00FF) as u8;
                let vx = self.reg_v[x as usize];
                if vx != nn {
                    self.pc += 2;
                }
            }
            // 5xy0 - Skip next instruction if Vx = Vy
            (0x5, x, y, 0x0) => {
                let vx = self.reg_v[x as usize];
                let vy = self.reg_v[y as usize];
                if vx == vy {
                    self.pc += 2;
                }
            }
            // 6xy0 - Set Vx = kk
            (0x6, x, _, _) => {
                let nn = (opcode & 0x00FF) as u8;
                self.reg_v[x as usize] = nn;
            }
            // 7xkk - Set Vx = Vx + kk
            (0x7, x, _, _) => {
                let nn = (opcode & 0x00FF) as u8;
                self.reg_v[x as usize] = self.reg_v[x as usize].wrapping_add(nn);
            }
            // 8xy0 - Set Vx = Vy
            (0x8, x, y, 0x0) => {
                self.reg_v[x as usize] = self.reg_v[y as usize];
            }
            // 8xy1 - Set Vx = Vx OR Vy
            (0x8, x, y, 0x1) => {
                self.reg_v[x as usize] |= self.reg_v[y as usize];
            }
            // 8xy2 - Vy - Set Vx = Vx AND Vy
            (0x8, x, y, 0x2) => {
                self.reg_v[x as usize] &= self.reg_v[y as usize];
            }
            // 8xy3 - Set Vx = Vx XOR Vy
            (0x8, x, y, 0x3) => {
                self.reg_v[x as usize] ^= self.reg_v[y as usize];
            }
            // 8xy4 - Set Vx = Vx + Vy, set VF = carry
            (0x8, x, y, 0x4) => {
                let (result, carry) =
                    self.reg_v[x as usize].overflowing_add(self.reg_v[y as usize]);
                self.reg_v[x as usize] = result;
                self.reg_v[0xF] = if carry { 0x1 } else { 0x0 };
            }
            // 8xy5 - Set Vx = Vx - Vy, set VF = NOT borrow
            (0x8, x, y, 0x5) => {
                let (result, borrow) =
                    self.reg_v[x as usize].overflowing_sub(self.reg_v[y as usize]);
                self.reg_v[x as usize] = result;
                self.reg_v[0xF] = if borrow { 0x0 } else { 0x1 };
            }
            // 8xy6 - Set Vx = Vx SHR 1
            (0x8, x, _, 0x6) => {
                let lsb = self.reg_v[x as usize] & 0b00000001;
                self.reg_v[x as usize] >>= 1;
                self.reg_v[0xF] = lsb;
            }
            // 8xy7 - Set Vx = Vy - Vx, set VF = NOT borrow
            (0x8, x, y, 0x7) => {
                let (result, borrow) =
                    self.reg_v[y as usize].overflowing_sub(self.reg_v[x as usize]);
                self.reg_v[x as usize] = result;
                self.reg_v[0xF] = if borrow { 0x0 } else { 0x1 };
            }
            // 8xyE - Set Vx = Vx SHL 1
            (0x8, x, _, 0xE) => {
                let msb = (self.reg_v[x as usize] & 0b10000000) >> 7;
                self.reg_v[x as usize] <<= 1;
                self.reg_v[0xF] = msb;
            }
            // 9xy0 - Skip next instruction if Vx != Vy
            (0x9, x, y, 0x0) => {
                let vx = self.reg_v[x as usize];
                let vy = self.reg_v[y as usize];
                if vx != vy {
                    self.pc += 2;
                }
            }
            // Annn - Set I = nnn
            (0xA, _, _, _) => {
                let nnn = opcode & 0x0FFF;
                self.reg_i = nnn;
            }
            // Bnnn - Jump to location nnn + V0
            (0xB, _, _, _) => {
                let nnn = opcode & 0x0FFF;
                self.reg_i = nnn + self.reg_v[0x0] as u16;
            }
            // Cxnn - Set Vx = random byte AND nn
            (0xC, x, _, _) => {
                let nn = opcode & 0x00FF;
                self.reg_v[x as usize] = (rand::thread_rng().gen_range(0..256) & nn) as u8;
            }
            // Dxyn - Display n-byte sprite starting at memory location I at (Vx, Vy), set VF = collision
            (0xD, x, y, n) => {
                let vx = self.reg_v[x as usize];
                let vy = self.reg_v[y as usize];
                let addr_start = self.reg_i as usize;
                let addr_end = addr_start + n as usize;
                let sprite: Vec<u8> = self.ram[addr_start..addr_end].to_vec();
                let unset = self.display.draw(sprite, vx, vy);
                self.reg_v[0xF] = if unset { 1 } else { 0 }
            }
            // Ex9E - Display n-byte sprite starting at memory location I at (Vx, Vy), set VF = collision
            (0xE, x, 0x9, 0xE) => {
                let vx = self.reg_v[x as usize] as usize;
                if self.keyboard[vx] {
                    self.pc += 2;
                }
            }
            // ExA1 - Skip next instruction if key with the value of Vx is not pressed
            (0xE, x, 0xA, 0x1) => {
                let vx = self.reg_v[x as usize] as usize;
                if !self.keyboard[vx] {
                    self.pc += 2;
                }
            }
            // Fx07 - Set Vx = delay timer value
            (0xF, x, 0x0, 0x7) => {
                self.reg_v[x as usize] = self.delay_timer;
            }
            // Fx0A - Wait for a key press, store the value of the key in Vx
            (0xF, x, 0x0, 0xA) => {
                self.pause_until_keypress(x as u8);
            }
            // Fx15 - Set delay timer = Vx
            (0xF, x, 0x1, 0x5) => {
                self.delay_timer = self.reg_v[x as usize];
            }
            // Fx18 - Set sound timer = Vx
            (0xF, x, 0x1, 0x8) => {
                self.sound_timer = self.reg_v[x as usize];
            }
            // Fx1E - The values of I and Vx are added, and the results are stored in I
            (0xF, x, 0x1, 0xE) => {
                let vx = self.reg_v[x as usize];
                self.reg_i += vx as u16;
            }
            // Fx29 - Set I = location of sprite for digit Vx
            (0xF, x, 0x2, 0x9) => {
                self.reg_i = (FONT_SPRITES_MEM_ADDR + FONT_SPRITE_LEN * x as usize) as u16;
            }
            // Fx33 - Store BCD representation of Vx in memory locations I, I+1, and I+2
            (0xF, x, 0x3, 0x3) => {
                let vx = self.reg_v[x as usize];
                let hundreds: u8 = vx / 100;
                let tens: u8 = (vx % 100) / 10;
                let units: u8 = vx % 10;
                self.ram[self.reg_i as usize] = hundreds;
                self.ram[(self.reg_i + 1) as usize] = tens;
                self.ram[(self.reg_i + 2) as usize] = units;
            }
            // Fx55 - Store registers V0 through Vx in memory starting at location I
            (0xF, x, 0x5, 0x5) => {
                for i in 0..x + 1 {
                    let to_i = (self.reg_i + i) as usize;
                    self.ram[to_i] = self.reg_v[i as usize];
                }
            }
            // Fx65 - Read registers V0 through Vx from memory starting at location I
            (0xF, x, 0x6, 0x5) => {
                for i in 0..x + 1 {
                    let from_i = (self.reg_i + i) as usize;
                    self.reg_v[i as usize] = self.ram[from_i];
                }
            }
            (_, _, _, _) => {
                panic!("unimplemented {:#06x}", opcode);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::chip8::display::RES_WIDTH;
    use crate::chip8::Chip8;
    use crate::chip8::FONT_SPRITES_MEM_ADDR;
    #[test]
    fn loaded_data_is_in_memory() {
        let mut emu = Chip8::new();
        let data: [u8; 4] = [0xA, 0x1, 0xF, 0x12];
        emu.load(&data);
        assert_eq!(emu.ram[0x200..0x204], data);
    }
    #[test]
    fn opcode_00e0_clear_display() {
        let mut emu = Chip8::new();
        emu.execute(0x00E0);
        assert_eq!(emu.display.as_buffer(), [false; 2048]);
    }
    #[test]
    fn opcode_00ee_return_from_subroutine() {
        let mut emu = Chip8::new();
        emu.stack.push(0xDD3);
        emu.execute(0x00EE);
        assert_eq!(emu.pc, 0xDD3);
    }
    #[test]
    fn opcode_1nnn_jump_to_nn() {
        let mut emu = Chip8::new();
        emu.execute(0x1AAF);
        assert_eq!(emu.pc, 0xAAF);
    }
    #[test]
    fn opcode_2nnn_call_subroutine_at_nn() {
        let mut emu = Chip8::new();
        let old_pc = emu.pc;
        emu.execute(0x2AAF);
        assert_eq!(emu.pc, 0xAAF);
        assert_eq!(emu.stack[0], old_pc);
    }
    #[test]
    fn opcode_3xnn_skip_next_op_if_vx_eq_nn() {
        let mut emu = Chip8::new();
        emu.pc = 0x230;
        emu.reg_v[1] = 0x3E;
        emu.execute(0x313E);
        assert_eq!(emu.pc, 0x232);
    }
    #[test]
    fn opcode_4xnn_skip_next_op_if_vx_ne_nn() {
        let mut emu = Chip8::new();
        emu.pc = 0x230;
        emu.reg_v[1] = 0x3E;
        emu.execute(0x4122);
        assert_eq!(emu.pc, 0x232);
    }
    #[test]
    fn opcode_5xnn_skip_next_op_if_vx_eq_vy() {
        let mut emu = Chip8::new();
        emu.pc = 0x230;
        emu.reg_v[0x1] = 0x3E;
        emu.reg_v[0xE] = 0x3E;
        emu.execute(0x51E0);
        assert_eq!(emu.pc, 0x232);
    }
    #[test]
    fn opcode_5xnn_not_skip_next_op_if_vx_ne_vy() {
        let mut emu = Chip8::new();
        emu.pc = 0x230;
        emu.reg_v[0x1] = 0x3E;
        emu.reg_v[0xE] = 0x1F;
        emu.execute(0x51E0);
        assert_eq!(emu.pc, 0x230);
    }
    #[test]
    fn opcode_4xnn_not_skip_next_op_if_vx_eq_nn() {
        let mut emu = Chip8::new();
        emu.pc = 0x230;
        emu.reg_v[1] = 0x3E;
        emu.execute(0x413E);
        assert_eq!(emu.pc, 0x230);
    }
    #[test]
    fn opcode_6xnn_set_vx_to_nn() {
        let mut emu = Chip8::new();
        emu.execute(0x602F);
        assert_eq!(emu.reg_v[0], 0x2F);
    }
    #[test]
    fn opcode_7xnn_add_nn_to_vx_without_overflow() {
        let mut emu = Chip8::new();
        emu.execute(0x702F);
        assert_eq!(emu.reg_v[0], 0x2F);
        emu.execute(0x702F);
        assert_eq!(emu.reg_v[0], 0x5E);
    }
    #[test]
    fn opcode_7xnn_add_nn_to_vx_with_overflow() {
        let mut emu = Chip8::new();
        emu.execute(0x70F0);
        assert_eq!(emu.reg_v[0], 0xF0);
        emu.execute(0x70F2);
        assert_eq!(emu.reg_v[0], 0xE2);
    }
    #[test]
    fn opcode_8xy0_set_vx_to_vy() {
        let mut emu = Chip8::new();
        emu.reg_v[0x3] = 0x4A;
        emu.reg_v[0x9] = 0x11;
        assert_ne!(emu.reg_v[0x3], emu.reg_v[9]);
        emu.execute(0x8390);
        assert_eq!(emu.reg_v[0x3], emu.reg_v[0x9]);
    }
    #[test]
    fn opcode_8xy1_set_vx_to_vx_or_vy() {
        let mut emu = Chip8::new();
        emu.reg_v[0x3] = 0x4A;
        emu.reg_v[0x9] = 0x11;
        emu.execute(0x8391);
        assert_eq!(emu.reg_v[0x3], 0x5B);
    }
    #[test]
    fn opcode_8xy2_set_vx_to_vx_and_vy() {
        let mut emu = Chip8::new();
        emu.reg_v[0x3] = 0x4A;
        emu.reg_v[0x9] = 0x11;
        emu.execute(0x8392);
        assert_eq!(emu.reg_v[0x3], 0x00);
    }
    #[test]
    fn opcode_8xy3_set_vx_to_vx_xor_vy() {
        let mut emu = Chip8::new();
        emu.reg_v[0x3] = 0x4A;
        emu.reg_v[0x9] = 0x11;
        emu.execute(0x8393);
        assert_eq!(emu.reg_v[0x3], 0x5B);
    }
    #[test]
    fn opcode_8xy4_add_vy_to_vx_without_carry() {
        let mut emu = Chip8::new();
        emu.reg_v[0x3] = 0x4A;
        emu.reg_v[0x9] = 0x11;
        emu.execute(0x8394);
        assert_eq!(emu.reg_v[0x3], 0x5B);
        assert_eq!(emu.reg_v[0xF], 0x00);
    }
    #[test]
    fn opcode_8xy4_add_vy_to_vx_with_carry() {
        let mut emu = Chip8::new();
        emu.reg_v[0x3] = 0x4A;
        emu.reg_v[0x9] = 0xFE;
        emu.execute(0x8394);
        assert_eq!(emu.reg_v[0x3], 0x48);
        assert_eq!(emu.reg_v[0xF], 0x01);
    }
    #[test]
    fn opcode_8xy5_sub_vy_from_vx_without_borrow() {
        let mut emu = Chip8::new();
        emu.reg_v[0x3] = 0x4A;
        emu.reg_v[0x9] = 0x11;
        emu.execute(0x8395);
        assert_eq!(emu.reg_v[0x3], 0x39);
        assert_eq!(emu.reg_v[0xF], 0x01);
    }
    #[test]
    fn opcode_8xy5_sub_vy_from_vx_with_borrow() {
        let mut emu = Chip8::new();
        emu.reg_v[0x3] = 0x4A;
        emu.reg_v[0x9] = 0x53;
        emu.execute(0x8395);
        assert_eq!(emu.reg_v[0x3], 0xF7);
        assert_eq!(emu.reg_v[0xF], 0x00);
    }
    #[test]
    fn opcode_8xy6_shift_vx_right() {
        let mut emu = Chip8::new();
        emu.reg_v[0x3] = 0x4A;
        emu.execute(0x8396);
        assert_eq!(emu.reg_v[0x3], 0x25);
        assert_eq!(emu.reg_v[0xF], 0x00);
    }
    #[test]
    fn opcode_8xy7_set_vx_as_vy_minus_vx_without_borrow() {
        let mut emu = Chip8::new();
        emu.reg_v[0x3] = 0x4A;
        emu.reg_v[0x9] = 0x53;
        emu.execute(0x8397);
        assert_eq!(emu.reg_v[0x3], 0x09);
        assert_eq!(emu.reg_v[0xF], 0x01);
    }
    #[test]
    fn opcode_8xy7_set_vx_as_vy_minus_vx_with_borrow() {
        let mut emu = Chip8::new();
        emu.reg_v[0x3] = 0x4A;
        emu.reg_v[0x9] = 0x22;
        emu.execute(0x8397);
        assert_eq!(emu.reg_v[0x3], 0xD8);
        assert_eq!(emu.reg_v[0xF], 0x00);
    }
    #[test]
    fn opcode_8xye_shift_vx_left() {
        let mut emu = Chip8::new();
        emu.reg_v[0x3] = 0x4A;
        emu.execute(0x839E);
        assert_eq!(emu.reg_v[0x3], 0x94);
        assert_eq!(emu.reg_v[0xF], 0x00);
    }
    #[test]
    fn opcode_9xnn_skip_next_op_if_vx_ne_vy() {
        let mut emu = Chip8::new();
        emu.pc = 0x230;
        emu.reg_v[0x1] = 0x3E;
        emu.reg_v[0xE] = 0x1F;
        emu.execute(0x91E0);
        assert_eq!(emu.pc, 0x232);
    }
    #[test]
    fn opcode_9xnn_not_skip_next_op_if_vx_eq_vy() {
        let mut emu = Chip8::new();
        emu.pc = 0x230;
        emu.reg_v[0x1] = 0x3E;
        emu.reg_v[0xE] = 0x3E;
        emu.execute(0x91E0);
        assert_eq!(emu.pc, 0x230);
    }
    #[test]
    fn opcode_annn_set_i_to_nn() {
        let mut emu = Chip8::new();
        emu.execute(0xAE12);
        assert_eq!(emu.reg_i, 0xE12);
    }
    #[test]
    fn opcode_bnnn_set_i_to_nn_plus_v0() {
        let mut emu = Chip8::new();
        emu.reg_v[0x0] = 0x3;
        emu.execute(0xBE12);
        assert_eq!(emu.reg_i, 0xE15);
    }
    #[test]
    fn opcode_ex9e_skip_next_if_key_vx_is_pressed() {
        let mut emu = Chip8::new();
        emu.pc = 0x206;
        emu.reg_v[0x7] = 0x4;
        emu.keyboard[0x4] = true;
        emu.execute(0xE79E);
        assert_eq!(emu.pc, 0x208);
    }
    #[test]
    fn opcode_ex9e_not_skip_next_if_key_vx_is_not_pressed() {
        let mut emu = Chip8::new();
        emu.pc = 0x206;
        emu.reg_v[0x7] = 0x4;
        emu.keyboard[0x4] = false;
        emu.execute(0xE79E);
        assert_eq!(emu.pc, 0x206);
    }
    #[test]
    fn opcode_exa1_not_skip_next_if_key_vx_is_pressed() {
        let mut emu = Chip8::new();
        emu.pc = 0x206;
        emu.reg_v[0x7] = 0x4;
        emu.keyboard[0x4] = true;
        emu.execute(0xE7A1);
        assert_eq!(emu.pc, 0x206);
    }
    #[test]
    fn opcode_exa1_skip_next_if_key_vx_is_not_pressed() {
        let mut emu = Chip8::new();
        emu.pc = 0x206;
        emu.reg_v[0x7] = 0x4;
        emu.keyboard[0x4] = false;
        emu.execute(0xE7A1);
        assert_eq!(emu.pc, 0x208);
    }
    // TODO: understand how to seed RNG to test CXNN
    // fn opcode_cxnn_set_vx_to_rand_and_nn() {
    // }
    #[test]
    fn opcode_dxyn_draw_sprite() {
        let mut emu = Chip8::new();
        emu.load_sprites();
        emu.execute(0xA000 + FONT_SPRITES_MEM_ADDR as u16);
        emu.execute(0x6000);
        emu.execute(0x6100);
        emu.execute(0xD015);
        assert_eq!(
            emu.display.as_buffer()[0..8],
            [true, true, true, true, false, false, false, false]
        );
        assert_eq!(
            emu.display.as_buffer()[0 + RES_WIDTH..8 + RES_WIDTH],
            [true, false, false, true, false, false, false, false]
        );
        assert_eq!(
            emu.display.as_buffer()[0 + RES_WIDTH * 2..8 + RES_WIDTH * 2],
            [true, false, false, true, false, false, false, false]
        );
        assert_eq!(
            emu.display.as_buffer()[0 + RES_WIDTH * 3..8 + RES_WIDTH * 3],
            [true, false, false, true, false, false, false, false]
        );
        assert_eq!(
            emu.display.as_buffer()[0 + RES_WIDTH * 4..8 + RES_WIDTH * 4],
            [true, true, true, true, false, false, false, false]
        );
    }
    #[test]
    fn opcode_fx1e_add_vx_to_i() {
        let mut emu = Chip8::new();
        emu.reg_i = 0x342;
        emu.reg_v[0x5] = 0x4E;
        emu.execute(0xF51E);
        assert_eq!(emu.reg_i, 0x390);
    }
    #[test]
    fn opcode_fx07_set_vx_to_delay_timer() {
        let mut emu = Chip8::new();
        emu.delay_timer = 0x55;
        emu.execute(0xF607);
        assert_eq!(emu.reg_v[0x6], 0x55);
    }
    #[test]
    fn opcode_fx15_set_delay_timer_to_vx() {
        let mut emu = Chip8::new();
        emu.reg_v[0x6] = 0x55;
        emu.execute(0xF615);
        assert_eq!(emu.delay_timer, 0x55);
    }
    #[test]
    fn opcode_fx18_set_sound_timer_to_vx() {
        let mut emu = Chip8::new();
        emu.reg_v[0x6] = 0x55;
        emu.execute(0xF618);
        assert_eq!(emu.sound_timer, 0x55);
    }
    #[test]
    fn opcode_fx29_set_sprite_addr() {
        let mut emu = Chip8::new();
        emu.execute(0xF329);
        assert_eq!(emu.reg_i, 0xF);
    }
    #[test]
    fn opcode_fx33_store_bcd() {
        let mut emu = Chip8::new();
        emu.reg_i = 0x342;
        emu.reg_v[0x5] = 0x4E;
        emu.execute(0xF533);
        assert_eq!(emu.ram[0x342], 0x00);
        assert_eq!(emu.ram[0x343], 0x07);
        assert_eq!(emu.ram[0x344], 0x08);
    }
    #[test]
    fn opcode_fx55_reg_dump_from_v0_to_vx() {
        let mut emu = Chip8::new();
        emu.reg_v[0x0] = 0x11;
        emu.reg_v[0x1] = 0x22;
        emu.reg_v[0x2] = 0x33;
        emu.reg_i = 0x22A;
        assert_eq!(emu.ram[0x22A], 0x00);
        assert_eq!(emu.ram[0x22B], 0x00);
        assert_eq!(emu.ram[0x22C], 0x00);
        emu.execute(0xF255);
        assert_eq!(emu.ram[0x22A], 0x11);
        assert_eq!(emu.ram[0x22B], 0x22);
        assert_eq!(emu.ram[0x22C], 0x33);
    }
    #[test]
    fn opcode_fx65_reg_load_from_v0_to_vx() {
        let mut emu = Chip8::new();
        emu.ram[0x22A] = 0x11;
        emu.ram[0x22B] = 0x22;
        emu.ram[0x22C] = 0x33;
        emu.reg_i = 0x22A;
        assert_eq!(emu.reg_v[0x0], 0x00);
        assert_eq!(emu.reg_v[0x1], 0x00);
        assert_eq!(emu.reg_v[0x2], 0x00);
        emu.execute(0xF265);
        assert_eq!(emu.reg_v[0x0], 0x11);
        assert_eq!(emu.reg_v[0x1], 0x22);
        assert_eq!(emu.reg_v[0x2], 0x33);
    }
}
