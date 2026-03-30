// crates/kernel/src/shell/executor.rs
// Command execution - builtin registry and dispatch

use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

use super::parser::Command;
use crate::vfs::KPath;
use crate::vfs::Namespace;

pub type BuiltinFn = fn(&mut BuiltinContext) -> i32;

pub struct BuiltinContext<'a> {
    pub args: &'a [String],
    pub ns: &'a Namespace,
    pub cwd: &'a mut String,
    pub output: &'a mut String,
}

pub struct Executor {
    builtins: BTreeMap<String, BuiltinFn>,
}

impl Executor {
    pub fn new() -> Self {
        Self {
            builtins: BTreeMap::new(),
        }
    }

    pub fn register(&mut self, name: &str, func: BuiltinFn) {
        self.builtins.insert(String::from(name), func);
    }

    pub fn execute(&self, cmd: &Command, ns: &Namespace, cwd: &mut String) -> (i32, String) {
        let mut output = String::new();

        if let Some(builtin) = self.builtins.get(&cmd.cmd) {
            let mut ctx = BuiltinContext {
                args: &cmd.args,
                ns,
                cwd,
                output: &mut output,
            };
            let exit_code = builtin(&mut ctx);
            (exit_code, output)
        } else {
            (1, alloc::format!("{}: command not found\n", cmd.cmd))
        }
    }
}
