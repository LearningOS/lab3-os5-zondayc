//! Implementation of [`TaskManager`]
//!
//! It is only used to manage processes and schedule process based on ready queue.
//! Other CPU process monitoring functions are in Processor.


use super::TaskControlBlock;
use crate::sync::UPSafeCell;
use alloc::collections::VecDeque;
use alloc::sync::Arc;
use lazy_static::*;

pub struct TaskManager {
    ready_queue: VecDeque<Arc<TaskControlBlock>>,
}

// YOUR JOB: FIFO->Stride
/// A simple FIFO scheduler.
impl TaskManager {
    pub fn new() -> Self {
        Self {
            ready_queue: VecDeque::new(),
        }
    }
    /// Add process back to ready queue
    pub fn add(&mut self, task: Arc<TaskControlBlock>) {
        self.ready_queue.push_back(task);
    }
    /// Take a process out of the ready queue
    pub fn fetch(&mut self) -> Option<Arc<TaskControlBlock>> {
        self.ready_queue.pop_front()
    }

    pub fn stride_fetch(&mut self) -> Option<Arc<TaskControlBlock>> {
        let index=self.find_min_pass_index();
        let task=self.ready_queue.get_mut(index as usize).unwrap();
        let mut inner=task.inner_exclusive_access();
        inner.pass+=inner.stride as u128;
        drop(inner);
        drop(task);
        self.ready_queue.remove(index as usize)

    }

    pub fn find_min_pass_index(&mut self)->i32{//这里可以直接遍历，但是也应该可以实现一个partialEq 的trait然后调用iter().min()找到最小的那个
        let mut index=0;
        let mut ret=0;
        let mut min=self.ready_queue.get(0).unwrap().inner_exclusive_access().pass;
        for task in self.ready_queue.iter(){
            match task.inner_exclusive_access().pass{
                a if a<min =>{
                    min=a;
                    ret=index;
                }
                _=>{}
            }
            index+=1;
        }
        ret
    }

}

lazy_static! {
    /// TASK_MANAGER instance through lazy_static!
    pub static ref TASK_MANAGER: UPSafeCell<TaskManager> =
        unsafe { UPSafeCell::new(TaskManager::new()) };
}

pub fn add_task(task: Arc<TaskControlBlock>) {
    TASK_MANAGER.exclusive_access().add(task);
}

pub fn fetch_task() -> Option<Arc<TaskControlBlock>> {
    TASK_MANAGER.exclusive_access().stride_fetch()
}
