//! Implementation of process management mechanism
//!
//! Here is the entry for process scheduling required by other modules
//! (such as syscall or clock interrupt).
//! By suspending or exiting the current process, you can
//! modify the process state, manage the process queue through TASK_MANAGER,
//! and switch the control flow through PROCESSOR.
//!
//! Be careful when you see [`__switch`]. Control flow around this function
//! might not be what you expect.

mod context;
mod manager;
mod pid;
mod processor;
mod switch;
#[allow(clippy::module_inception)]
mod task;

use crate::loader::get_app_data_by_name;
use alloc::sync::Arc;
use lazy_static::*;
use manager::fetch_task;
use switch::__switch;
pub use task::{TaskControlBlock, TaskStatus, CurTaskInfo,BIG_STRIDE};

pub use context::TaskContext;
pub use manager::add_task;
pub use pid::{pid_alloc, KernelStack, PidHandle};
pub use processor::{
    current_task, current_trap_cx, current_user_token, run_tasks, schedule, take_current_task,
};
use crate::syscall::{SYSCALL_EXIT, SYSCALL_GET_TIME, SYSCALL_WRITE, SYSCALL_TASK_INFO, SYSCALL_YIELD};

/// Make current task suspended and switch to the next task
pub fn suspend_current_and_run_next() {
    // There must be an application running.
    let task = take_current_task().unwrap();

    // ---- access current TCB exclusively
    let mut task_inner = task.inner_exclusive_access();
    let task_cx_ptr = &mut task_inner.task_cx as *mut TaskContext;
    // Change status to Ready
    task_inner.task_status = TaskStatus::Ready;
    task_inner.task_info.task_status=TaskStatus::Ready;
    drop(task_inner);
    // ---- release current PCB

    // push back to ready queue.
    add_task(task);
    // jump to scheduling cycle
    schedule(task_cx_ptr);
}

/// Exit current task, recycle process resources and switch to the next task
pub fn exit_current_and_run_next(exit_code: i32) {
    // take from Processor
    let task = take_current_task().unwrap();
    // **** access current TCB exclusively
    let mut inner = task.inner_exclusive_access();
    // Change status to Zombie
    inner.task_status = TaskStatus::Zombie;
    // Record exit code
    inner.exit_code = exit_code;
    // do not move to its parent but under initproc

    // ++++++ access initproc TCB exclusively
    {
        let mut initproc_inner = INITPROC.inner_exclusive_access();
        for child in inner.children.iter() {
            child.inner_exclusive_access().parent = Some(Arc::downgrade(&INITPROC));
            initproc_inner.children.push(child.clone());
        }
    }
    // ++++++ release parent PCB

    inner.children.clear();
    // deallocate user space
    inner.memory_set.recycle_data_pages();
    drop(inner);
    // **** release current PCB
    // drop task manually to maintain rc correctly
    drop(task);
    // we do not have to save task context
    let mut _unused = TaskContext::zero_init();
    schedule(&mut _unused as *mut _);
}

lazy_static! {
    /// Creation of initial process
    ///
    /// the name "initproc" may be changed to any other app name like "usertests",
    /// but we have user_shell, so we don't need to change it.
    pub static ref INITPROC: Arc<TaskControlBlock> = Arc::new(TaskControlBlock::new(
        get_app_data_by_name("ch5b_initproc").unwrap()
    ));
}

pub fn add_initproc() {
    add_task(INITPROC.clone());
}

pub fn get_current_tcb_info()->CurTaskInfo{
    let current_task=current_task().unwrap();
    let inner=current_task.inner_exclusive_access();
    inner.task_info.clone()
}

pub fn update_task_info(syscall_id:usize){
    let current_task=current_task().unwrap();
    let mut inner=current_task.inner_exclusive_access();
    match syscall_id {
        SYSCALL_EXIT=>inner.task_info.sys_exit+=1,
        SYSCALL_GET_TIME=>inner.task_info.sys_time+=1,
        SYSCALL_WRITE=>inner.task_info.sys_write+=1,
        SYSCALL_TASK_INFO=>inner.task_info.sys_info+=1,
        SYSCALL_YIELD=>inner.task_info.sys_yield+=1,
        _=>(),
    }
}

pub fn current_task_mmap(_start: usize, _len: usize, _port: usize) -> isize {
    let current_task=current_task().unwrap();
    current_task.task_mmap(_start, _len, _port)
}
pub fn current_task_munmap(_start: usize, _len: usize) -> isize {
    let current_task=current_task().unwrap();
    current_task.task_munmap(_start, _len)
}