pub const RES_WIDTH: usize = 64;
pub const RES_HEIGHT: usize = 32;

pub type DisplayBuffer = [bool; RES_WIDTH * RES_HEIGHT];

pub struct Display {
    buffer: DisplayBuffer,
}

impl Display {
    pub fn new() -> Self {
        Self {
            buffer: [false; RES_WIDTH * RES_HEIGHT],
        }
    }

    pub fn clear(&mut self) {
        self.buffer = [false; RES_WIDTH * RES_HEIGHT];
    }

    pub fn as_buffer(&mut self) -> DisplayBuffer {
        self.buffer
    }

    pub fn draw(&mut self, sprite: Vec<u8>, x: u8, y: u8) -> bool {
        let x_wrapped = x as usize % RES_WIDTH;
        let y_wrapped = y as usize % RES_HEIGHT;
        let mut unset = false;
        for (row, byte) in sprite.iter().enumerate() {
            for col in 0..8 {
                let pixel_value = byte & (0b1000_0000 >> col);
                let pixel_idx = (y_wrapped + row) * RES_WIDTH + x_wrapped + col;
                if pixel_idx >= RES_WIDTH * RES_HEIGHT {
                    continue;
                };
                if pixel_value > 0 {
                    if self.buffer[pixel_idx] {
                        unset = true;
                    }
                    self.buffer[pixel_idx] = !self.buffer[pixel_idx];
                }
            }
        }
        unset
    }
}
