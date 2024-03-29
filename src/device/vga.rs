use crate::device::{Device, WriteError};
use olc_pixel_game_engine as olc;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

pub mod vecs;

pub struct KeyUpdate(olc::Key, bool);

pub struct Keyboard {
    keys: Arc<Mutex<VecDeque<KeyUpdate>>>,
    addr: u16,
}

impl Keyboard {
    pub fn new(addr: u16, keys: Arc<Mutex<VecDeque<KeyUpdate>>>) -> Self {
        Self { keys, addr }
    }
}

impl Device for Keyboard {
    fn read(&mut self, address: u16) -> Option<u8> {
        if address == self.addr {
            let mut k = self.keys.lock().unwrap();
            match k.pop_front() {
                None => Some(0),
                Some(kv) => {
                    if kv.1 {
                        k.push_front(KeyUpdate(kv.0, false));
                        Some(0xE0)
                    } else {
                        Some(vecs::key_to_scancode(kv.0))
                    }
                }
            }
        } else {
            None
        }
    }

    fn write(&mut self, _: u16, _: u8) -> Result<(), WriteError> {
        Err(WriteError::NotWritable)
    }
}

pub struct Vga {
    font: psf::Font,
    keys: Arc<Mutex<VecDeque<KeyUpdate>>>,
    mem: Arc<Mutex<super::Ram>>,
}

impl Vga {
    pub fn new(
        font: psf::Font,
        keys: Arc<Mutex<VecDeque<KeyUpdate>>>,
        mem: Arc<Mutex<super::Ram>>,
    ) -> Self {
        Self { font, keys, mem }
    }

    fn draw(&self, x: i32, y: i32, char: u8, colors: u8) {
        let fg = (colors & 0xF) as usize;
        let bg = (colors >> 4) as usize;

        let c = char::from(char);
        let c = self.font.get_char(c);
        match c {
            None => {}
            Some(g) => {
                for xx in 0..g.width() {
                    for yy in 0..g.height() {
                        let pix = g.get(xx, yy).unwrap_or(false);
                        if pix {
                            olc::fill_rect(
                                x * 8 + (xx as i32),
                                y * 14 + (yy as i32),
                                1,
                                1,
                                vecs::COLORS[fg],
                            );
                        } else {
                            olc::fill_rect(
                                x * 8 + (xx as i32),
                                y * 14 + (yy as i32),
                                1,
                                1,
                                vecs::COLORS[bg],
                            );
                        }
                    }
                }
            }
        }
        // olc::draw_partial_sprite(x * 8, y * 14, &self.spr, char * 8, 0, 8, 14)
    }

    fn _draw_string(&self, x: i32, y: i32, text: &str, colors: u8) {
        for (idx, c) in text.char_indices() {
            self.draw((idx as i32) + x, y, c as u8, colors)
        }
    }
}

impl olc::Application for Vga {
    fn on_user_create(&mut self) -> Result<(), olc::Error> {
        // Mirrors `olcPixelGameEngine::onUserCreate`. Your code goes here.
        Ok(())
    }

    fn on_user_update(&mut self, _elapsed_time: f32) -> Result<(), olc::Error> {
        olc::set_pixel_mode(olc::PixelMode::MASK);
        olc::clear(olc::BLACK);
        let mem = self.mem.lock().unwrap();
        for x in 0..80 {
            for y in 0..25 {
                let i = (x + y * 80) * 2;

                self.draw(x, y, mem.data[i as usize], mem.data[i as usize + 1])
            }
        }
        for k in vecs::KEYS {
            let state = olc::get_key(k);
            if state.pressed {
                let mut ksr = self.keys.lock().unwrap();
                ksr.push_back(KeyUpdate(k, false))
            }
            if state.released {
                let mut ksr = self.keys.lock().unwrap();
                ksr.push_back(KeyUpdate(k, true))
            }
        }
        Ok(())
    }

    fn on_user_destroy(&mut self) -> Result<(), olc::Error> {
        Ok(())
    }
}
