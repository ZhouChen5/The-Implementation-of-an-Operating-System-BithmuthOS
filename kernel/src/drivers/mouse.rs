// Project Name:  BismuthOS
// File Name:     mouse.rs
// File Function: PS/2 Mouse driver
// Author:         & ChatGPT
// License:       MIT License

use core::arch::asm;
use crate::drivers::pic::PICS;

pub const MOUSE_INT: u8 = 44; // IRQ12 = 32+12

pub struct Mouse {
    pub x: i32,
    pub y: i32,
    pub buttons: u8,
    packet: [u8; 3],
    packet_index: usize,
    pub enabled: bool,
    pub last_pick: Option<(usize, usize, u8)>, // (row, col, char)
    last_left: bool, // 上一帧左键状态
    pub last_highlight: Option<(usize, usize, u8)>, // (row, col, old_attr)
    pub selection_start: Option<(usize, usize)>,
    pub selection_end: Option<(usize, usize)>,
    pub original_attrs: [Option<(usize, usize, u8)>; 80], // 保存原始属性
    pub original_attr_count: usize,
}

pub static mut MOUSE: Mouse = Mouse {
    x: 0,
    y: 0,
    buttons: 0,
    packet: [0; 3],
    packet_index: 0,
    enabled: false,
    last_pick: None,
    last_left: false,
    last_highlight: None,
    selection_start: None,
    selection_end: None,
    original_attrs: [None; 80],
    original_attr_count: 0,
};

impl Mouse {
    pub unsafe fn init(&mut self) {
        // 启用辅助设备（鼠标）
        Self::wait_write();
        Self::write_cmd(0xA8);
        // 启用中断
        Self::wait_write();
        Self::write_cmd(0x20);
        Self::wait_read();
        let mut status: u8 = Self::read_data();
        status |= 2;
        Self::wait_write();
        Self::write_cmd(0x60);
        Self::wait_write();
        Self::write_data(status);
        // 设置默认值
        Self::mouse_write(0xF6);
        Self::mouse_read();
        // 启用鼠标
        Self::mouse_write(0xF4);
        Self::mouse_read();
        self.enabled = true;
    }

    pub fn get_position(&self) -> (i32, i32) {
        (self.x, self.y)
    }

    fn wait_write() {
        let mut status: u8;
        loop {
            unsafe {
                asm!("in al, dx", out("al") status, in("dx") 0x64u16);
            }
            if status & 0x2 == 0 { break; }
        }
    }
    fn wait_read() {
        let mut status: u8;
        loop {
            unsafe {
                asm!("in al, dx", out("al") status, in("dx") 0x64u16);
            }
            if status & 0x1 != 0 { break; }
        }
    }
    fn write_cmd(cmd: u8) {
        unsafe {
            asm!("out dx, al", in("dx") 0x64u16, in("al") cmd);
        }
    }
    fn write_data(data: u8) {
        unsafe {
            asm!("out dx, al", in("dx") 0x60u16, in("al") data);
        }
    }
    fn read_data() -> u8 {
        let data: u8;
        unsafe {
            asm!("in al, dx", out("al") data, in("dx") 0x60u16);
        }
        data
    }
    fn mouse_write(data: u8) {
        Self::wait_write();
        Self::write_cmd(0xD4);
        Self::wait_write();
        Self::write_data(data);
    }
    fn mouse_read() -> u8 {
        Self::wait_read();
        Self::read_data()
    }
    
    pub unsafe fn handle_irq(&mut self) {
        let data = Self::read_data();
        self.packet[self.packet_index] = data;
        self.packet_index += 1;
        if self.packet_index == 3 {
            self.packet_index = 0;
            let packet = self.packet;
            let x_overflow = (packet[0] & 0x40) != 0;
            let y_overflow = (packet[0] & 0x80) != 0;
            let dx = packet[1] as i8 as i32;
            let dy = packet[2] as i8 as i32;
            if x_overflow || y_overflow {
                return;
            }
            self.x = self.x.saturating_add(dx);
            self.y = self.y.saturating_sub(dy);
            // 限制鼠标坐标在屏幕范围内
            self.x = self.x.clamp(0, 80 * 8 - 1);
            self.y = self.y.clamp(0, 25 * 16 - 1);
            let left = (packet[0] & 0x1) != 0;
            let col = (self.x / 8).clamp(0, 79) as usize;
            let row = (self.y / 16).clamp(0, 24) as usize;
            
            // 处理鼠标事件
            if left && !self.last_left {
                if self.selection_start.is_some() {
                    // 如果已有选区，只清除，不新建
                    self.clear_all_highlights();
                    self.selection_start = None;
                    self.selection_end = None;
                } else {
                    // 没有选区，才新建
                    self.clear_all_highlights();
                    self.start_new_selection(row, col);
                    // self.update_single_highlight(row, col); // 不再单独高亮
                }
            } else if left && self.last_left {
                // 鼠标左键拖动
                self.update_selection(row, col);
            } else if !left && self.last_left {
                // 鼠标左键释放
                // 什么都不做，保持选区高亮
            }
            
            self.last_left = left;
            self.buttons = packet[0] & 0x07;
        }
        PICS.end_interrupt(MOUSE_INT);
    }
    
    // 清除所有高亮（包括选区和单字符高亮）
    unsafe fn clear_all_highlights(&mut self) {
        // 清除选区高亮
        self.restore_original_attrs();
        
        // 清除单字符高亮
        if let Some((last_row, last_col, old_attr)) = self.last_highlight {
            let vga_ptr = 0xB8000 as *mut u8;
            if last_row < 25 && last_col < 80 {
                let last_attr_ptr = vga_ptr.add((last_row * 80 + last_col) * 2 + 1);
                *last_attr_ptr = old_attr;
            }
            self.last_highlight = None;
        }
    }
    
    // 开始新选区
    unsafe fn start_new_selection(&mut self, row: usize, col: usize) {
        self.selection_start = Some((row, col));
        self.selection_end = Some((row, col));
        self.original_attr_count = 0;
        self.highlight_selection();
    }
    
    // 更新选区
    unsafe fn update_selection(&mut self, row: usize, col: usize) {
        if let Some(start) = self.selection_start {
            // 恢复之前的高亮
            self.restore_original_attrs();
            
            // 更新选区终点
            self.selection_end = Some((row, col));
            
            // 应用新选区高亮
            self.highlight_selection();
        }
    }
    
    // 恢复原始属性
    unsafe fn restore_original_attrs(&mut self) {
        let vga_ptr = 0xB8000 as *mut u8;
        for i in 0..self.original_attr_count {
            if let Some((row, col, attr)) = self.original_attrs[i] {
                if row < 25 && col < 80 {
                    let attr_ptr = vga_ptr.add((row * 80 + col) * 2 + 1);
                    *attr_ptr = attr;
                }
            }
        }
        self.original_attr_count = 0;
    }
    
    // 高亮选区
    unsafe fn highlight_selection(&mut self) {
        if let (Some((start_row, start_col)), Some((end_row, end_col))) = (self.selection_start, self.selection_end) {
            let vga_ptr = 0xB8000 as *mut u8;
            
            // 确定选区范围
            let (min_row, min_col, max_row, max_col) = if start_row <= end_row {
                (start_row, start_col.min(end_col), end_row, start_col.max(end_col))
            } else {
                (end_row, start_col.min(end_col), start_row, start_col.max(end_col))
            };
            
            // 仅处理单行选区
            if min_row == max_row {
                for col in min_col..=max_col {
                    if col < 80 {
                        let offset = (min_row * 80 + col) * 2 + 1;
                        let attr_ptr = vga_ptr.add(offset);
                        
                        // 保存原始属性
                        let old_attr = *attr_ptr;
                        if self.original_attr_count < 80 {
                            self.original_attrs[self.original_attr_count] = Some((min_row, col, old_attr));
                            self.original_attr_count += 1;
                        }
                        
                        // 应用高亮
                        *attr_ptr = 0xF0; // 亮白底黑字
                    }
                }
            }
        }
    }
    
    // 更新单字符高亮
    unsafe fn update_single_highlight(&mut self, row: usize, col: usize) {
        let vga_ptr = 0xB8000 as *mut u8;
        let offset = (row * 80 + col) * 2;
        
        // 保存点击的字符
        let ch = if row < 25 && col < 80 {
            *vga_ptr.add(offset)
        } else {
            b' '
        };
        self.last_pick = Some((row, col, ch));
        
        // 保存当前属性并设置高亮
        let attr_ptr = vga_ptr.add(offset + 1);
        let old_attr = *attr_ptr;
        *attr_ptr = 0xF0; // 设置高亮
        self.last_highlight = Some((row, col, old_attr));
    }

    pub fn get_last_pick(&self) -> Option<(usize, usize, u8)> {
        self.last_pick
    }

    pub fn get_selection_text(&self) -> Option<[char; 80]> {
        if let (Some((start_row, start_col)), Some((end_row, end_col))) = (self.selection_start, self.selection_end) {
            if start_row == end_row {
                let (min_col, max_col) = (start_col.min(end_col), start_col.max(end_col));
                let mut buf = ['\0'; 80];
                let vga_ptr = 0xB8000 as *const u8;
                for (i, col) in (min_col..=max_col).enumerate() {
                    if col < 80 {
                        let offset = (start_row * 80 + col) * 2;
                        let ch = unsafe { *vga_ptr.add(offset) } as char;
                        buf[i] = ch;
                    }
                }
                return Some(buf);
            }
        }
        None
    }
}

#[naked]
pub extern "C" fn mouse() {
    unsafe {
        asm!(
            "pushad",
            "call mouse_handler",
            "popad",
            "iretd",
            options(noreturn)
        );
    }
}

#[no_mangle]
pub extern "C" fn mouse_handler() {
    unsafe {
        MOUSE.handle_irq();
    }
}