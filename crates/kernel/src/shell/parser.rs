// crates/kernel/src/shell/parser.rs
// Simple command line parser - handles commands, arguments, and redirection

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

#[derive(Debug, Clone)]
pub struct Command {
    pub cmd: String,
    pub args: Vec<String>,
    pub redirect: Option<(String, bool)>, // (path, append)
}

pub fn parse_line(line: &str) -> Option<Command> {
    let line = line.trim();
    if line.is_empty() {
        return None;
    }

    // Check for redirection
    let mut redirect: Option<(String, bool)> = None;
    let mut remaining = line;

    // Look for >> (append) first
    if let Some(pos) = remaining.find(">>") {
        let (before, after) = remaining.split_at(pos);
        remaining = before.trim();
        let path = String::from(after[2..].trim());
        redirect = Some((path, true));
    } else if let Some(pos) = remaining.find('>') {
        let (before, after) = remaining.split_at(pos);
        remaining = before.trim();
        let path = String::from(after[1..].trim());
        redirect = Some((path, false));
    }

    // Split remaining into command and args
    let parts: Vec<&str> = remaining.split_whitespace().collect();
    if parts.is_empty() {
        return None;
    }

    let cmd = String::from(parts[0]);
    let args: Vec<String> = parts[1..].iter().map(|s| String::from(*s)).collect();

    Some(Command {
        cmd,
        args,
        redirect,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple() {
        let cmd = parse_line("ls -la /tmp").unwrap();
        assert_eq!(cmd.cmd, "ls");
        assert_eq!(cmd.args, vec!["-la", "/tmp"]);
        assert!(cmd.redirect.is_none());
    }

    #[test]
    fn test_parse_redirect() {
        let cmd = parse_line("echo hello > output.txt").unwrap();
        assert_eq!(cmd.cmd, "echo");
        assert_eq!(cmd.args, vec!["hello"]);
        assert_eq!(cmd.redirect, Some((String::from("output.txt"), false)));
    }

    #[test]
    fn test_parse_append() {
        let cmd = parse_line("echo world >> log.txt").unwrap();
        assert_eq!(cmd.cmd, "echo");
        assert_eq!(cmd.args, vec!["world"]);
        assert_eq!(cmd.redirect, Some((String::from("log.txt"), true)));
    }

    #[test]
    fn test_parse_empty() {
        assert!(parse_line("").is_none());
        assert!(parse_line("   ").is_none());
    }
}
