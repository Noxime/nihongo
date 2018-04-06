use sdl2::render::{Canvas, Texture};
use sdl2::video::Window;
use sdl2::Sdl;

use constants::*;
use *;

/*
pub fn init(context: &Sdl) -> (Window, Texture) {
    
    (window, tex)
}
*/

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

// pull the framebuffer from memory and display it
pub fn draw_screen(
    mem: &mut Vec<u8>, 
    tex: &mut Texture, canvas: 
    &mut Canvas<Window>,
    last: &mut i64) 
    -> (usize, usize, usize) 
{
    let width  = read(mem, DISPLAY_DAT + 0) as usize;
    let height = read(mem, DISPLAY_DAT + 8) as usize;
    let depth  = read(mem, DISPLAY_DAT + 16) as usize;
    let direct = read(mem, DISPLAY_DAT + 24) as usize;
    let frame  = read(mem, DISPLAY_DAT + 32);
    let rate   = read(mem, DISPLAY_DAT + 40) as usize;
    let on     = read(mem, DISPLAY_DAT + 48) == 0;

    // we do not need to send time on displaying anything if screen is off or there is no new frame
    if !on || last == &frame {
        return (width, height, depth);
    }

    *last = frame;

    // draw our framebuffer
    let _ = tex.update(
        None,
        &mem[
            DISPLAY_LOC as usize .. 
            DISPLAY_LOC as usize + width * height * depth / 8
        ],
        width * depth / 8
    );

    canvas.copy(
        &tex,
        None, None
    ).unwrap();

    let _ = canvas.window_mut().set_size(width as u32, height  as u32);

    canvas.present();
    println!("Draw!");
    (width, height, depth)
}