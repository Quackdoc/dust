use crate::{
    cpu::arm7,
    utils::{mem_prelude::*, Bytes},
};

#[derive(Clone)]
pub struct KeyBuffer<const LEVEL_3: bool> {
    key_buf: [u32; 0x412],
    key_code: [u32; 3],
}

impl<const LEVEL_3: bool> KeyBuffer<LEVEL_3> {
    pub fn new_boxed<const MODULO: usize>(
        id_code: u32,
        arm7_bios: &Bytes<{ arm7::BIOS_SIZE }>,
    ) -> Box<Self> {
        let mut result = unsafe { Box::<Self>::new_zeroed().assume_init() };
        result.key_code = [id_code, id_code >> 1, id_code << 1];
        for (i, word) in result.key_buf.iter_mut().enumerate() {
            *word = arm7_bios.read_le(0x30 + (i << 2));
        }
        result.apply_key_code::<MODULO>();
        result.apply_key_code::<MODULO>();
        result
    }

    pub fn encrypt_64_bit(&self, [mut y, mut x]: [u32; 2]) -> [u32; 2] {
        for i in 0..0x10 {
            let z = x ^ self.key_buf[i];
            x = (self.key_buf[0x12 + (z >> 24) as usize]
                .wrapping_add(self.key_buf[0x112 + (z >> 16 & 0xFF) as usize])
                ^ self.key_buf[0x212 + (z >> 8 & 0xFF) as usize])
                .wrapping_add(self.key_buf[0x312 + (z & 0xFF) as usize])
                ^ y;
            y = z;
        }
        [x ^ self.key_buf[0x10], y ^ self.key_buf[0x11]]
    }

    pub fn decrypt_64_bit(&self, [mut y, mut x]: [u32; 2]) -> [u32; 2] {
        for i in (2..0x12).rev() {
            let z = x ^ self.key_buf[i];
            x = (self.key_buf[0x12 + (z >> 24) as usize]
                .wrapping_add(self.key_buf[0x112 + (z >> 16 & 0xFF) as usize])
                ^ self.key_buf[0x212 + (z >> 8 & 0xFF) as usize])
                .wrapping_add(self.key_buf[0x312 + (z & 0xFF) as usize])
                ^ y;
            y = z;
        }
        [x ^ self.key_buf[1], y ^ self.key_buf[0]]
    }

    fn apply_key_code<const MODULO: usize>(&mut self) {
        let mut scratch = self.encrypt_64_bit([self.key_code[1], self.key_code[2]]);
        self.key_code[1] = scratch[0];
        self.key_code[2] = scratch[1];
        scratch = self.encrypt_64_bit([self.key_code[0], self.key_code[1]]);
        self.key_code[0] = scratch[0];
        self.key_code[1] = scratch[1];
        scratch = [0, 0];
        for i in 0..0x12 {
            self.key_buf[i] ^= self.key_code[i % MODULO].swap_bytes();
        }
        for i in (0..0x412).step_by(2) {
            scratch = self.encrypt_64_bit(scratch);
            self.key_buf[i] = scratch[1];
            self.key_buf[i + 1] = scratch[0];
        }
    }
}

impl KeyBuffer<false> {
    pub fn level_3<const MODULO: usize>(&self) -> KeyBuffer<true> {
        let mut result = KeyBuffer {
            key_buf: self.key_buf,
            key_code: self.key_code,
        };
        result.key_code[1] <<= 1;
        result.key_code[2] >>= 1;
        result.apply_key_code::<MODULO>();
        result
    }
}
