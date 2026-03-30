// crates/kernel/src/builtins/mod.rs
pub mod fs;
pub mod text;

pub use fs::{cat, cp, ls, mkdir, mv, pwd, rm, touch};
pub use text::{echo, head, tail, wc};
