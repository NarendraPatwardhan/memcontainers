// crates/kernel/src/kernel.rs
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

pub type TaskId = u32;
pub type PipeId = u32;

pub struct KernelState {
    pub scheduler: Scheduler,
    pub tasks: BTreeMap<TaskId, Task>,
    pub pipes: BTreeMap<PipeId, Pipe>,
    pub vfs: Namespace,
    pub http_handles: BTreeMap<i32, HttpRequest>,
    pub ws_handles: BTreeMap<i32, WsConnection>,
    pub input_buf: Vec<u8>,
    pub terminal_size: (u16, u16),
    pub next_task_id: TaskId,
    pub next_pipe_id: PipeId,
    pub exit_code: Option<i32>,
}

static mut KERNEL: Option<KernelState> = None;

pub fn init(state: KernelState) {
    unsafe {
        KERNEL = Some(state);
    }
}

pub fn get() -> &'static KernelState {
    unsafe { &*core::ptr::addr_of!(KERNEL) }.as_ref().unwrap()
}

pub fn get_mut() -> &'static mut KernelState {
    unsafe { &mut *core::ptr::addr_of_mut!(KERNEL) }
        .as_mut()
        .unwrap()
}

impl KernelState {
    pub fn deliver_input(&mut self, bytes: &[u8]) {
        self.input_buf.extend_from_slice(bytes);
    }

    pub fn resize(&mut self, cols: u16, rows: u16) {
        self.terminal_size = (cols, rows);
    }
}

// Placeholder types - to be implemented
pub struct Scheduler;
pub struct Task;
pub struct Pipe;
pub struct Namespace;
pub struct HttpRequest;
pub struct WsConnection;
