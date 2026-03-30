// crates/kernel/src/builtins/text.rs
// Text processing builtins: echo, head, tail, wc

use crate::vfs::KPath;
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

use crate::shell::executor::BuiltinContext;
use crate::vfs::OpenFlags;

pub fn echo(ctx: &mut BuiltinContext) -> i32 {
    let output = ctx.args.join(" ");
    ctx.output.push_str(&output);
    ctx.output.push_str("\r\n");
    0
}

pub fn head(ctx: &mut BuiltinContext) -> i32 {
    let (path, n) = parse_head_tail_args(ctx.cwd, ctx.args);

    match path {
        Some(p) => match ctx.ns.open(&p, OpenFlags::READ) {
            Ok(mut file) => {
                let mut content = Vec::new();
                let mut buf = [0u8; 1024];
                loop {
                    match file.read(&mut buf) {
                        Ok(0) => break,
                        Ok(n_read) => content.extend_from_slice(&buf[..n_read]),
                        Err(e) => {
                            ctx.output.push_str(&alloc::format!(
                                "head: {}: {}\r\n",
                                p.as_str(),
                                fs_error_str(e)
                            ));
                            return 1;
                        }
                    }
                }

                let text = String::from_utf8_lossy(&content);
                let lines: Vec<&str> = text.lines().collect();
                let to_show = n.min(lines.len());
                for line in &lines[..to_show] {
                    ctx.output.push_str(line);
                    ctx.output.push_str("\r\n");
                }
                0
            }
            Err(e) => {
                ctx.output.push_str(&alloc::format!(
                    "head: {}: {}\r\n",
                    p.as_str(),
                    fs_error_str(e)
                ));
                1
            }
        },
        None => {
            ctx.output.push_str("head: missing file operand\r\n");
            1
        }
    }
}

pub fn tail(ctx: &mut BuiltinContext) -> i32 {
    let (path, n) = parse_head_tail_args(ctx.cwd, ctx.args);

    match path {
        Some(p) => match ctx.ns.open(&p, OpenFlags::READ) {
            Ok(mut file) => {
                let mut content = Vec::new();
                let mut buf = [0u8; 1024];
                loop {
                    match file.read(&mut buf) {
                        Ok(0) => break,
                        Ok(n_read) => content.extend_from_slice(&buf[..n_read]),
                        Err(e) => {
                            ctx.output.push_str(&alloc::format!(
                                "tail: {}: {}\r\n",
                                p.as_str(),
                                fs_error_str(e)
                            ));
                            return 1;
                        }
                    }
                }

                let text = String::from_utf8_lossy(&content);
                let lines: Vec<&str> = text.lines().collect();
                let start = if lines.len() > n { lines.len() - n } else { 0 };
                for line in &lines[start..] {
                    ctx.output.push_str(line);
                    ctx.output.push_str("\r\n");
                }
                0
            }
            Err(e) => {
                ctx.output.push_str(&alloc::format!(
                    "tail: {}: {}\r\n",
                    p.as_str(),
                    fs_error_str(e)
                ));
                1
            }
        },
        None => {
            ctx.output.push_str("tail: missing file operand\r\n");
            1
        }
    }
}

pub fn wc(ctx: &mut BuiltinContext) -> i32 {
    if ctx.args.is_empty() {
        ctx.output.push_str("wc: missing file operand\r\n");
        return 1;
    }

    let path = if ctx.args[0].starts_with('/') {
        KPath::new(&ctx.args[0])
    } else {
        KPath::new(&alloc::format!(
            "{}/{}",
            ctx.cwd.trim_end_matches('/'),
            ctx.args[0]
        ))
    };

    match ctx.ns.open(&path, OpenFlags::READ) {
        Ok(mut file) => {
            let mut content = Vec::new();
            let mut buf = [0u8; 1024];
            loop {
                match file.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => content.extend_from_slice(&buf[..n]),
                    Err(e) => {
                        ctx.output.push_str(&alloc::format!(
                            "wc: {}: {}\r\n",
                            path.as_str(),
                            fs_error_str(e)
                        ));
                        return 1;
                    }
                }
            }

            let text = String::from_utf8_lossy(&content);
            let lines = text.lines().count();
            let words: usize = text.split_whitespace().count();
            let bytes = content.len();

            ctx.output.push_str(&alloc::format!(
                "{:>8} {:>8} {:>8} {}\r\n",
                lines,
                words,
                bytes,
                ctx.args[0]
            ));
            0
        }
        Err(e) => {
            ctx.output.push_str(&alloc::format!(
                "wc: {}: {}\r\n",
                path.as_str(),
                fs_error_str(e)
            ));
            1
        }
    }
}

fn parse_head_tail_args(cwd: &str, args: &[String]) -> (Option<KPath>, usize) {
    let mut n: usize = 10;
    let mut path: Option<KPath> = None;

    let mut i = 0;
    while i < args.len() {
        if args[i] == "-n" && i + 1 < args.len() {
            if let Ok(num) = args[i + 1].parse::<usize>() {
                n = num;
            }
            i += 2;
        } else if !args[i].starts_with('-') {
            let p = if args[i].starts_with('/') {
                String::from(&args[i])
            } else {
                alloc::format!("{}/{}", cwd.trim_end_matches('/'), args[i])
            };
            path = Some(KPath::new(&p));
            i += 1;
        } else {
            i += 1;
        }
    }

    (path, n)
}

fn fs_error_str(e: crate::vfs::FsError) -> &'static str {
    match e {
        crate::vfs::FsError::NotFound => "No such file or directory",
        crate::vfs::FsError::AlreadyExists => "File exists",
        crate::vfs::FsError::NotDir => "Not a directory",
        crate::vfs::FsError::IsDir => "Is a directory",
        crate::vfs::FsError::PermissionDenied => "Permission denied",
        crate::vfs::FsError::InvalidPath => "Invalid path",
        crate::vfs::FsError::NotEmpty => "Directory not empty",
        crate::vfs::FsError::IoError => "I/O error",
        crate::vfs::FsError::BadFileDescriptor => "Bad file descriptor",
        crate::vfs::FsError::NotImplemented => "Not implemented",
    }
}
