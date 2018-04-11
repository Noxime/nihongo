#![feature(core_intrinsics)]
#![feature(const_fn)]

extern crate sdl2;
#[cfg(feature = "detect_cpu")]
extern crate raw_cpuid;
extern crate argparse;
#[macro_use]
extern crate lazy_static;
extern crate time;

use sdl2::pixels::PixelFormatEnum;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use argparse::{ArgumentParser, Store};

use std::fs::File;
use std::io::Read;
use std::time::Instant;
use std::thread;

use std::intrinsics::{bswap, likely, unlikely};
use std::mem::transmute;

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

    // argument parsing
    let mut filename = "disk0.bin".to_string();
    let mut option_cores = 4;
    {
        let mut ap = ArgumentParser::new();
        ap.set_description("Nihongo-VM is a DawnOS compatible SUBLEQ emulator.");
        ap.refer(&mut filename)
            .add_option(&["-f", "--file"], Store, "Binary file to load (default 'disk0.bin')");
        ap.refer(&mut option_cores)
            .add_option(&["-c", "--cores"], Store, "How many cores to run (default 4)");
        ap.parse_args_or_exit();
    }

    println!("Loading file: {}", &filename);
    let bin = &mut load_bin(filename);
    println!("Binary file loaded ({} bytes)", bin.len());

    let context = sdl2::init().unwrap();
    let video = context.video().unwrap();
    let window = video.window("Nihongo", 1024, 512)
                                .position_centered()
                                .build().unwrap();
    let mut canvas = window.into_canvas().build().unwrap();
    let tex_creator = canvas.texture_creator();
    let mut tex = tex_creator.create_texture_streaming(
                                Some(PixelFormatEnum::RGB24), // pixel format
                                1024, // dimens    
                                512).unwrap();
    canvas.clear();
    canvas.present();
    let mut pump = context.event_pump().unwrap();

    io::init_disks(bin, vec!["aux_disk_0.bin".into()]);

    write(bin, CPU_RUNNING, CPU_0_FLAGS); // Cpu 0 starts off running

    {
        let mut cpu = CPU_STR.to_string();
        #[cfg(feature = "detect_cpu")]
        {
            let id = raw_cpuid::CpuId::new();
            let brand = id.get_extended_function_info()
                .map(|v| v.processor_brand_string()
                    .map(|v| v.to_string())
                    .unwrap_or("[Unknown]".to_string()))
                .unwrap_or("[Unknown]".to_string());

            cpu = format!("N-VM {}", brand);
            println!("Detected CPU: {}", cpu);
        }
        if cpu.bytes().count() > 40 {
            cpu.truncate(40);
            println!("ERROR: CPU identifier string was longer than 40 bytes. Trunacated to \"{}\"", cpu);
        }
        for (i, b) in cpu.bytes().enumerate() {
            bin[CPU_VENDOR_INFO as usize + i] = b;
        }
    }
    
    // This allows us to circumvent Rust's FEARLESS CONCERRUNCY and replace it with our _spooky parallelism_
    let mem_ptr: u64 = unsafe { transmute((&bin[..]).as_ptr()) };
    let local_time = {
        let t = time::get_time();
        t.sec + time::now().tm_utcoff as i64 - (46*365*24*60*60)
    };
    
    println!("VM state initialized");
    //PROFILER.lock().unwrap().start("./nihongo.profile").expect("Profile failed");
    let start = Instant::now();

    for i in 1 .. option_cores {
        let offset = i*16;
        write(bin, CPU_STOPPED, CPU_0_FLAGS + offset);
        thread::spawn(move || {
            let mem: *mut u8 = unsafe { transmute(mem_ptr) };
            let mut ins = 0;
            let mut pc = read_ptr(mem, CPU_0_PC + offset);
            let mut state = read_ptr(mem, CPU_0_FLAGS + offset);
            println!("CPU_{} thread launched", i);
            loop {
                // check for state changes, aka if we should launch our CPU or
                // something
                ins += 1;
                if unsafe { unlikely(ins % CORE_CHECK_RATE == 0) } {
                    let old_state = state;
                    state = read_ptr(mem, CPU_0_FLAGS + offset);

                    // state, change do our thang
                    if unsafe { unlikely(state != old_state) } {
                        match state {
                            CPU_RUNNING => {
                                pc = read_ptr(mem, CPU_0_PC + offset);
                            },
                            CPU_STOP_REQUESTED => {
                                state = CPU_STOPPED;
                                write_ptr(mem, state, CPU_0_FLAGS + offset);
                                write_ptr(mem, pc, CPU_0_PC + offset);
                            },
                            _ => {}
                        }
                    }
                }

                if unsafe { likely(state == CPU_STOPPED) } {
                    continue;
                } else if unsafe { unlikely(state == CPU_RUNNING) } {
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
                }
            }
        });
    }

    // CPU_0, our main core is super simple to be fast
    thread::spawn(move || {
        let mem: *mut u8 = unsafe { transmute(mem_ptr) };
        let mut pc = read_ptr(mem, CPU_0_PC);
        println!("CPU_0 thread launched");
        loop {
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
        }
    });
    
    // program loop
    'update: loop {
        let time = {
            let e = start.elapsed();
            e.as_secs() as f64 + e.subsec_nanos() as f64 / 1_000_000_000.0
        };

        // Timer
        write(bin, unsafe { transmute(((local_time as f64 + time) * F2TO32) as u64) }, TIMER);

        for event in pump.poll_iter() {
            match event {
                Event::Quit { .. } |
                Event::KeyDown { keycode: Some(Keycode::Escape), .. } => {
                    break 'update;
                },
                Event::MouseMotion { x, y, .. } => {
                    let (w, h) = display::get_dimens(bin);
                    io::update_mouse_pos(bin, x, y, w, h);
                },
                Event::MouseButtonDown { which, .. } => {
                    io::queue_mouse_press(io::MousePress::Down(which as i64));
                },
                Event::MouseButtonUp { which, .. } => {
                    io::queue_mouse_press(io::MousePress::Up(which as i64));
                },
                Event::TextInput { text, .. } => {
                    for ch in text.chars() {
                        io::queue_key(ch as i64);
                    }
                },
                Event::KeyDown { keycode: Some(v), .. } => {
                    io::queue_keycode(v);
                },
                Event::Window { .. } | 
                Event::Unknown { .. } => {},
                v => { println!("Event: {:#?}", v)}
            }
        }

        display::refresh(bin, &mut tex, &mut canvas);
        io::work_mouse_queue(bin);
        io::work_key_queue(bin);
        io::work_disk(bin);

        // updating more often is unnecessary
        thread::sleep_ms(2);
    }

    let finish = {
        let e = start.elapsed();
        e.as_secs() as f64 + e.subsec_nanos() as f64 / 1_000_000_000.0
    };

    io::save_disks();

    println!();
    //PROFILER.lock().unwrap().stop().expect("Can't stop profiler");
    //println!("Nihongo-VM exited with code: {}", code);
    println!("Runtime: {:.2}s", finish);
    println!("Average MIPS: {:.2}", 1 as f64 / finish / 1_000_000.0);
}
