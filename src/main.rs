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
    MissingStringDelimiter,
    UnknownOpcode,
    ClientErr { code: u16, message: String }
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

enum Packet {
    ReadRequest(RequestParts),
    WriteRequest(RequestParts),
    Data { block_num: usize, data: Vec<u8> },
    Ack(usize),
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
    let zero_index = buf.as_ref().iter().position(is_zero_byte);
    let zero_index = zero_index.ok_or(Error::MissingStringDelimiter)?;
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
    Ok(Packet::Data { block_num, data })
}

fn parse_ack_body(buf: &mut BytesMut) -> Result<Packet, Error> {
    let block_num = split_u16(buf);
    Ok(Packet::Ack(block_num as usize))
}

fn parse_error_body(buf: &mut BytesMut) -> Result<Packet, Error> {
    let code = split_u16(buf);
    let message = split_string(buf)?;
    Err(Error::ClientErr { code, message })
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
            1 => parse_request_body(buf).map(Packet::ReadRequest),
            2 => parse_request_body(buf).map(Packet::WriteRequest),
            3 => parse_data_body(buf),
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

#[cfg(test)]
mod test {
    #[test]
    fn test_decode_ack() {

    }
}
