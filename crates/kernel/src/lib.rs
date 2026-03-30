#![no_std]
#![no_main]

extern crate alloc;

mod bridge;
mod builtins;
mod fs;
mod kernel;
mod shell;
mod vfs;

use alloc::boxed::Box;
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

use core::cell::UnsafeCell;

use builtins::{cat, cp, echo, head, ls, mkdir, mv, pwd, rm, tail, touch, wc};
use fs::MemFs;
use shell::{parse_line, Executor};
use vfs::{KPath, Namespace};

// Static system state
struct SystemState {
    ns: UnsafeCell<Option<Namespace>>,
    executor: UnsafeCell<Option<Executor>>,
    cwd: UnsafeCell<String>,
}

impl SystemState {
    const fn new() -> Self {
        Self {
            ns: UnsafeCell::new(None),
            executor: UnsafeCell::new(None),
            cwd: UnsafeCell::new(String::new()),
        }
    }

    unsafe fn ns(&self) -> &mut Namespace {
        (*self.ns.get()).as_mut().unwrap()
    }

    unsafe fn executor(&self) -> &mut Executor {
        (*self.executor.get()).as_mut().unwrap()
    }

    unsafe fn cwd(&self) -> &mut String {
        &mut *self.cwd.get()
    }
}

unsafe impl Sync for SystemState {}

static STATE: SystemState = SystemState::new();

// Input/line buffers for terminal handling
struct InputBuffer(UnsafeCell<Vec<u8>>);
struct LineBuffer(UnsafeCell<Vec<u8>>);

impl InputBuffer {
    const fn new() -> Self {
        InputBuffer(UnsafeCell::new(Vec::new()))
    }
    unsafe fn get(&self) -> &mut Vec<u8> {
        &mut *self.0.get()
    }
}

impl LineBuffer {
    const fn new() -> Self {
        LineBuffer(UnsafeCell::new(Vec::new()))
    }
    unsafe fn get(&self) -> &mut Vec<u8> {
        &mut *self.0.get()
    }
}

unsafe impl Sync for InputBuffer {}
unsafe impl Sync for LineBuffer {}

static INPUT_BUFFER: InputBuffer = InputBuffer::new();
static LINE_BUFFER: LineBuffer = LineBuffer::new();
static mut PROMPT_SHOWN: bool = false;
static mut INITIALIZED: bool = false;

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    let msg = alloc::format!("kernel panic: {}\r\n", info.message());
    unsafe {
        bridge::mc_stderr_write(msg.as_ptr(), msg.len());
        core::arch::wasm32::unreachable();
    }
}

// --- Talc Global Allocator ---
#[cfg(all(not(target_feature = "atomics"), target_family = "wasm"))]
#[global_allocator]
static ALLOCATOR: talc::wasm::WasmDynamicTalc = talc::wasm::new_wasm_dynamic_allocator();

fn init_system() {
    unsafe {
        if INITIALIZED {
            return;
        }

        // Create namespace and mount root filesystem
        let mut ns = Namespace::new();
        let memfs = Box::new(MemFs::new());
        ns.mount("/", memfs);

        // Create default directories
        let _ = ns.mkdir(&KPath::new("/home"));
        let _ = ns.mkdir(&KPath::new("/home/user"));
        let _ = ns.mkdir(&KPath::new("/tmp"));
        let _ = ns.mkdir(&KPath::new("/etc"));

        // Set cwd to /home/user
        *STATE.cwd.get() = String::from("/home/user");

        // Create executor and register builtins
        let mut executor = Executor::new();
        executor.register("pwd", pwd);
        executor.register("ls", ls);
        executor.register("cat", cat);
        executor.register("mkdir", mkdir);
        executor.register("rm", rm);
        executor.register("cp", cp);
        executor.register("mv", mv);
        executor.register("touch", touch);
        executor.register("echo", echo);
        executor.register("head", head);
        executor.register("tail", tail);
        executor.register("wc", wc);

        // Store in global state
        *STATE.ns.get() = Some(ns);
        *STATE.executor.get() = Some(executor);

        INITIALIZED = true;
    }
}

// --- Exported Functions (Host Entry Points) ---

#[unsafe(no_mangle)]
pub extern "C" fn mc_init() -> i32 {
    unsafe {
        let msg = "memcontainers v0.1.0 booting...\r\nMounting root filesystem... ok\r\nCreating default directories... ok\r\n";
        bridge::mc_stdout_write(msg.as_ptr(), msg.len());

        init_system();

        // Show initial prompt
        let prompt = "$ ";
        bridge::mc_stdout_write(prompt.as_ptr(), prompt.len());
        PROMPT_SHOWN = true;
    }
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn mc_tick() -> i32 {
    unsafe {
        if !INITIALIZED {
            init_system();
        }

        // Process any buffered input from host
        let input_buf = INPUT_BUFFER.get();
        if !input_buf.is_empty() {
            let input: Vec<u8> = input_buf.clone();
            input_buf.clear();

            let line = LINE_BUFFER.get();
            let mut enter_pressed = false;

            for &byte in &input {
                if byte == 0x7F || byte == 0x08 {
                    // Backspace
                    if !line.is_empty() {
                        let bs_seq = b"\x08 \x08";
                        bridge::mc_stdout_write(bs_seq.as_ptr(), bs_seq.len());
                        line.pop();
                    }
                } else if byte == b'\n' || byte == b'\r' {
                    // Enter pressed - mark for execution
                    enter_pressed = true;
                } else if byte.is_ascii_graphic() || byte == b' ' {
                    // Printable character
                    bridge::mc_stdout_write(&byte as *const u8, 1);
                    line.push(byte);
                }
            }

            // Execute command if Enter was pressed
            if enter_pressed {
                // Newline
                let crlf = b"\r\n";
                bridge::mc_stdout_write(crlf.as_ptr(), crlf.len());

                if !line.is_empty() {
                    // Convert line to string and parse
                    let line_str = String::from_utf8_lossy(line);

                    if let Some(cmd) = parse_line(&line_str) {
                        let ns = STATE.ns();
                        let executor = STATE.executor();
                        let cwd = STATE.cwd();

                        let (exit_code, output) = executor.execute(&cmd, ns, cwd);

                        // Handle output redirection
                        if let Some((path, append)) = cmd.redirect {
                            let redirect_path = if path.starts_with('/') {
                                path
                            } else {
                                alloc::format!("{}/{}", cwd.trim_end_matches('/'), path)
                            };

                            let flags = if append {
                                vfs::OpenFlags::APPEND
                            } else {
                                vfs::OpenFlags::TRUNCATE
                            };

                            match ns.open(&KPath::new(&redirect_path), flags) {
                                Ok(mut file) => {
                                    let _ = file.write(output.as_bytes());
                                }
                                Err(_) => {
                                    let err =
                                        alloc::format!("{}: cannot redirect\r\n", redirect_path);
                                    bridge::mc_stderr_write(err.as_ptr(), err.len());
                                }
                            }
                        } else {
                            // Write output to stdout
                            bridge::mc_stdout_write(output.as_ptr(), output.len());
                        }
                    }
                }

                // Clear line and show new prompt
                line.clear();
                let prompt = "$ ";
                bridge::mc_stdout_write(prompt.as_ptr(), prompt.len());
            }
        }
    }
    1 // Keep looping
}

#[unsafe(no_mangle)]
pub extern "C" fn mc_input(ptr: *const u8, len: usize) {
    unsafe {
        let bytes = core::slice::from_raw_parts(ptr, len);
        INPUT_BUFFER.get().extend_from_slice(bytes);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn mc_resize(_cols: i32, _rows: i32) {
    // TODO: Handle terminal resize
}
