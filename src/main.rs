#![feature(await_macro, async_await, futures_api)]

#[macro_use]
extern crate tokio;

use std::net::SocketAddr;

use tokio::net::{TcpListener, TcpStream};
use tokio::prelude::*;

fn main() {
    let addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();
    let listener = TcpListener::bind(&addr).unwrap();

    tokio::run_async(async {
        let mut incoming = listener.incoming();

        while let Some(stream) = await!(incoming.next()) {
            let stream = stream.unwrap();
            handle(stream);
        }
    });
}

fn handle(mut stream: TcpStream) {
    tokio::spawn_async(async move {
        let mut buf = [0; 1024];

        loop {
            match await!(stream.read_async(&mut buf)).unwrap() {
                0 => break, // Socket closed
                n => {
                    // Send the data back
                    await!(stream.write_all_async(&buf[0..n])).unwrap();
                }
            }
        }
    });
}
