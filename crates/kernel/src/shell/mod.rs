// crates/kernel/src/shell/mod.rs
pub mod executor;
pub mod parser;

pub use executor::{BuiltinContext, Executor};
pub use parser::{parse_line, Command};
