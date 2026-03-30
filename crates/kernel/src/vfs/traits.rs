// crates/kernel/src/vfs/traits.rs
// Virtual File System traits - defines the interface for all filesystems

use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;

pub type Result<T> = core::result::Result<T, FsError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FsError {
    NotFound,
    AlreadyExists,
    NotDir,
    IsDir,
    PermissionDenied,
    InvalidPath,
    NotEmpty,
    IoError,
    BadFileDescriptor,
    NotImplemented,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeType {
    File,
    Dir,
}

#[derive(Debug, Clone)]
pub struct Metadata {
    pub node_type: NodeType,
    pub size: u64,
}

#[derive(Debug, Clone)]
pub struct DirEntry {
    pub name: String,
    pub node_type: NodeType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OpenFlags {
    pub read: bool,
    pub write: bool,
    pub create: bool,
    pub truncate: bool,
    pub append: bool,
}

impl OpenFlags {
    pub const READ: Self = Self {
        read: true,
        write: false,
        create: false,
        truncate: false,
        append: false,
    };
    pub const WRITE: Self = Self {
        read: false,
        write: true,
        create: false,
        truncate: false,
        append: false,
    };
    pub const CREATE: Self = Self {
        read: false,
        write: true,
        create: true,
        truncate: false,
        append: false,
    };
    pub const TRUNCATE: Self = Self {
        read: false,
        write: true,
        create: true,
        truncate: true,
        append: false,
    };
    pub const APPEND: Self = Self {
        read: false,
        write: true,
        create: true,
        truncate: false,
        append: true,
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeekFrom {
    Start(u64),
    Current(i64),
    End(i64),
}

// Simple path type for no_std
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct KPath(pub String);

impl KPath {
    pub fn new(path: &str) -> Self {
        KPath(String::from(path))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn parent(&self) -> Option<KPath> {
        let s = self.0.trim_end_matches('/');
        match s.rfind('/') {
            Some(0) => Some(KPath::new("/")),
            Some(idx) => Some(KPath::new(&s[..idx])),
            None => None,
        }
    }

    pub fn name(&self) -> &str {
        let s = self.0.trim_end_matches('/');
        match s.rfind('/') {
            Some(idx) => &s[idx + 1..],
            None => s,
        }
    }

    pub fn join(&self, other: &str) -> KPath {
        if other.starts_with('/') {
            KPath::new(other)
        } else if self.0.ends_with('/') {
            KPath::new(&alloc::format!("{}{}", self.0, other))
        } else {
            KPath::new(&alloc::format!("{}/{}", self.0, other))
        }
    }
}

pub trait FileHandle {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize>;
    fn write(&mut self, buf: &[u8]) -> Result<usize>;
    fn seek(&mut self, pos: SeekFrom) -> Result<u64>;
    fn stat(&self) -> Result<Metadata>;
}

pub trait FileSystem {
    fn open(&mut self, path: &KPath, flags: OpenFlags) -> Result<Box<dyn FileHandle>>;
    fn stat(&self, path: &KPath) -> Result<Metadata>;
    fn readdir(&self, path: &KPath) -> Result<Vec<DirEntry>>;
    fn mkdir(&mut self, path: &KPath) -> Result<()>;
    fn unlink(&mut self, path: &KPath) -> Result<()>;
    fn rename(&mut self, from: &KPath, to: &KPath) -> Result<()>;
}
