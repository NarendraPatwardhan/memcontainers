use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;
use wasmtime::{Engine, Linker, Memory, Module, Store};

mod terminal;
use terminal::Terminal;

#[derive(Parser, Debug)]
#[command(name = "host-wasmtime")]
#[command(about = "WASM host for memcontainers kernel")]
struct Cli {
    /// Path to the kernel WASM file
    #[arg(long = "kernel", value_name = "PATH")]
    kernel: PathBuf,
}

struct HostState {
    memory: Option<Memory>,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let wasm_bytes = std::fs::read(&cli.kernel)?;

    let engine = Engine::default();
    let mut linker = Linker::<HostState>::new(&engine);

    // Register all mc_* imports
    linker.func_wrap(
        "env",
        "mc_stdout_write",
        |mut caller: wasmtime::Caller<'_, HostState>, ptr: i32, len: i32| {
            if let Some(memory) = caller.data().memory {
                let data = memory.data(&caller);
                let start = ptr as usize;
                let end = start + len as usize;
                let bytes = &data[start..end];
                print!("{}", String::from_utf8_lossy(bytes));
                std::io::Write::flush(&mut std::io::stdout()).unwrap();
            }
        },
    )?;

    linker.func_wrap(
        "env",
        "mc_stderr_write",
        |mut caller: wasmtime::Caller<'_, HostState>, ptr: i32, len: i32| {
            if let Some(memory) = caller.data().memory {
                let data = memory.data(&caller);
                let start = ptr as usize;
                let end = start + len as usize;
                let bytes = &data[start..end];
                eprint!("{}", String::from_utf8_lossy(bytes));
            }
        },
    )?;

    linker.func_wrap(
        "env",
        "mc_stdin_read",
        |_caller: wasmtime::Caller<'_, HostState>, _buf: i32, _len: i32| -> i32 {
            0 // No blocking stdin read
        },
    )?;

    linker.func_wrap("env", "mc_time_now", || -> i64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64
    })?;

    linker.func_wrap("env", "mc_time_monotonic", || -> i64 {
        std::time::Instant::now().elapsed().as_millis() as i64
    })?;

    linker.func_wrap(
        "env",
        "mc_random",
        |mut caller: wasmtime::Caller<'_, HostState>, ptr: i32, len: i32| {
            if let Some(memory) = caller.data_mut().memory {
                let data = memory.data_mut(&mut caller);
                let start = ptr as usize;
                let end = start + len as usize;
                let buf = &mut data[start..end];
                for byte in buf.iter_mut() {
                    *byte = rand::random();
                }
            }
        },
    )?;

    linker.func_wrap(
        "env",
        "mc_http_request",
        |_req_ptr: i32, _req_len: i32| -> i32 {
            -1 // Not implemented
        },
    )?;

    linker.func_wrap(
        "env",
        "mc_http_response_poll",
        |_handle: i32, _buf: i32, _buf_len: i32| -> i32 { -1 },
    )?;

    linker.func_wrap(
        "env",
        "mc_http_response_body",
        |_handle: i32, _buf: i32, _buf_len: i32| -> i32 { -1 },
    )?;

    linker.func_wrap("env", "mc_http_request_close", |_handle: i32| {})?;

    linker.func_wrap(
        "env",
        "mc_ws_connect",
        |_url_ptr: i32, _url_len: i32| -> i32 { -1 },
    )?;

    linker.func_wrap(
        "env",
        "mc_ws_send",
        |_handle: i32, _ptr: i32, _len: i32| -> i32 { -1 },
    )?;

    linker.func_wrap(
        "env",
        "mc_ws_recv",
        |_handle: i32, _buf: i32, _len: i32| -> i32 { -1 },
    )?;

    linker.func_wrap("env", "mc_ws_close", |_handle: i32| {})?;

    linker.func_wrap(
        "env",
        "mc_persist_get",
        |_kp: i32, _kl: i32, _vp: i32, _vl: i32| -> i32 { -1 },
    )?;

    linker.func_wrap(
        "env",
        "mc_persist_put",
        |_kp: i32, _kl: i32, _vp: i32, _vl: i32| -> i32 { -1 },
    )?;

    linker.func_wrap("env", "mc_persist_delete", |_kp: i32, _kl: i32| -> i32 {
        -1
    })?;

    linker.func_wrap(
        "env",
        "mc_persist_list",
        |_pp: i32, _pl: i32, _bp: i32, _bl: i32| -> i32 { -1 },
    )?;

    linker.func_wrap("env", "mc_yield", || {})?;

    linker.func_wrap("env", "mc_exit", |_code: i32| -> () {
        std::process::exit(_code);
    })?;

    linker.func_wrap(
        "env",
        "mc_log",
        |mut caller: wasmtime::Caller<'_, HostState>, level: i32, ptr: i32, len: i32| {
            if let Some(memory) = caller.data().memory {
                let data = memory.data(&caller);
                let start = ptr as usize;
                let end = start + len as usize;
                let msg = String::from_utf8_lossy(&data[start..end]);
                match level {
                    0 => println!("[DEBUG] {}", msg),
                    1 => println!("[INFO] {}", msg),
                    2 => println!("[WARN] {}", msg),
                    3 => eprintln!("[ERROR] {}", msg),
                    _ => println!("[LOG] {}", msg),
                }
            }
        },
    )?;

    let mut store = Store::new(&engine, HostState { memory: None });

    let module = Module::new(&engine, &wasm_bytes)?;
    let instance = linker.instantiate(&mut store, &module)?;

    // Get memory export
    let memory = instance
        .get_export(&mut store, "memory")
        .unwrap()
        .into_memory()
        .unwrap();
    store.data_mut().memory = Some(memory);

    // Call mc_init()
    let mc_init = instance.get_typed_func::<(), i32>(&mut store, "mc_init")?;
    let _init_result = mc_init.call(&mut store, ())?;

    // Setup terminal
    let terminal = Terminal::new()?;
    let mc_input = instance.get_typed_func::<(i32, i32), ()>(&mut store, "mc_input")?;
    let mc_tick = instance.get_typed_func::<(), i32>(&mut store, "mc_tick")?;

    // Main loop: poll terminal and tick
    loop {
        // Check for keyboard input
        if let Ok(Some(key)) = terminal.read_key() {
            match key.code {
                crossterm::event::KeyCode::Char('c')
                    if key
                        .modifiers
                        .contains(crossterm::event::KeyModifiers::CONTROL) =>
                {
                    break; // Ctrl+C to exit
                }
                _ => {
                    // Convert key to bytes and send to kernel
                    let byte = match key.code {
                        crossterm::event::KeyCode::Char(c) => c as u8,
                        crossterm::event::KeyCode::Enter => b'\n',
                        crossterm::event::KeyCode::Backspace => 0x08,
                        _ => 0,
                    };
                    if byte != 0 {
                        if let Some(memory) = store.data().memory {
                            let input_buf_addr = 0x1000;
                            let data = memory.data_mut(&mut store);
                            data[input_buf_addr] = byte;
                            mc_input.call(&mut store, (input_buf_addr as i32, 1))?;
                        }
                    }
                }
            }
        }

        // Tick the kernel
        let result = mc_tick.call(&mut store, ())?;
        if result != 1 {
            break;
        }

        // Small delay to prevent spinning too fast
        std::thread::sleep(std::time::Duration::from_millis(5));
    }

    Ok(())
}
