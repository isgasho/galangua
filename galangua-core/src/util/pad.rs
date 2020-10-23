use bitflags::bitflags;

use crate::framework::VKey;

bitflags! {
    #[derive(Default)]
    pub struct PadBit: u32 {
        const L = 0b00000001;
        const R = 0b00000010;
        const U = 0b00000100;
        const D = 0b00001000;
        const A = 0b00010000;
    }
}

#[derive(Default)]
pub struct Pad {
    pad: PadBit,
    trg: PadBit,
    last_pad: PadBit,
    key: PadBit,
    joy: PadBit,
}

impl Pad {
    pub fn update(&mut self) {
        self.pad = self.key | self.joy;
        self.trg = self.pad & !self.last_pad;
        self.last_pad = self.pad;
    }

    pub fn is_pressed(&self, btn: PadBit) -> bool {
        self.pad.contains(btn)
    }

    pub fn is_trigger(&self, btn: PadBit) -> bool {
        self.trg.contains(btn)
    }

    pub fn on_key(&mut self, keycode: VKey, down: bool) {
        let bit = get_key_bit(keycode);
        if down {
            self.key |= bit;
        } else {
            self.key &= !bit;
        }
    }

    pub fn on_joystick_axis(&mut self, axis_index: u8, dir: i8) {
        match axis_index {
            0 => {
                let lr = match dir {
                    dir if dir < 0 => PadBit::L,
                    dir if dir > 0 => PadBit::R,
                    _              => PadBit::empty(),
                };
                self.joy = (self.joy & !(PadBit::L | PadBit::R)) | lr;
            }
            1 => {
                let ud = match dir {
                    dir if dir < 0 => PadBit::U,
                    dir if dir > 0 => PadBit::D,
                    _              => PadBit::empty(),
                };
                self.joy = (self.joy & !(PadBit::U | PadBit::D)) | ud;
            }
            _ => {}
        }
    }

    pub fn on_joystick_button(&mut self, _button_index: u8, down: bool) {
        let bit = PadBit::A;
        if down {
            self.joy |= bit;
        } else {
            self.joy &= !bit;
        }
    }
}

fn get_key_bit(key: VKey) -> PadBit {
    match key {
        VKey::Left => PadBit::L,
        VKey::Right => PadBit::R,
        VKey::Up => PadBit::U,
        VKey::Down => PadBit::D,
        VKey::Space => PadBit::A,
        _ => PadBit::empty(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trigger() {
        let mut pad = Pad::new();
        pad.on_key(VKey::Space, true);
        pad.update();

        assert_eq!(true, pad.is_pressed(PadBit::A));
        assert_eq!(true, pad.is_trigger(PadBit::A));

        pad.update();
        assert_eq!(true, pad.is_pressed(PadBit::A));
        assert_eq!(false, pad.is_trigger(PadBit::A));
    }
}
