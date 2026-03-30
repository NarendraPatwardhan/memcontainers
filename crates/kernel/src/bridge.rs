// crates/kernel/src/bridge.rs
// These are the ONLY functions the kernel calls on the host.
// Every host (wasmtime, browser) must implement all of them.

#[link(wasm_import_module = "env")]
unsafe extern "C" {
    // Terminal I/O
    pub fn mc_stdout_write(ptr: *const u8, len: usize);
    pub fn mc_stderr_write(ptr: *const u8, len: usize);
    pub fn mc_stdin_read(buf: *mut u8, len: usize) -> usize;

    // Time
    pub fn mc_time_now() -> i64;
    pub fn mc_time_monotonic() -> i64;

    // Randomness
    pub fn mc_random(buf: *mut u8, len: usize);

    // HTTP
    pub fn mc_http_request(req_ptr: *const u8, req_len: usize) -> i32;
    pub fn mc_http_response_poll(handle: i32, buf: *mut u8, buf_len: usize) -> i32;
    pub fn mc_http_response_body(handle: i32, buf: *mut u8, buf_len: usize) -> i32;
    pub fn mc_http_request_close(handle: i32);

    // WebSocket
    pub fn mc_ws_connect(url_ptr: *const u8, url_len: usize) -> i32;
    pub fn mc_ws_send(handle: i32, ptr: *const u8, len: usize) -> i32;
    pub fn mc_ws_recv(handle: i32, buf: *mut u8, len: usize) -> i32;
    pub fn mc_ws_close(handle: i32);

    // Persistence
    pub fn mc_persist_get(kp: *const u8, kl: usize, vp: *mut u8, vl: usize) -> i32;
    pub fn mc_persist_put(kp: *const u8, kl: usize, vp: *const u8, vl: usize) -> i32;
    pub fn mc_persist_delete(kp: *const u8, kl: usize) -> i32;
    pub fn mc_persist_list(pp: *const u8, pl: usize, bp: *mut u8, bl: usize) -> i32;

    // Control
    pub fn mc_yield();
    pub fn mc_exit(code: i32) -> !;
    pub fn mc_log(level: i32, ptr: *const u8, len: usize);
}
