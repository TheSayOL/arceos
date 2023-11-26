use axstd::vec::Vec;

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
