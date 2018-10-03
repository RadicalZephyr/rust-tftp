#![feature(await_macro, async_await, futures_api)]
#![allow(dead_code, unused_variables)]

// #[macro_use]
// extern crate tokio;

use std::io;
use std::net::SocketAddr;

use bytes::{Buf, BytesMut, IntoBuf};

use tokio::net::{UdpFramed, UdpSocket};
// use tokio::prelude::*;
use tokio::prelude::future::Either;
use tokio_io::codec::{/*Encoder, */Decoder};

enum Error {
    ClientErr { code: u16, message: String },
    MissingStringDelimiter,
    UnknownOpcode,
    UnexpectedPacket(Either<Request, Data>),
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

enum Request {
    Read(RequestParts),
    Write(RequestParts),
}

enum Data {
    Data { block_num: usize, data: Vec<u8> },
    Ack(usize),
}

enum Packet {
    Request(Request),
    Data(Data),
}

impl Packet {
    pub fn into_request(self) -> Result<Request, Error> {
        match self {
            Packet::Request(request) => Ok(request),
            Packet::Data(data) => Err(Error::UnexpectedPacket(Either::B(data))),
        }
    }

    pub fn into_data(self) -> Result<Data, Error> {
        match self {
            Packet::Request(request) => Err(Error::UnexpectedPacket(Either::A(request))),
            Packet::Data(data) => Ok(data),
        }
    }
}

fn is_zero_byte(b: &u8) -> bool {
    *b == b'\0'
}

fn split_u16(buf: &mut BytesMut) -> u16 {
    assert!(buf.len() >= 2);
    let mut u16_buf = buf.split_to(2).into_buf();
    u16_buf.get_u16_le() // TODO: figure out if this is the right byte order
}

fn split_string(buf: &mut BytesMut) -> Result<String, Error> {
    let zero_index = buf.as_ref()
        .iter()
        .position(is_zero_byte)
        .ok_or(Error::MissingStringDelimiter)?;
    let str_buf = buf.split_to(zero_index);
    buf.advance(1);

    Ok(String::from_utf8_lossy(&str_buf).to_string())
}

fn parse_request_body(buf: &mut BytesMut) -> Result<RequestParts, Error> {
    let filename = split_string(buf)?;
    let mode = split_string(buf)?;
    Ok(RequestParts { filename, mode })
}

fn parse_data_body(buf: &mut BytesMut) -> Result<Packet, Error> {
    let block_num = split_u16(buf) as usize;
    let data = buf.take().to_vec();
    Ok(Packet::Data(Data::Data { block_num, data }))
}

fn parse_ack_body(buf: &mut BytesMut) -> Result<Packet, Error> {
    let block_num = split_u16(buf);
    Ok(Packet::Data(Data::Ack(block_num as usize)))
}

fn parse_error_body(buf: &mut BytesMut) -> Result<Packet, Error> {
    let code = split_u16(buf);
    let message = split_string(buf)?;
    Err(Error::ClientErr { code, message })
}

struct Tftp {
    received_end: bool,
}

impl Tftp {
    pub fn new() -> Tftp {
        Tftp { received_end: false }
    }
}

impl Decoder for Tftp {
    type Item = Result<Packet, Error>;
    type Error = io::Error;

    fn decode(&mut self, buf: &mut BytesMut) -> Result<Option<Self::Item>, io::Error> {
        if buf.len() <= 2 && self.received_end {
            return Ok(None);
        }

        let mut opcode = buf.split_to(2).into_buf();
        assert_eq!(0, opcode.get_u8());

        let packet = match opcode.get_u8() {
            1 => parse_request_body(buf).map(|parts| {
                Packet::Request(Request::Read(parts))
            }),
            2 => parse_request_body(buf).map(|parts| {
                Packet::Request(Request::Write(parts))
            }),
            3 => parse_data_body(buf).map(|data| {
                data
            }),
            4 => parse_ack_body(buf),
            5 => parse_error_body(buf),
            _ => Err(Error::UnknownOpcode),
        };

        // Make sure we clear the buf so even if parsing didn't empty
        // it, the next packet will start on the opcode bytes.  This
        // also guards against a malformed dgram that has extra bytes.
        buf.clear();

        Ok(Some(packet))
    }
}

fn main() {
    let addr: SocketAddr = "0.0.0.0:69".parse().unwrap();
    let listener = UdpSocket::bind(&addr).unwrap();
    let _stream = UdpFramed::new(listener, Tftp::new());

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

#[cfg(test)]
mod test {
    #[test]
    fn test_decode_ack() {

    }
}
