#![feature(await_macro, async_await, futures_api)]
#![allow(dead_code, unused_variables)]

mod decode;
pub use self::decode::{TftpClient, TftpServer, Packet, Request, AccessType, Data, Block};
