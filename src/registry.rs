use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{RwLock, RwLockReadGuard, RwLockWriteGuard, TryLockError},
};

use failure::Fail;

#[derive(Debug, Fail)]
pub enum Error {
    #[fail(display = "file lock poisoned")]
    Poisoned,

    #[fail(display = "file is already read or write locked")]
    ReadOrWriteLocked,

    #[fail(display = "file is already write locked")]
    WriteLocked,
}

pub struct ReadFile<'a>(RwLockReadGuard<'a, PathBuf>);

pub struct WriteFile<'a>(RwLockWriteGuard<'a, PathBuf>);

pub struct FileRegistry {
    root: PathBuf,
    reg: HashMap<String, RwLock<PathBuf>>,
}

impl FileRegistry {
    pub fn new() -> FileRegistry {
        let root = PathBuf::from(".");
        let reg = HashMap::new();
        FileRegistry { root, reg }
    }

    pub fn read_file(&mut self, filename: impl AsRef<str>) -> Result<ReadFile, Error> {
        let try_lock = self.file_entry(filename).try_read();
        match try_lock {
            Err(TryLockError::Poisoned(_)) => Err(Error::Poisoned),
            Err(TryLockError::WouldBlock) => Err(Error::WriteLocked),
            Ok(lock) => Ok(ReadFile(lock)),
        }
    }

    pub fn write_file(&mut self, filename: impl AsRef<str>) -> Result<WriteFile, Error> {
        let try_lock = self.file_entry(filename).try_write();
        match try_lock {
            Err(TryLockError::Poisoned(_)) => Err(Error::Poisoned),
            Err(TryLockError::WouldBlock) => Err(Error::ReadOrWriteLocked),
            Ok(lock) => Ok(WriteFile(lock)),
        }
    }

    fn file_entry(&mut self, filename: impl AsRef<str>) -> &mut RwLock<PathBuf> {
        let filename = filename.as_ref();
        let path = PathBuf::from(filename);
        let full_path = self.root.join(path);
        self.reg.entry(filename.to_string()).or_insert_with(move || {
            RwLock::new(full_path)
        })
    }
}
