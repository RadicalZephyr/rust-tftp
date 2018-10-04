#![feature(await_macro, async_await, futures_api)]
#![allow(dead_code, unused_variables)]

use std::{
    collections::HashMap,
    net::SocketAddr,
    path::PathBuf,
    sync::{RwLock, RwLockReadGuard, RwLockWriteGuard, TryLockError},
};

use log::{error, info};

use rand::prelude::*;

use tokio::await;
use tokio::net::{UdpFramed, UdpSocket};
use tokio::prelude::*;

use tftp::TftpServer;

enum Error {
    Poisoned,
    ReadOrWriteLocked,
    WriteLocked,
}

#[derive(Clone, Debug, PartialEq)]
struct Tid(u16);

impl Tid {
    pub fn new(val: u16) -> Tid {
        Tid(val)
    }

    fn random() -> Tid {
        Tid(random())
    }
}

struct Connection {
    tid: Tid,
    client_tid: Tid,
}

impl Connection {
    pub fn new(addr: &SocketAddr) -> Connection {
        let tid = Tid::random();
        let client_tid = Tid::new(addr.port());
        Connection { tid, client_tid }
    }
}

struct ReadFile<'a>(RwLockReadGuard<'a, PathBuf>);

struct WriteFile<'a>(RwLockWriteGuard<'a, PathBuf>);

struct FileRegistry {
    root: PathBuf,
    reg: HashMap<String, RwLock<PathBuf>>,
}

impl FileRegistry {
    pub fn new() -> FileRegistry {
        let root = PathBuf::from(".");
        let reg = HashMap::new();
        FileRegistry { root, reg }
    }

    pub fn read_file(&mut self, filename: String) -> Result<ReadFile, Error> {
        let try_lock = self.file_entry(filename).try_read();
        match try_lock {
            Err(TryLockError::Poisoned(_)) => Err(Error::Poisoned),
            Err(TryLockError::WouldBlock) => Err(Error::WriteLocked),
            Ok(lock) => Ok(ReadFile(lock)),
        }
    }

    pub fn write_file(&mut self, filename: String) -> Result<WriteFile, Error> {
        let try_lock = self.file_entry(filename).try_write();
        match try_lock {
            Err(TryLockError::Poisoned(_)) => Err(Error::Poisoned),
            Err(TryLockError::WouldBlock) => Err(Error::ReadOrWriteLocked),
            Ok(lock) => Ok(WriteFile(lock)),
        }
    }

    fn file_entry(&mut self, filename: String) -> &mut RwLock<PathBuf> {
        let path = PathBuf::from(&filename);
        let full_path = self.root.join(path);
        self.reg.entry(filename).or_insert_with(move || {
            RwLock::new(full_path)
        })
    }
}

fn main() {
    tokio::run_async(async {
        let addr: SocketAddr = "0.0.0.0:69".parse().unwrap();
        let listener = UdpSocket::bind(&addr).unwrap();
        let mut stream = UdpFramed::new(listener, TftpServer::new());

        while let Some(Ok((packet, addr))) = await!(stream.next()) {
            match packet {
                Err(e) => error!("{:?}", e),
                Ok(request) => {
                    info!("{:?}", request);
                    let conn = Connection::new(&addr);

                }
            }
        }
    });
}
