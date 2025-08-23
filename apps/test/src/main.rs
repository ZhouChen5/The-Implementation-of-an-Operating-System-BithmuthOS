// Project Name:  BithmusOS
// File Name:     main.rs
// File Function: System Status Snapshot (Demo)
// Author:        
// License:       MIT License

#![no_std]
#![no_main]

use core::panic::PanicInfo;
use lib;
use lib::{print, println};
use lib::rand::LCG;
use core::fmt::Write;

const TOTAL_MEM_MB: u32 = 16 * 1024; // 16GB
const MIN_PROC: u32 = 100;
const MAX_PROC: u32 = 300;
const MIN_CPU: u32 = 5;
const MAX_CPU: u32 = 95;
const MIN_NET: u32 = 50;
const MAX_NET: u32 = 500;

fn main() {
    println!("+----------------+----------------------+");
    println!("|   BismuthOS System Status Snapshot    |");
    println!("+----------------+----------------------+");
    println!("| (All data below is simulated)         |");
    println!("+--------------+-----------------------+");
    println!("| Item         | Value                 |");
    println!("+--------------+-----------------------+");

    let tick = get_tick();

    // 时间
    let seconds = (tick * 123) % (24 * 3600);
    let (h, m, s) = seconds_to_hms(seconds);

    let cpu = 30 + (tick % 60); // 30~89
    let mem = 4096 + ((tick * 100) % 8192); // 4096~12287 MB
    let proc_num = 100 + (tick % 200); // 100~299
    let net_up = 100 + (tick % 200); // 100~299
    let net_down = 200 + ((tick * 2) % 300); // 200~499

    println!("| Time         | 2025-08-16 {:02}:{:02}:{:02}      |", h, m, s);
    println!("| CPU Usage    | {:>3}%                   |", cpu);
    println!("| Memory       | {:>5.1} GB / 16 GB         |", mem as f32 / 1024.0);
    println!("| Processes    | {:>3}                    |", proc_num);
    println!("| Network      | UP {:>4} KB, DOWN {:>4} KB |", net_up, net_down);
    println!("+--------------+-----------------------+");
}

fn get_tick() -> u32 {
    42 // 你可以每次main运行时手动改这个数
}

fn seconds_to_hms(sec: u32) -> (u32, u32, u32) {
    let h = (sec / 3600) % 24;
    let m = (sec / 60) % 60;
    let s = sec % 60;
    (h, m, s)
}

fn clamp(val: i32, min: u32, max: u32) -> u32 {
    if val < min as i32 {
        min
    } else if val > max as i32 {
        max
    } else {
        val as u32
    }
}

#[no_mangle]
#[link_section = ".start"]
pub extern "C" fn _start() {
    main();
    loop {}
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}