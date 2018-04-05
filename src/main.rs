#![feature(core_intrinsics)]
#![feature(const_fn)]

#[cfg(feature = "sdl")]
extern crate sdl2;
#[cfg(feature = "detect_cpu")]
extern crate raw_cpuid;


#[cfg(feature = "sdl")]
use sdl2::pixels::PixelFormatEnum;
#[cfg(feature = "sdl")]
use sdl2::event::Event;
#[cfg(feature = "sdl")]
use sdl2::keyboard::Keycode;

use std::env;
use std::fs::File;
use std::io::Read;
use std::time::Instant;

use std::thread;
use std::sync::{Mutex, Arc};

use std::intrinsics::{bswap, likely, unlikely};

mod constants;
#[cfg(feature = "sdl")]
mod display;
#[cfg(feature = "sdl")]
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

#[inline(always)]
fn read_ptr(base: *const u8, address: i64) -> i64 {
    let addr = unsafe { base.offset(address as isize) as *const i64 };
    unsafe { bswap(*(addr as *const i64) ) }
}
#[inline(always)]
fn write_ptr(base: *mut u8, s: i64, address: i64) {
    use std::ptr::write;
    //let base = (&mut mem[..]).as_mut_ptr();
    let addr = unsafe { base.offset(address as isize) };
    unsafe { write(addr as *mut i64, bswap(s)) };
}

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

    //let raw_bin = &mut load_bin(filename);
    //let bin = Arc::new(raw_bin);
    let bin = &mut load_bin(filename);

    println!("Binary file loaded ({} bytes)", bin.len());

    
    // default video
    let width = 1024;
    let height = 512;

    
    #[cfg(feature = "sdl")] let context = sdl2::init().unwrap();
    #[cfg(feature = "sdl")] let video = context.video().unwrap();
    #[cfg(feature = "sdl")] let window = video.window("Nihongo", width, height)
                                .position_centered()
                                .build().unwrap();
    #[cfg(feature = "sdl")] let mut canvas = window.into_canvas().build().unwrap();
    #[cfg(feature = "sdl")] let tex_creator = canvas.texture_creator();
    #[cfg(feature = "sdl")] let mut tex = tex_creator.create_texture_streaming(
                                Some(PixelFormatEnum::RGB24), // pixel format
                                width, // dimens    
                                height).unwrap();
    #[cfg(feature = "sdl")] canvas.clear();
    #[cfg(feature = "sdl")] canvas.present();
    #[cfg(feature = "sdl")] let mut pump = context.event_pump().unwrap();

    let mut iter = 0usize;
    let mut test = 0usize;
    let mut last_frame = 0i64;

    write(bin, CPU_RUNNING, CPU_0_FLAGS); // Cpu 0 starts off running
    write(bin, CPU_STOPPED, CPU_1_FLAGS); // State is stopped
    write(bin, CPU_STOPPED, CPU_2_FLAGS); // State is stopped
    write(bin, CPU_STOPPED, CPU_3_FLAGS); // State is stopped
    write(bin, 0, CPU_0_PC); // start 0
    write(bin, 0, CPU_1_PC); // start 1
    write(bin, 0, CPU_2_PC); // start 2
    write(bin, 0, CPU_3_PC); // start 3

    // write cpu metadata
    write(bin, 1, CPU_SPINUP_CYCLES); // since we are single threaded, our core spinup is instant

    {
        let mut cpu = CPU_STR.to_string();
        #[cfg(feature = "detect_cpu")]
        {
            let id = raw_cpuid::CpuId::new();
            let v = format!("{}", id.get_vendor_info()
                .map(|v| format!("{}", v))
                .unwrap_or("[UNKNOWN]".to_string()));
            let m = format!("{}", id.get_feature_info()
                .map(|v| format!("{}-{}", v.family_id(), v.model_id()))
                .unwrap_or("[UNKNOWN]".to_string()));

            cpu = format!("N-VM {} {}", v, m);
            println!("Detected CPU: {}", cpu);
        }
        for (i, b) in cpu.bytes().enumerate() {
            bin[CPU_VENDOR_INFO as usize + i] = b;
        }
    }
    

    println!("VM state initialized");
    
    //PROFILER.lock().unwrap().start("./nihongo.profile").expect("Profile failed");
    let start = Instant::now();

    use std::mem::transmute;
    let mem_ptr: u64 = unsafe { transmute((&bin[..]).as_ptr()) };

    let t1 = thread::spawn(move || {
        let mem: *mut u8 = unsafe { transmute(mem_ptr) };
        let mut ins = 0usize;
        loop {
            match read_ptr(mem, CPU_0_FLAGS) { // CPU_1
                //CPU_NOT_PRESENT | // This is disabled, don't run
                //CPU_STOPPED => {}, // CPU is asleep, NOP
                //CPU_STOP_REQUESTED => { write_ptr(mem, CPU_STOPPED, CPU_0_FLAGS); },
                CPU_SHUTDOWN => { println!("CPU_0 requested shutdown"); unimplemented!() },
                CPU_RESET => { println!("CPU_0 requested hard reset"); unimplemented!() },
                //CPU_RUNNING | 
                CPU_RUNNING => {
                    let mut pc = read_ptr(mem, CPU_3_PC);
                    let pc_restore = pc;
                    let a_addr = read_ptr(mem, pc +  0);
                    let b_addr = read_ptr(mem, pc +  8);
                    pc         = read_ptr(mem, pc + 16);
                    let a = read_ptr(mem, a_addr);
                    let b = read_ptr(mem, b_addr);
                    let s = b - a;
                    write_ptr(mem, s, b_addr);
                    if unsafe { unlikely(s > 0) } {
                        pc = pc_restore + 24;
                    }
                    write_ptr(mem, pc, CPU_3_PC);
                    ins += 1;
                },
                _ => {}
            }
        }
    });
    thread::spawn(move || {
        let mem: *mut u8 = unsafe { transmute(mem_ptr) };
        loop {
            match read_ptr(mem, CPU_1_FLAGS) { // CPU_1
                CPU_NOT_PRESENT | // This is disabled, don't run
                CPU_STOPPED => {}, // CPU is asleep, NOP
                CPU_STOP_REQUESTED => { write_ptr(mem, CPU_STOPPED, CPU_1_FLAGS); },
                CPU_SHUTDOWN => { println!("CPU_1 is not allowed to shutdown the system; Ignored"); },
                CPU_RESET => { println!("CPU_1 is not allowed to reset the system; Ignored"); },
                //CPU_RUNNING | 
                CPU_RUNNING => {
                    let mut pc = read_ptr(mem, CPU_1_PC);
                    let pc_restore = pc;
                    let a_addr = read_ptr(mem, pc +  0);
                    let b_addr = read_ptr(mem, pc +  8);
                    pc         = read_ptr(mem, pc + 16);
                    let a = read_ptr(mem, a_addr);
                    let b = read_ptr(mem, b_addr);
                    let s = b - a;
                    write_ptr(mem, s, b_addr);
                    if unsafe { unlikely(s > 0) } {
                        pc = pc_restore + 24;
                    }
                    write_ptr(mem, pc, CPU_1_PC);
                },
                _ => {}
            }
        }
    });
    thread::spawn(move || {
        let mem: *mut u8 = unsafe { transmute(mem_ptr) };
        loop {
            match read_ptr(mem, CPU_2_FLAGS) { // CPU_1
                CPU_NOT_PRESENT | // This is disabled, don't run
                CPU_STOPPED => {}, // CPU is asleep, NOP
                CPU_STOP_REQUESTED => { write_ptr(mem, CPU_STOPPED, CPU_2_FLAGS); },
                CPU_SHUTDOWN => { println!("CPU_2 is not allowed to shutdown the system; Ignored"); },
                CPU_RESET => { println!("CPU_2 is not allowed to reset the system; Ignored"); },
                //CPU_RUNNING | 
                CPU_RUNNING => {
                    let mut pc = read_ptr(mem, CPU_2_PC);
                    let pc_restore = pc;
                    let a_addr = read_ptr(mem, pc +  0);
                    let b_addr = read_ptr(mem, pc +  8);
                    pc         = read_ptr(mem, pc + 16);
                    let a = read_ptr(mem, a_addr);
                    let b = read_ptr(mem, b_addr);
                    let s = b - a;
                    write_ptr(mem, s, b_addr);
                    if unsafe { unlikely(s > 0) } {
                        pc = pc_restore + 24;
                    }
                    write_ptr(mem, pc, CPU_2_PC);
                },
                _ => {}
            }
        }
    });
    thread::spawn(move || {
        let mem: *mut u8 = unsafe { transmute(mem_ptr) };
        loop {
            match read_ptr(mem, CPU_3_FLAGS) { // CPU_1
                CPU_NOT_PRESENT | // This is disabled, don't run
                CPU_STOPPED => {}, // CPU is asleep, NOP
                CPU_STOP_REQUESTED => { write_ptr(mem, CPU_STOPPED, CPU_3_FLAGS); },
                CPU_SHUTDOWN => { println!("CPU_3 is not allowed to shutdown the system; Ignored"); },
                CPU_RESET => { println!("CPU_3 is not allowed to reset the system; Ignored"); },
                //CPU_RUNNING | 
                CPU_RUNNING => {
                    let mut pc = read_ptr(mem, CPU_3_PC);
                    let pc_restore = pc;
                    let a_addr = read_ptr(mem, pc +  0);
                    let b_addr = read_ptr(mem, pc +  8);
                    pc         = read_ptr(mem, pc + 16);
                    let a = read_ptr(mem, a_addr);
                    let b = read_ptr(mem, b_addr);
                    let s = b - a;
                    write_ptr(mem, s, b_addr);
                    if unsafe { unlikely(s > 0) } {
                        pc = pc_restore + 24;
                    }
                    write_ptr(mem, pc, CPU_3_PC);
                },
                _ => {}
            }
        }
    });
    
    // program loop
    'main: loop {
        iter += 1;

        #[cfg(not(feature = "sdl"))] {
            if unsafe { unlikely(iter % 32_000_000 == 0) } {
                let time = {
                    let e = start.elapsed();
                    e.as_secs() as f64 + e.subsec_nanos() as f64 / 1_000_000_000.0
                };
                println!("{:.2} MIPS, {:.2} CLCS",
                ins as f64 / time / 1_000_000.0,
                iter as f64 / time / 1_000_000.0
                );
            }
        }

        #[cfg(feature = "sdl")]
        {
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
                .set_title(&format!("Nihongo {}x{}x{} @ {:.2} MIPS, CLCS: {:.2} Cores: {}{}{}{}", 
                w, h, d, 
                1 as f64 / time / 1_000_000.0,
                iter as f64 / time / 1_000_000.0,
                if read(bin, CPU_0_FLAGS) == 1 {'R'} else {'S'},
                if read(bin, CPU_1_FLAGS) == 1 {'R'} else {'S'},
                if read(bin, CPU_2_FLAGS) == 1 {'R'} else {'S'},
                if read(bin, CPU_3_FLAGS) == 1 {'R'} else {'S'},
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
        

    }

    let finish = {
        let e = start.elapsed();
        e.as_secs() as f64 + e.subsec_nanos() as f64 / 1_000_000_000.0
    };

    println!();
    //PROFILER.lock().unwrap().stop().expect("Can't stop profiler");
    //println!("Nihongo-VM exited with code: {}", code);
    println!("Halted CPU_0, PC: {:#X}", read(bin, CPU_0_PC));
    println!("Halted CPU_1, PC: {:#X}", read(bin, CPU_1_PC));
    println!("Halted CPU_2, PC: {:#X}", read(bin, CPU_2_PC));
    println!("Halted CPU_3, PC: {:#X}", read(bin, CPU_3_PC));
    println!("Runtime: {:.2}s, instructions: {} million, cycles {} million", finish, 1 / 1_000_000, iter / 1_000_000);
    println!("Average clcs: {:.2}", iter as f64 / finish / 1_000_000.0);
    println!("Average MIPS: {:.2}", 1 as f64 / finish / 1_000_000.0);
}
