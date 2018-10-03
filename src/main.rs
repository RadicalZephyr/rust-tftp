#![feature(await_macro, async_await, futures_api)]

#[macro_use]
extern crate tokio;

use std::io;
use std::net::SocketAddr;

use bytes::{BytesMut, BufMut};

use tokio::net::{UdpFramed, UdpSocket};
use tokio::prelude::*;
use tokio_io::codec::{Encoder, Decoder};

struct Error {}

struct Tftp {}

impl Decoder for Tftp {
    type Item = Box<dyn Request>;
    type Error = io::Error;

    fn decode(&mut self, buf: &mut BytesMut) -> Result<Option<Self::Item>, io::Error> {
        Ok(Some(Box::new(ReadRequest { tid: Tid(10) })))
    }
}

fn main() {
    let addr: SocketAddr = "0.0.0.0:69".parse().unwrap();
    let listener = UdpSocket::bind(&addr).unwrap();
//    let stream = UdpFramed::new(listener, );

    tokio::run_async(async {

    });
}

#[derive(Clone)]
struct Tid(usize);

impl Tid {
    pub fn new(val: usize) -> Tid {
        Tid(val)
    }
}

trait Request {
    fn tid(&self) -> Tid;
}

struct ReadRequest {
    tid: Tid,

}

impl Request for ReadRequest {
    fn tid(&self) -> Tid {
        self.tid.clone()
    }
}
