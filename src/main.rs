#![feature(core_intrinsics)]

extern crate sdl2;
extern crate cpuprofiler;

use sdl2::pixels::PixelFormatEnum;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;

use cpuprofiler::PROFILER;

use std::env;
use std::fs::File;
use std::io::Read;
use std::time::Instant;

use std::intrinsics::{bswap, likely, unlikely};

mod constants;
mod display;
mod io;

use constants::*;

fn load_bin(filename: String) -> Vec<u8> {
    let mut file = match File::open(&filename) {
        Ok(v) => v,
        Err(why) => { panic!("Failed to open file: {}", why); }
    };
    let mut bin: Vec<u8> = vec![];
    match file.read_to_end(&mut bin) {
        Err(why) => { panic!("Reading file bytes failed: {}", why); }
        _ => ()
    };
    bin
}

/*
#[inline(always)]
fn read_ptr(base: *const u8, address: i64) -> i64 {
    let addr = unsafe { base.offset(address as isize) as *const i64 };
    unsafe { bswap(*(addr as *const i64) ) }
}

#[inline(always)]
fn write_ptr(base: *mut u8, s: i64, address: i64) {
    use std::ptr::write;
    let addr = unsafe { base.offset(address as isize) };
    unsafe { write(addr as *mut i64, bswap(s)) };
}
*/

#[inline(always)]
fn read(mem: &Vec<u8>, address: i64) -> i64 {
    let base = (&mem[..]).as_ptr();
    let addr = unsafe { base.offset(address as isize) as *const i64 };
    unsafe { bswap(*(addr as *const i64) ) }
}

#[inline(always)]
fn write(mem: &mut Vec<u8>, s: i64, address: i64) {
    use std::ptr::write;
    let base = (&mut mem[..]).as_mut_ptr();
    let addr = unsafe { base.offset(address as isize) };
    unsafe { write(addr as *mut i64, bswap(s)) };
}

fn main() {
    println!("Starting Nihongo, DAWN system emulator");

    let filename = env::args().skip(1).next().unwrap_or("disk0.bin".to_string());
    println!("Loading file: {}", &filename);

    let bin = &mut load_bin(filename);

    println!("Binary file loaded ({} bytes)", bin.len());

    
    // default video
    let width = 1024;
    let height = 512;

    let context = sdl2::init().unwrap();
    let video = context.video().unwrap();
    let window = video.window("Nihongo", width, height)
        .position_centered()
        .build().unwrap();
    let mut canvas = window.into_canvas().build().unwrap();
    let tex_creator = canvas.texture_creator();
    let mut tex = tex_creator.create_texture_streaming(
        Some(PixelFormatEnum::RGB24), // pixel format
        width, // dimens
        height
    ).unwrap();
    canvas.clear();
    canvas.present();
    let mut pump = context.event_pump().unwrap();
    

    // vm variables
    let mut pc1 = 0i64;
    let mut pc2 = 0i64;

    let mut iter = 0usize;
    let mut ins = 0usize;
    let mut test = 0usize;
    let mut last_frame = 0i64;

    // initialize second core
    let mut state2 = 4;

    write(bin, state2, CPU_1_FLAGS); // state is stopped
    write(bin, pc2, CPU_1_PC); // start 0

    println!("VM state initialized, launching");
    
    //PROFILER.lock().unwrap().start("./nihongo.profile").expect("Profile failed");
    let start = Instant::now();
    
    /*
    let mem = (&bin[..]).as_ptr();
    let mem_mut = (&mut bin[..]).as_mut_ptr();
    */

    // program loop
    'main: loop {
        
        { // CPU_0
            let mut pc = pc1;

            let pc_restore = pc;
            let a_addr = read(bin, pc +  0);
            let b_addr = read(bin, pc +  8);
            pc         = read(bin, pc + 16);

            let a = read(bin, a_addr);
            let b = read(bin, b_addr);

            let s = b - a;
            write(bin, s, b_addr);        

            // wait.. we didn't jump! go back to where we came from and go 24
            if unsafe { unlikely(s > 0) } {
                pc = pc_restore + 24;
            }
            write(bin, pc, CPU_0_PC);
            pc1 = pc;

            ins += 1;
        }

        let cpu_1_state = read(bin, CPU_1_FLAGS);
        if cpu_1_state == 1 { // CPU_1
            let mut pc = pc2;
            let lpc = read(bin, CPU_1_PC);
            if pc != lpc {
                println!("CPU_1 PC was off sync! PC: {:#X}, Mem: {:#X}", pc, lpc);
                pc = lpc;
            }

            let pc_restore = pc2;
            let a_addr = read(bin, pc +  0);
            let b_addr = read(bin, pc +  8);
            pc         = read(bin, pc + 16);

            let a = read(bin, a_addr);
            let b = read(bin, b_addr);

            let s = b - a;
            write(bin, s, b_addr);        

            // wait.. we didn't jump! go back to where we came from and go 24
            if unsafe { unlikely(s > 0) } {
                pc = pc_restore + 24;
            }

            write(bin, pc, CPU_1_PC);
            pc2 = pc;

            ins += 1;
        } else if cpu_1_state == 2 {
            println!("CPU_1 disabled");
            write(bin, 4, CPU_1_FLAGS);
        }


        //write(bin, pc, CPU_0_PC);

        /*
        // multicore
        state2 = read(bin, CPU_1_FLAGS);
        pc2 = read(bin, CPU_1_PC);

        let state1 = read(bin, CPU_0_FLAGS);
        if state1 >= 2 {  
            println!("CPU_0 stopped?!");
        }

        // stop requested, stopped
        if state2 == 2 {
            state2 = 4;
            write(bin, state2, CPU_1_FLAGS);
            println!("CPU_1 stop requested");
        }

        ins += 1;

        // cpu_1 running
        if state2 == 1 {
            test += 1;
            let pc_restore = pc2;
            let a_addr = read(bin, pc2 +  0);
            let b_addr = read(bin, pc2 +  8);
            pc2        = read(bin, pc2 + 16);

            let a = read(bin, a_addr);
            let b = read(bin, b_addr);

            let s = b - a;
            write(bin, s, b_addr);

            
            state2 = read(bin, CPU_1_FLAGS);
            pc2 = read(bin, CPU_1_PC);

            // wait.. we didn't jump! go back to where we came from and go 24
            if unsafe { unlikely(s > 0) } {
                pc2 = pc_restore + 24;
            }

            ins += 1;
        }
        write(bin, pc2, CPU_1_PC);
        */

        // end of actual emulator
        
        iter += 1;

        
        if unsafe { unlikely(iter % 4_000_000 == 0) } {
            let time = {
                let e = start.elapsed();
                e.as_secs() as f64 + e.subsec_nanos() as f64 / 1_000_000_000.0
            };

            let (w, h, d) = display::draw_screen(
                bin, 
                &mut tex, 
                &mut canvas, 
                &mut last_frame
            );
            
            io::write_mouse(bin, pump.mouse_state(), w, h);

            let _  = canvas.window_mut()
                .set_title(&format!("Nihongo {}x{}x{} @ {:.2} MIPS, Ins/Clc: {:.3}", 
                w, h, d, 
                ins as f64 / time / 1_000_000.0,
                ins as f64 / iter as f64
            ));

            for event in pump.poll_iter() {
                match event {
                    Event::Quit {..} |
                    Event::KeyDown { keycode: Some(Keycode::Escape), .. } => {
                        break 'main
                    },
                    _ => {}
                }
            }

            let time2 = {
                let e = start.elapsed();
                e.as_secs() as f64 + e.subsec_nanos() as f64 / 1_000_000_000.0
            };
        }
        

    }

    let finish = {
        let e = start.elapsed();
        e.as_secs() as f64 + e.subsec_nanos() as f64 / 1_000_000_000.0
    };

    //PROFILER.lock().unwrap().stop().expect("Can't stop profiler");

    println!("Halted CPU_0, PC: {:#X}", pc1);
    println!("Halted CPU_1, PC: {:#X}", pc2);
    println!("Runtime: {:.2}s, instructions: {} million, cycles {} million", finish, ins / 1_000_000, iter / 1_000_000);
    println!("Average clcs: {:.2}", iter as f64 / finish / 1_000_000.0);
    println!("Average MIPS: {:.2}", ins as f64 / finish / 1_000_000.0);
}