#![feature(await_macro, async_await, futures_api)]
#![allow(dead_code, unused_variables)]

use std::net::SocketAddr;

use log::{error, info};

use tokio::await;
use tokio::net::{UdpFramed, UdpSocket};
use tokio::prelude::*;

use tftp::TftpServer;

#[derive(Clone, Debug, PartialEq)]
struct Tid(u16);

impl Tid {
    pub fn new(val: u16) -> Tid {
        Tid(val)
    }
}

struct Connection {
    tid: Tid,
    client_tid: Tid,
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
                }
            }
        }
    });
}
