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

#[derive(Debug)]
enum Error {
    ClientErr { code: u16, message: String },
    MissingStringDelimiter,
    UnknownOpcode,
    UnexpectedPacket(Either<Request, Data>),
}

#[derive(Debug,PartialEq)]
struct RequestParts {
    filename: String,
    mode: String,
}

impl RequestParts {
    pub fn new(filename: String, mode: String) -> RequestParts {
        RequestParts { filename, mode }
    }
}

#[derive(Debug,PartialEq)]
enum Request {
    Read(RequestParts),
    Write(RequestParts),
}

#[derive(Debug,PartialEq)]
struct Block {
    block_num: usize,
    bytes: Vec<u8>,
}

#[derive(Debug,PartialEq)]
enum Data {
    Data(Block),
    Ack(usize),
}

#[derive(Debug,PartialEq)]
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

    pub fn from_bytes(buf: &mut BytesMut) -> Option<Result<Packet, Error>> {
        if buf.len() <= 2 {
            return None;
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
            3 => parse_data_body(buf).map(|block| {
                Packet::Data(Data::Data(block))
            }),
            4 => parse_ack_body(buf),
            5 => parse_error_body(buf),
            _ => Err(Error::UnknownOpcode),
        };

        // Make sure we clear the buf so even if parsing didn't empty
        // it, the next packet will start on the opcode bytes.  This
        // also guards against a malformed dgram that has extra bytes.
        buf.clear();

        Some(packet)
    }
}

fn is_zero_byte(b: &u8) -> bool {
    *b == b'\0'
}

fn split_u16(buf: &mut BytesMut) -> u16 {
    assert!(buf.len() >= 2);
    let mut u16_buf = buf.split_to(2).into_buf();
    u16_buf.get_u16_be() // TODO: figure out if this is the right byte order
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

fn parse_data_body(buf: &mut BytesMut) -> Result<Block, Error> {
    let block_num = split_u16(buf) as usize;
    let bytes = buf.take().to_vec();
    Ok(Block { block_num, bytes })
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
        Ok(Packet::from_bytes(buf))
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
    use super::*;

    #[test]
    fn test_decode_ack() {
        let mut buf = BytesMut::from(&[0, 4, 0, 0][..]);
        match Packet::from_bytes(&mut buf) {
            None => panic!("received None"),
            Some(e @ Err(_)) => { e.unwrap(); unreachable!() },
            Some(Ok(packet)) => {
                assert_eq!(packet, Packet::Data(Data::Ack(0)))
            }
        }

    }

    #[test]
    fn test_decode_data() {
        let mut buf = BytesMut::from(&[0, 3, 0, 1, 11, 12, 13][..]);
        match Packet::from_bytes(&mut buf) {
            None => panic!("received None"),
            Some(e @ Err(_)) => { e.unwrap(); unreachable!() },
            Some(Ok(packet)) => {
                let block_num = 1;
                let bytes = vec![11, 12, 13];
                assert_eq!(packet, Packet::Data(Data::Data(Block { block_num, bytes })))
            }
        }
    }

    #[test]
    fn test_decode_error() {
        let mut buf = BytesMut::from(&[0, 5, 0, 3, 66, 97, 100, 0][..]);
        match Packet::from_bytes(&mut buf) {
            None => panic!("received None"),
            Some(Ok(packet)) => panic!("expected error, got {:?}", packet),
            Some(Err(e)) => {
                match e {
                    Error::ClientErr { code, message } => {
                        assert_eq!(code, 3);
                        assert_eq!(message, String::from("Bad"));
                    },
                    e => panic!("got unexpected error: {:?}"),
                }
            },
        }
    }

    #[test]
    fn test_decode_read_request() {
        let mut buf = BytesMut::from(&[0, 1, 70, 111, 111, 0, 66, 97, 114, 0][..]);
        match Packet::from_bytes(&mut buf) {
            None => panic!("received None"),
            Some(e @ Err(_)) => { e.unwrap(); unreachable!() },
            Some(Ok(packet)) => {
                let filename = "Foo".into();
                let mode = "Bar".into();
                let parts = RequestParts { filename, mode };
                assert_eq!(packet, Packet::Request(Request::Read(parts)))
            }
        }
    }

    #[test]
    fn test_decode_write_request() {
        let mut buf = BytesMut::from(&[0, 2, 70, 111, 111, 0, 66, 97, 114, 0][..]);
        match Packet::from_bytes(&mut buf) {
            None => panic!("received None"),
            Some(e @ Err(_)) => { e.unwrap(); unreachable!() },
            Some(Ok(packet)) => {
                let filename = "Foo".into();
                let mode = "Bar".into();
                let parts = RequestParts { filename, mode };
                assert_eq!(packet, Packet::Request(Request::Write(parts)))
            }
        }

    }

}
