#![no_std]
#![no_main]

extern crate alloc;

mod bridge;
mod kernel;

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    // Best-effort: write panic message to stderr via host bridge, then trap.
    let msg = alloc::format!("kernel panic: {}\n", info.message());
    unsafe {
        bridge::mc_stderr_write(msg.as_ptr(), msg.len());
        core::arch::wasm32::unreachable();
    }
}

// --- Talc Global Allocator ---
// Talc provides a purpose-built WASM allocator that automatically manages
// WebAssembly memory growth. No manual heap setup required.
// This replaces the abandoned wee_alloc with something faster and correct.

#[cfg(all(not(target_feature = "atomics"), target_family = "wasm"))]
#[global_allocator]
static ALLOCATOR: talc::wasm::WasmDynamicTalc = talc::wasm::new_wasm_dynamic_allocator();

// --- Exported Functions (Host Entry Points) ---

static mut TICK_COUNT: u32 = 0;

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
        if TICK_COUNT == 0 {
            let prompt = "$ ";
            bridge::mc_stdout_write(prompt.as_ptr(), prompt.len());
        }
        TICK_COUNT += 1;
    }
    1
}

#[unsafe(no_mangle)]
pub extern "C" fn mc_input(ptr: *const u8, len: usize) {
    let bytes = unsafe { core::slice::from_raw_parts(ptr, len) };
    kernel::get_mut().deliver_input(bytes);
}

#[unsafe(no_mangle)]
pub extern "C" fn mc_resize(cols: i32, rows: i32) {
    kernel::get_mut().resize(cols as u16, rows as u16);
}
