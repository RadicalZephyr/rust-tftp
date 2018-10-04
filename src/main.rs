#![feature(await_macro, async_await, futures_api)]
#![allow(dead_code, unused_variables)]

// #[macro_use]
// extern crate tokio;

use std::net::SocketAddr;

use tokio::net::{UdpFramed, UdpSocket};
// use tokio::prelude::*;

use tftp::TftpServer;

fn main() {
    let addr: SocketAddr = "0.0.0.0:69".parse().unwrap();
    let listener = UdpSocket::bind(&addr).unwrap();
    let _stream = UdpFramed::new(listener, TftpServer::new());

    tokio::run_async(async {
    });
}

#[derive(Clone)]
struct Tid(u16);

impl Tid {
    pub fn new(val: u16) -> Tid {
        Tid(val)
    }
}
