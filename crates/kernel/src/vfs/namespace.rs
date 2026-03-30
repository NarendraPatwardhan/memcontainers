// crates/kernel/src/vfs/namespace.rs
// Mount namespace - manages mounted filesystems and resolves paths

use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::cell::UnsafeCell;

use super::traits::{
    DirEntry, FileHandle, FileSystem, FsError, KPath, Metadata, OpenFlags, Result,
};

pub struct Namespace {
    mounts: UnsafeCell<BTreeMap<String, Box<dyn FileSystem>>>,
}

impl Namespace {
    pub fn new() -> Self {
        Self {
            mounts: UnsafeCell::new(BTreeMap::new()),
        }
    }

    pub fn mount(&self, path: &str, fs: Box<dyn FileSystem>) {
        unsafe {
            (*self.mounts.get()).insert(String::from(path), fs);
        }
    }

    /// Resolve a path to (mount_point, relative_path, filesystem)
    /// Uses longest-prefix matching
    fn resolve(&self, path: &KPath) -> Option<(&str, KPath, &mut dyn FileSystem)> {
        let path_str = path.as_str();

        // Find the longest matching mount point
        let mut best_mount: Option<&String> = None;
        let mounts = unsafe { &*self.mounts.get() };

        for mount_point in mounts.keys() {
            if path_str.starts_with(mount_point) {
                if best_mount.is_none() || mount_point.len() > best_mount.unwrap().len() {
                    best_mount = Some(mount_point);
                }
            }
        }

        let mount_point = best_mount?;
        let fs = unsafe { (*self.mounts.get()).get_mut(mount_point)?.as_mut() };

        // Calculate relative path
        let rel_path = if path_str == mount_point {
            KPath::new("/")
        } else if path_str.starts_with(mount_point)
            && path_str[mount_point.len()..].starts_with('/')
        {
            KPath::new(&path_str[mount_point.len()..])
        } else {
            path.clone()
        };

        Some((mount_point, rel_path, fs))
    }

    // Delegate operations to the appropriate filesystem
    pub fn open(&self, path: &KPath, flags: OpenFlags) -> Result<Box<dyn FileHandle>> {
        let (_, rel_path, fs) = self.resolve(path).ok_or(FsError::NotFound)?;
        fs.open(&rel_path, flags)
    }

    pub fn stat(&self, path: &KPath) -> Result<Metadata> {
        let (_, rel_path, fs) = self.resolve(path).ok_or(FsError::NotFound)?;
        fs.stat(&rel_path)
    }

    pub fn readdir(&self, path: &KPath) -> Result<Vec<DirEntry>> {
        let (_, rel_path, fs) = self.resolve(path).ok_or(FsError::NotFound)?;
        fs.readdir(&rel_path)
    }

    pub fn mkdir(&self, path: &KPath) -> Result<()> {
        let (_, rel_path, fs) = self.resolve(path).ok_or(FsError::NotFound)?;
        fs.mkdir(&rel_path)
    }

    pub fn unlink(&self, path: &KPath) -> Result<()> {
        let (_, rel_path, fs) = self.resolve(path).ok_or(FsError::NotFound)?;
        fs.unlink(&rel_path)
    }

    pub fn rename(&self, from: &KPath, to: &KPath) -> Result<()> {
        // For simplicity, require both paths to be on same filesystem
        let (from_mount, from_rel, from_fs) = self.resolve(from).ok_or(FsError::NotFound)?;
        let (to_mount, to_rel, _to_fs) = self.resolve(to).ok_or(FsError::NotFound)?;

        if from_mount != to_mount {
            return Err(FsError::NotImplemented); // Cross-device rename not supported
        }

        from_fs.rename(&from_rel, &to_rel)
    }
}
