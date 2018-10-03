#![feature(await_macro, async_await, futures_api)]
#![allow(dead_code, unused_variables)]

// #[macro_use]
// extern crate tokio;

use std::io;
use std::net::SocketAddr;

use bytes::{Buf, BytesMut, IntoBuf};

use tokio::net::{UdpFramed, UdpSocket};
// use tokio::prelude::*;
use tokio_io::codec::{/*Encoder, */Decoder};

enum Error {
    MissingStringDelimiters,
    UnknownOpcode,
}

struct RequestParts {
    filename: String,
    mode: String,
}

impl RequestParts {
    pub fn new(filename: String, mode: String) -> RequestParts {
        RequestParts { filename, mode }
    }
}

fn is_zero_byte(b: &u8) -> bool {
    *b == b'\0'
}

fn zero_indices(buf: &mut BytesMut) -> Result<(usize, usize), Error> {
    let buf = buf.as_ref();
    let first_zero_index = buf.iter()
        .position(is_zero_byte);
    let last_zero_index = buf.iter()
        .rposition(is_zero_byte);

    match (first_zero_index, last_zero_index) {
        (None, None) |
        (Some(_), None) |
        (None, Some(_)) => Err(Error::MissingStringDelimiters),
        (Some(i), Some(j)) => {
            if i != j {
                Ok((i, j))
            } else {
                Err(Error::MissingStringDelimiters)
            }
        }
    }
}

fn parse_request_body(buf: &mut BytesMut) -> Result<RequestParts, Error> {
    let (i, j) = zero_indices(buf)?;
    let filename_buf = buf.split_to(i);
    let mode_buf = buf.split_to(j-i);

    buf.advance(1); // drop trailing zero byte

    Ok(RequestParts {
        filename: String::from_utf8_lossy(&filename_buf).to_string(),
        mode: String::from_utf8_lossy(&mode_buf).to_string(),
    })
}

fn parse_data_body(buf: &mut BytesMut) -> Result<Packet, Error> {
    Err(Error::UnknownOpcode)
}

fn parse_error_body(buf: &mut BytesMut) -> Result<Packet, Error> {
    Err(Error::UnknownOpcode)
}

enum Packet {
    ReadRequest(RequestParts),
    WriteRequest(RequestParts),
    Data { block_num: usize, data: Vec<u8> },
    Ack,
}

struct Tftp {}

impl Decoder for Tftp {
    type Item = Result<Packet, Error>;
    type Error = io::Error;

    fn decode(&mut self, buf: &mut BytesMut) -> Result<Option<Self::Item>, io::Error> {
        if buf.len() <= 2 {
            return Ok(None);
        }

        let mut opcode = buf.split_to(2).into_buf();
        assert_eq!(0, opcode.get_u8());

        let packet = match opcode.get_u8() {
            1 => parse_request_body(buf).map(|parts| Packet::ReadRequest(parts)),
            2 => parse_request_body(buf).map(|parts| Packet::WriteRequest(parts)),
            3 => parse_data_body(buf),
            4 => Ok(Packet::Ack),
            5 => parse_error_body(buf),
            _ => Err(Error::UnknownOpcode),
        };

        Ok(Some(packet))
    }
}

fn main() {
    let addr: SocketAddr = "0.0.0.0:69".parse().unwrap();
    let listener = UdpSocket::bind(&addr).unwrap();
    let _stream = UdpFramed::new(listener, Tftp {});

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
