#![no_std]
#![no_main]

extern crate alloc;

mod bridge;
mod kernel;
use alloc::vec::Vec;

// Static input buffer for receiving from host
use core::cell::UnsafeCell;

struct InputBuffer(UnsafeCell<Vec<u8>>);

impl InputBuffer {
    const fn new() -> Self {
        InputBuffer(UnsafeCell::new(Vec::new()))
    }

    unsafe fn get(&self) -> &mut Vec<u8> {
        &mut *self.0.get()
    }
}

unsafe impl Sync for InputBuffer {}

// Current line being typed (for backspace handling)
struct LineBuffer(UnsafeCell<Vec<u8>>);

impl LineBuffer {
    const fn new() -> Self {
        LineBuffer(UnsafeCell::new(Vec::new()))
    }

    unsafe fn get(&self) -> &mut Vec<u8> {
        &mut *self.0.get()
    }
}

unsafe impl Sync for LineBuffer {}

static INPUT_BUFFER: InputBuffer = InputBuffer::new();
static LINE_BUFFER: LineBuffer = LineBuffer::new();
static mut PROMPT_SHOWN: bool = false;

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    let msg = alloc::format!("kernel panic: {}\n", info.message());
    unsafe {
        bridge::mc_stderr_write(msg.as_ptr(), msg.len());
        core::arch::wasm32::unreachable();
    }
}

// --- Talc Global Allocator ---
#[cfg(all(not(target_feature = "atomics"), target_family = "wasm"))]
#[global_allocator]
static ALLOCATOR: talc::wasm::WasmDynamicTalc = talc::wasm::new_wasm_dynamic_allocator();

// --- Exported Functions (Host Entry Points) ---

#[unsafe(no_mangle)]
pub extern "C" fn mc_init() -> i32 {
    let msg = "memcontainers v0.1.0 booting...\n";
    unsafe {
        bridge::mc_stdout_write(msg.as_ptr(), msg.len());
    }
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn mc_tick() -> i32 {
    unsafe {
        // Show prompt on first tick
        if !PROMPT_SHOWN {
            let prompt = "$ ";
            bridge::mc_stdout_write(prompt.as_ptr(), prompt.len());
            PROMPT_SHOWN = true;
        }

        // Process any buffered input from host
        let input_buf = INPUT_BUFFER.get();
        if !input_buf.is_empty() {
            let input: Vec<u8> = input_buf.clone();
            input_buf.clear();

            let line = LINE_BUFFER.get();

            for &byte in &input {
                if byte == 0x7F || byte == 0x08 {
                    // Backspace (DEL or BS)
                    if !line.is_empty() {
                        // Echo backspace sequence: move back, space, move back
                        let bs_seq = b"\x08 \x08";
                        bridge::mc_stdout_write(bs_seq.as_ptr(), bs_seq.len());
                        line.pop();
                    }
                } else if byte == b'\n' || byte == b'\r' {
                    // Enter pressed
                    // Print carriage return + newline to move to start of next line
                    let crlf = b"\r\n";
                    bridge::mc_stdout_write(crlf.as_ptr(), crlf.len());
                    let prompt = "$ ";
                    bridge::mc_stdout_write(prompt.as_ptr(), prompt.len());
                    // Clear the current line
                    line.clear();
                } else if byte.is_ascii_graphic() || byte == b' ' {
                    // Printable character
                    // Echo it
                    bridge::mc_stdout_write(&byte as *const u8, 1);
                    // Add to current line
                    line.push(byte);
                }
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
