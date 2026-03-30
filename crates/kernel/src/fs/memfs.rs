// crates/kernel/src/fs/memfs.rs
// In-memory filesystem backed by BTreeMap

use crate::vfs::KPath;
use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::collections::BTreeSet;
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

use crate::vfs::traits::{
    DirEntry, FileHandle, FileSystem, FsError, Metadata, NodeType, OpenFlags, Result, SeekFrom,
};

#[derive(Debug, Clone)]
pub enum MemNode {
    File(Vec<u8>),
    Dir(BTreeSet<String>),
}

pub struct MemFs {
    nodes: BTreeMap<String, MemNode>,
}

impl MemFs {
    pub fn new() -> Self {
        let mut fs = Self {
            nodes: BTreeMap::new(),
        };
        // Create root directory
        fs.nodes
            .insert(String::from("/"), MemNode::Dir(BTreeSet::new()));
        fs
    }

    fn normalize_path(&self, path: &KPath) -> String {
        let s = path.as_str();
        if s.is_empty() || s == "." {
            String::from("/")
        } else if s.starts_with('/') {
            String::from(s)
        } else {
            alloc::format!("/{}", s)
        }
    }

    fn get_parent(&self, path: &str) -> Option<String> {
        if path == "/" {
            return None;
        }
        let trimmed = path.trim_end_matches('/');
        match trimmed.rfind('/') {
            Some(0) => Some(String::from("/")),
            Some(idx) => Some(String::from(&trimmed[..idx])),
            None => Some(String::from("/")),
        }
    }

    fn get_name(&self, path: &str) -> String {
        let trimmed = path.trim_end_matches('/');
        match trimmed.rfind('/') {
            Some(idx) => String::from(&trimmed[idx + 1..]),
            None => String::from(trimmed),
        }
    }

    fn ensure_dir_exists(&self, path: &str) -> Result<()> {
        match self.nodes.get(path) {
            Some(MemNode::Dir(_)) => Ok(()),
            Some(MemNode::File(_)) => Err(FsError::NotDir),
            None => Err(FsError::NotFound),
        }
    }
}

pub struct MemFileHandle {
    path: String,
    offset: u64,
    append: bool,
    nodes: *mut BTreeMap<String, MemNode>,
}

impl FileHandle for MemFileHandle {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        let offset = self.offset as usize;
        let path = self.path.clone();

        unsafe {
            match (*self.nodes).get(&path) {
                Some(MemNode::File(data)) => {
                    if offset >= data.len() {
                        return Ok(0);
                    }
                    let end = (offset + buf.len()).min(data.len());
                    let to_read = end - offset;
                    buf[..to_read].copy_from_slice(&data[offset..end]);
                    self.offset += to_read as u64;
                    Ok(to_read)
                }
                Some(MemNode::Dir(_)) => Err(FsError::IsDir),
                None => Err(FsError::NotFound),
            }
        }
    }

    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        let append = self.append;
        let offset = self.offset as usize;
        let path = self.path.clone();

        unsafe {
            match (*self.nodes).get_mut(&path) {
                Some(MemNode::File(data)) => {
                    if append {
                        data.extend_from_slice(buf);
                        self.offset = data.len() as u64;
                    } else {
                        let start = offset;
                        let end = start + buf.len();
                        if end > data.len() {
                            data.resize(end, 0);
                        }
                        data[start..end].copy_from_slice(buf);
                        self.offset = end as u64;
                    }
                    Ok(buf.len())
                }
                Some(MemNode::Dir(_)) => Err(FsError::IsDir),
                None => Err(FsError::NotFound),
            }
        }
    }

    fn seek(&mut self, pos: SeekFrom) -> Result<u64> {
        let offset = self.offset;
        let path = self.path.clone();

        unsafe {
            let size = match (*self.nodes).get(&path) {
                Some(MemNode::File(data)) => data.len() as i64,
                Some(MemNode::Dir(_)) => 0,
                None => return Err(FsError::NotFound),
            };

            let new_offset = match pos {
                SeekFrom::Start(n) => n as i64,
                SeekFrom::Current(n) => offset as i64 + n,
                SeekFrom::End(n) => size + n,
            };

            if new_offset < 0 {
                return Err(FsError::InvalidPath);
            }

            self.offset = new_offset as u64;
            Ok(self.offset)
        }
    }

    fn stat(&self) -> Result<Metadata> {
        let path = self.path.clone();
        unsafe {
            match (*self.nodes).get(&path) {
                Some(MemNode::File(data)) => Ok(Metadata {
                    node_type: NodeType::File,
                    size: data.len() as u64,
                }),
                Some(MemNode::Dir(_)) => Ok(Metadata {
                    node_type: NodeType::Dir,
                    size: 0,
                }),
                None => Err(FsError::NotFound),
            }
        }
    }
}

impl FileSystem for MemFs {
    fn open(&mut self, path: &KPath, flags: OpenFlags) -> Result<Box<dyn FileHandle>> {
        let path_str = self.normalize_path(path);
        let nodes_ptr = &mut self.nodes as *mut BTreeMap<String, MemNode>;

        // Check if file exists (using unsafe to avoid borrow issues)
        let exists = unsafe { (*nodes_ptr).contains_key(&path_str) };
        let is_file = if exists {
            matches!(
                unsafe { (*nodes_ptr).get(&path_str) },
                Some(MemNode::File(_))
            )
        } else {
            false
        };
        let is_dir = if exists {
            matches!(
                unsafe { (*nodes_ptr).get(&path_str) },
                Some(MemNode::Dir(_))
            )
        } else {
            false
        };

        if is_dir {
            return Err(FsError::IsDir);
        }

        if exists && is_file && flags.truncate {
            unsafe { (*nodes_ptr).insert(path_str.clone(), MemNode::File(Vec::new())) };
        }

        if !exists {
            if flags.create {
                let parent = self.get_parent(&path_str).ok_or(FsError::NotFound)?;
                self.ensure_dir_exists(&parent)?;

                unsafe { (*nodes_ptr).insert(path_str.clone(), MemNode::File(Vec::new())) };

                // Add to parent directory
                if let Some(MemNode::Dir(entries)) = unsafe { (*nodes_ptr).get_mut(&parent) } {
                    entries.insert(self.get_name(&path_str));
                }
            } else {
                return Err(FsError::NotFound);
            }
        }

        let offset = if flags.append {
            match unsafe { (*nodes_ptr).get(&path_str) } {
                Some(MemNode::File(data)) => data.len() as u64,
                _ => 0,
            }
        } else {
            0
        };

        let handle = MemFileHandle {
            path: path_str,
            offset,
            append: flags.append,
            nodes: nodes_ptr,
        };

        Ok(Box::new(handle))
    }

    fn stat(&self, path: &KPath) -> Result<Metadata> {
        let path_str = self.normalize_path(path);
        let node = self.nodes.get(&path_str).ok_or(FsError::NotFound)?;
        match node {
            MemNode::File(data) => Ok(Metadata {
                node_type: NodeType::File,
                size: data.len() as u64,
            }),
            MemNode::Dir(_) => Ok(Metadata {
                node_type: NodeType::Dir,
                size: 0,
            }),
        }
    }

    fn readdir(&self, path: &KPath) -> Result<Vec<DirEntry>> {
        let path_str = self.normalize_path(path);
        let node = self.nodes.get(&path_str).ok_or(FsError::NotFound)?;

        match node {
            MemNode::Dir(entries) => {
                let mut result = Vec::new();
                for name in entries {
                    let child_path = alloc::format!("{}/{}", path_str.trim_end_matches('/'), name);
                    if let Some(child) = self.nodes.get(&child_path) {
                        let node_type = match child {
                            MemNode::File(_) => NodeType::File,
                            MemNode::Dir(_) => NodeType::Dir,
                        };
                        result.push(DirEntry {
                            name: name.clone(),
                            node_type,
                        });
                    }
                }
                Ok(result)
            }
            MemNode::File(_) => Err(FsError::NotDir),
        }
    }

    fn mkdir(&mut self, path: &KPath) -> Result<()> {
        let path_str = self.normalize_path(path);

        if self.nodes.contains_key(&path_str) {
            return Err(FsError::AlreadyExists);
        }

        let parent = self.get_parent(&path_str).ok_or(FsError::NotFound)?;
        self.ensure_dir_exists(&parent)?;

        // Get name before mutable borrow
        let name = self.get_name(&path_str);

        // Create directory
        self.nodes
            .insert(path_str.clone(), MemNode::Dir(BTreeSet::new()));

        // Add to parent
        if let Some(MemNode::Dir(entries)) = self.nodes.get_mut(&parent) {
            entries.insert(name);
        }

        Ok(())
    }

    fn unlink(&mut self, path: &KPath) -> Result<()> {
        let path_str = self.normalize_path(path);

        match self.nodes.get(&path_str) {
            Some(MemNode::Dir(entries)) if !entries.is_empty() => {
                return Err(FsError::NotEmpty);
            }
            None => return Err(FsError::NotFound),
            _ => {}
        }

        let parent = self.get_parent(&path_str).ok_or(FsError::NotFound)?;
        let name = self.get_name(&path_str);

        // Remove from parent
        if let Some(MemNode::Dir(entries)) = self.nodes.get_mut(&parent) {
            entries.remove(&name);
        }

        // Remove node
        self.nodes.remove(&path_str);

        Ok(())
    }

    fn rename(&mut self, from: &KPath, to: &KPath) -> Result<()> {
        let from_str = self.normalize_path(from);
        let to_str = self.normalize_path(to);

        if !self.nodes.contains_key(&from_str) {
            return Err(FsError::NotFound);
        }

        if self.nodes.contains_key(&to_str) {
            return Err(FsError::AlreadyExists);
        }

        let from_parent = self.get_parent(&from_str).ok_or(FsError::NotFound)?;
        let to_parent = self.get_parent(&to_str).ok_or(FsError::NotFound)?;

        // Get names before mutable borrows
        let from_name = self.get_name(&from_str);
        let to_name = self.get_name(&to_str);

        // Move the node
        if let Some(node) = self.nodes.remove(&from_str) {
            self.nodes.insert(to_str.clone(), node);
        }

        // Update parent directories
        if let Some(MemNode::Dir(entries)) = self.nodes.get_mut(&from_parent) {
            entries.remove(&from_name);
        }

        if let Some(MemNode::Dir(entries)) = self.nodes.get_mut(&to_parent) {
            entries.insert(to_name);
        }

        Ok(())
    }
}
