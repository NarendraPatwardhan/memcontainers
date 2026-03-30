// crates/kernel/src/builtins/fs.rs
// Filesystem-related builtin commands: ls, cat, mkdir, rm, cp, mv, pwd, touch

use crate::vfs::KPath;
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

use crate::shell::executor::BuiltinContext;
use crate::vfs::{FsError, OpenFlags};

pub fn pwd(ctx: &mut BuiltinContext) -> i32 {
    ctx.output.push_str(ctx.cwd);
    ctx.output.push_str("\r\r\n");
    0
}

pub fn ls(ctx: &mut BuiltinContext) -> i32 {
    let path = if ctx.args.is_empty() {
        KPath::new(ctx.cwd)
    } else {
        resolve_path(ctx.cwd, &ctx.args[0])
    };

    match ctx.ns.readdir(&path) {
        Ok(entries) => {
            for entry in entries {
                let suffix = if matches!(entry.node_type, crate::vfs::NodeType::Dir) {
                    "/"
                } else {
                    ""
                };
                ctx.output
                    .push_str(&alloc::format!("{}{}\r\n", entry.name, suffix));
            }
            0
        }
        Err(e) => {
            ctx.output.push_str(&alloc::format!(
                "ls: {}: {}\r\n",
                path.as_str(),
                fs_error_str(e)
            ));
            1
        }
    }
}

pub fn cat(ctx: &mut BuiltinContext) -> i32 {
    if ctx.args.is_empty() {
        ctx.output.push_str("cat: missing operand\r\n");
        return 1;
    }

    let mut exit_code = 0;
    for arg in ctx.args {
        let path = resolve_path(ctx.cwd, arg);
        match ctx.ns.open(&path, OpenFlags::READ) {
            Ok(mut file) => {
                let mut buf = [0u8; 1024];
                loop {
                    match file.read(&mut buf) {
                        Ok(0) => break,
                        Ok(n) => {
                            let s = String::from_utf8_lossy(&buf[..n]);
                            ctx.output.push_str(&s);
                        }
                        Err(e) => {
                            ctx.output.push_str(&alloc::format!(
                                "cat: {}: {}\r\n",
                                path.as_str(),
                                fs_error_str(e)
                            ));
                            exit_code = 1;
                            break;
                        }
                    }
                }
            }
            Err(e) => {
                ctx.output.push_str(&alloc::format!(
                    "cat: {}: {}\r\n",
                    path.as_str(),
                    fs_error_str(e)
                ));
                exit_code = 1;
            }
        }
    }
    exit_code
}

pub fn mkdir(ctx: &mut BuiltinContext) -> i32 {
    if ctx.args.is_empty() {
        ctx.output.push_str("mkdir: missing operand\r\n");
        return 1;
    }

    let mut exit_code = 0;
    for arg in ctx.args {
        let path = resolve_path(ctx.cwd, arg);
        match ctx.ns.mkdir(&path) {
            Ok(_) => {}
            Err(e) => {
                ctx.output.push_str(&alloc::format!(
                    "mkdir: cannot create directory '{}': {}\r\n",
                    path.as_str(),
                    fs_error_str(e)
                ));
                exit_code = 1;
            }
        }
    }
    exit_code
}

pub fn rm(ctx: &mut BuiltinContext) -> i32 {
    if ctx.args.is_empty() {
        ctx.output.push_str("rm: missing operand\r\n");
        return 1;
    }

    let mut exit_code = 0;
    for arg in ctx.args {
        let path = resolve_path(ctx.cwd, arg);
        match ctx.ns.unlink(&path) {
            Ok(_) => {}
            Err(e) => {
                ctx.output.push_str(&alloc::format!(
                    "rm: cannot remove '{}': {}\r\n",
                    path.as_str(),
                    fs_error_str(e)
                ));
                exit_code = 1;
            }
        }
    }
    exit_code
}

pub fn touch(ctx: &mut BuiltinContext) -> i32 {
    if ctx.args.is_empty() {
        ctx.output.push_str("touch: missing file operand\r\n");
        return 1;
    }

    let mut exit_code = 0;
    for arg in ctx.args {
        let path = resolve_path(ctx.cwd, arg);
        match ctx.ns.open(&path, OpenFlags::CREATE) {
            Ok(_) => {}
            Err(e) => {
                ctx.output.push_str(&alloc::format!(
                    "touch: cannot touch '{}': {}\r\n",
                    path.as_str(),
                    fs_error_str(e)
                ));
                exit_code = 1;
            }
        }
    }
    exit_code
}

pub fn cp(ctx: &mut BuiltinContext) -> i32 {
    if ctx.args.len() < 2 {
        ctx.output
            .push_str("cp: missing destination file operand\r\n");
        return 1;
    }

    let src = resolve_path(ctx.cwd, &ctx.args[0]);
    let dst = resolve_path(ctx.cwd, &ctx.args[1]);

    let mut content = Vec::new();
    match ctx.ns.open(&src, OpenFlags::READ) {
        Ok(mut file) => {
            let mut buf = [0u8; 1024];
            loop {
                match file.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => content.extend_from_slice(&buf[..n]),
                    Err(e) => {
                        ctx.output.push_str(&alloc::format!(
                            "cp: cannot read '{}': {}\r\n",
                            src.as_str(),
                            fs_error_str(e)
                        ));
                        return 1;
                    }
                }
            }
        }
        Err(e) => {
            ctx.output.push_str(&alloc::format!(
                "cp: cannot stat '{}': {}\r\n",
                src.as_str(),
                fs_error_str(e)
            ));
            return 1;
        }
    }

    match ctx.ns.open(&dst, OpenFlags::TRUNCATE) {
        Ok(mut file) => {
            if let Err(e) = file.write(&content) {
                ctx.output.push_str(&alloc::format!(
                    "cp: cannot write '{}': {}\r\n",
                    dst.as_str(),
                    fs_error_str(e)
                ));
                return 1;
            }
        }
        Err(e) => {
            ctx.output.push_str(&alloc::format!(
                "cp: cannot create '{}': {}\r\n",
                dst.as_str(),
                fs_error_str(e)
            ));
            return 1;
        }
    }

    0
}

pub fn mv(ctx: &mut BuiltinContext) -> i32 {
    if ctx.args.len() < 2 {
        ctx.output
            .push_str("mv: missing destination file operand\r\n");
        return 1;
    }

    let src = resolve_path(ctx.cwd, &ctx.args[0]);
    let dst = resolve_path(ctx.cwd, &ctx.args[1]);

    match ctx.ns.rename(&src, &dst) {
        Ok(_) => 0,
        Err(e) => {
            ctx.output.push_str(&alloc::format!(
                "mv: cannot move '{}': {}\r\n",
                src.as_str(),
                fs_error_str(e)
            ));
            1
        }
    }
}

fn resolve_path(cwd: &str, path: &str) -> KPath {
    if path.starts_with('/') {
        KPath::new(path)
    } else {
        KPath::new(&alloc::format!("{}/{}", cwd.trim_end_matches('/'), path))
    }
}

fn fs_error_str(e: FsError) -> &'static str {
    match e {
        FsError::NotFound => "No such file or directory",
        FsError::AlreadyExists => "File exists",
        FsError::NotDir => "Not a directory",
        FsError::IsDir => "Is a directory",
        FsError::PermissionDenied => "Permission denied",
        FsError::InvalidPath => "Invalid path",
        FsError::NotEmpty => "Directory not empty",
        FsError::IoError => "I/O error",
        FsError::BadFileDescriptor => "Bad file descriptor",
        FsError::NotImplemented => "Not implemented",
    }
}
