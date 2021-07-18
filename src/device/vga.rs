use olc_pixel_game_engine as olc;
use std::sync::{Mutex, Arc};

pub mod vecs;

// use super::Device;
//
// pub struct VGA {
//
// }

// Very simple example application that prints "Hello, World!" on screen.

pub struct VGA {
    font: psf::Font,
    keys: Vec<olc::Key>,
    mem: Arc<Mutex<super::Ram>>,
}

impl VGA {
    pub fn new(font: psf::Font, keys: Vec<olc::Key>, mem: Arc<Mutex<super::Ram>>) -> Self {
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
                        let pix = g.get(xx, yy).unwrap_or_else(|| false);
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

    fn draw_string(&self, x: i32, y: i32, text: &str, colors: u8) {
        for (idx, c) in text.char_indices() {
            self.draw((idx as i32) + x, y, c as u8, colors)
        }
    }
}

impl olc::Application for VGA {
    fn on_user_create(&mut self) -> Result<(), olc::Error> {
        // Mirrors `olcPixelGameEngine::onUserCreate`. Your code goes here.
        Ok(())
    }

    fn on_user_update(&mut self, _elapsed_time: f32) -> Result<(), olc::Error> {
        olc::set_pixel_mode(olc::PixelMode::MASK);
        // Mirrors `olcPixelGameEngine::onUserUpdate`. Your code goes here.

        // Clears screen and sets black colour.
        olc::clear(olc::BLACK);
        // Prints the string starting at the position (40, 40) and using white colour.

        // olc::draw_string(0, 0, "Hello, World!", olc::WHITE)?;
        // self.draw_string(0, 0, "Display time :tada:", 0x0F);
        // self.draw_string(0, 1, "Random", 0xB3);
        // self.draw_string(7, 1, "Colors", 0xE1);
        // self.draw_string(14, 1, "Are", 0x7A);
        // self.draw_string(18, 1, "Fun!", 0x8F);
        let mem = self.mem.lock().unwrap();
        for x in 0..80 {
            for y in 0..25 {
                let i = (x + y * 80) * 2;

                self.draw(x, y, mem.data[i as usize], mem.data[i as usize + 1])
            }
        }
        for (i, k) in vecs::KEYS.iter().enumerate() {
            let _i = i as u8;
            let _state = olc::get_key(k.clone());
        }
        Ok(())
    }

    fn on_user_destroy(&mut self) -> Result<(), olc::Error> {
        Ok(())
    }
}
