use sdl2::render::{Canvas, Texture};
use sdl2::video::Window;
use sdl2::Sdl;

use constants::*;
use *;

pub fn refresh(mem: &mut Vec<u8>, tex: &mut Texture, can: &mut Canvas<Window>) {
    let width  = read(mem, DISPLAY_DAT + 0) as usize;
    let height = read(mem, DISPLAY_DAT + 8) as usize;
    let depth  = read(mem, DISPLAY_DAT + 16) as usize;
    let direct = read(mem, DISPLAY_DAT + 24) as usize;
    let frame  = read(mem, DISPLAY_DAT + 32);
    let rate   = read(mem, DISPLAY_DAT + 40) as usize;
    let on     = read(mem, DISPLAY_DAT + 48) == 0;

    if !on {
        return;
    }

    let _ = tex.update(
        None,
        &mem[
            DISPLAY_LOC as usize .. 
            DISPLAY_LOC as usize + width * height * depth / 8
        ],
        width * depth / 8
    );

    can.copy(
        &tex,
        None, None
    ).unwrap();

    let _ = can.window_mut().set_size(width as u32, height  as u32);

    can.present();
}

pub fn get_dimens(mem: &mut Vec<u8>) -> (i32, i32) {
    (read(mem, DISPLAY_DAT + 0) as i32, read(mem, DISPLAY_DAT + 8) as i32)
}