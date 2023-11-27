use axstd::vec::Vec;
use crate::header::Header;
#[cfg(feature = "axstd")]
use axstd::{println, process::exit};
use super::config::*;
use super::page::{init_app_page_table, switch_app_aspace};

struct TaskBlock {
    root_page_table: usize,
    pid: usize,
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
        self.queue.push(TaskBlock {
            root_page_table,
            pid,
        });
        pid
    }
}

static mut TASK_MANAGER: TaskManager = TaskManager::new();

pub fn add_task(root_page_table: usize) -> usize {
    unsafe { TASK_MANAGER.add_task(root_page_table) }
}




pub fn __libc_main_start(c_main: fn() -> i32) {
    println!("__libc_main_start: c_main = 0x{:x}", c_main as usize);
    let ret = c_main();
    println!("__libc_main_start: c_main return = {}", ret);
    run_next();
}

struct Runner {
    count: usize,
}

impl Runner {
    fn count(&mut self) -> usize {
        let ret = self.count;
        self.count += 1;
        ret
    }
}

static mut RUNNER: Runner = Runner { count: 0 };

/// start running apps from PLASH
pub fn start_apps() {
    init_app_page_table();
    run_next();
}

fn run_next() {
    switch_app_aspace();
    let i = unsafe { RUNNER.count() };

    let app_start = (PLASH_START + i) as *const u8;
    // my header: magic(UniKernl), appoff, appsize, all u64. just consider it as inode, to get len of ELF file
    let header = unsafe { (app_start as *const Header).as_ref().unwrap() };

    // check magic
    if header.magic != "UniKernl".as_bytes() {
        return;
    }
    let app_off = header.app_off;
    let app_size = header.app_size;

    // read elf
    let data = unsafe { app_start.add(app_off as usize) };
    let data = unsafe { core::slice::from_raw_parts(data, app_size as usize) };
    let data = super::dl::from_elf(data);

    // write data
    let entry = data.entry();
    data.map_data();

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
