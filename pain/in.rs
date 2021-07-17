use std::ops::BitAnd;
use std::num::Wrapping;


impl super::CPU{
  pub fn the_match_statement(&mut self, mut pins: super::Pins){
    match 0 {
$decode_block
      _ => panic!(
        "This instruction does not exist: {:#04X}|{}!",
        self.ir >> 3,
        self.ir & 7
      ),
    }
  }
}