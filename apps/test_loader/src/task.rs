use super::config::*;
use super::page::{init_app_page_table, switch_app_aspace};
use crate::header::Header;
use crate::mem;
use axstd::vec::Vec;
#[cfg(feature = "axstd")]
use axstd::{println, process::exit};

struct TaskBlock {
    root_page_table: usize,
    pid: usize,
    sp: usize,
}

impl TaskBlock {
    fn new(root_page_table: usize, pid: usize, sp: usize) -> Self {
        Self {
            root_page_table,
            pid,
            sp: 0,
        }
    }
}

struct TaskManager {
    queue: Vec<TaskBlock>,
    current_pid: usize,
}

impl TaskManager {
    const fn new() -> Self {
        Self {
            queue: Vec::new(),
            current_pid: 0,
        }
    }
    fn add_task(&mut self, root_page_table: usize) -> usize {
        let pid = self.current_pid;
        self.current_pid += 1;
        self.queue.push(TaskBlock::new(root_page_table, pid, 0));
        pid
    }
}

static mut TASK_MANAGER: TaskManager = TaskManager::new();

pub fn add_task() -> usize {
    1
}

pub fn __libc_main_start(c_main: fn() -> i32) {

    unsafe {
        core::arch::asm!("
        nop
        nop
        nop");
    }

    println!("__libc_main_start: c_main = 0x{:x}", c_main as usize);
    let ret = c_main();
    println!("__libc_main_start: c_main return = {}", ret);
    run_next();
}

/// start running apps from PLASH
pub fn start_apps() {
    init_app_page_table();
    switch_app_aspace();
    run_next();
}

fn run_next() {
    static mut xx:i32 = 0;
    unsafe {
        if xx == 2 {
            exit(1);
        } else {
            xx += 1;
        }
    }

    let mut raw_datas = Vec::new();
    println!("...  {:?}", "UniKnernl".as_bytes());

    let mut app_start = PLASH_START;
    loop {
        let header = unsafe { (app_start as *const Header).as_ref().unwrap() };
        // check magic
        if header.magic != "UniKernl".as_bytes() {
            break;
        }
        let app_off = header.app_off;
        let app_size = header.app_size;
        println!("off {:X}, size {:X}", app_off, app_size);

        let data = app_start + app_off as usize;
        let data = unsafe { core::slice::from_raw_parts(data as *const u8, app_size as usize) };
        raw_datas.push(data);
        app_start += (app_off + app_size) as usize;
    }
    // read elf
    let data = {
        let data = raw_datas[0];
        let data = super::dl::from_elf(data);
        data
    };
    let entry = data.entry();
    // write data

    println!("data");
    for s in data.data() {
        mem::map_data(&s.data, s.start_va, s.len);
    }

    // init stack
    let sp = STACK_TOP;

    // execute app
    unsafe {
        core::arch::asm!("
        nop
        mv      sp,t2
        jalr    t1
        ",
        in("t1")entry,
        in("t2")sp,
        )
    };
    // exit, or it will return to `__libc_start_main`
    exit(0);
}
