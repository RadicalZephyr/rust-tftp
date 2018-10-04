use std::io;
use std::result::Result as StdResult;

use bytes::{Buf, BytesMut, IntoBuf};

// use tokio::prelude::*;
use tokio::prelude::future::Either;
use tokio_io::codec::{/*Encoder, */Decoder};

type Result<T> = StdResult<T, Error>;

#[derive(Debug)]
pub enum Error {
    ClientErr { code: u16, message: String },
    MissingStringDelimiter,
    UnknownOpcode,
    UnexpectedPacket(Either<Request, Data>),
}

#[derive(Clone, Debug,PartialEq)]
pub enum AccessType {
    Read,
    Write,
}

#[derive(Clone, Debug,PartialEq)]
pub struct Request {
    r#type: AccessType,
    filename: String,
    mode: String,
}

impl Request {
    pub fn new(r#type: AccessType, filename: String, mode: String) -> Request {
        Request { r#type, filename, mode }
    }

    pub fn r#type(&self) -> AccessType {
        self.r#type.clone()
    }
}

#[derive(Debug,PartialEq)]
pub struct Block {
    block_num: usize,
    bytes: Vec<u8>,
}

#[derive(Debug,PartialEq)]
pub enum Data {
    Data(Block),
    Ack(usize),
}

#[derive(Debug,PartialEq)]
pub enum Packet {
    Request(Request),
    Data(Data),
}

impl Packet {
    pub fn into_request(self) -> Result<Request> {
        match self {
            Packet::Request(request) => Ok(request),
            Packet::Data(data) => Err(Error::UnexpectedPacket(Either::B(data))),
        }
    }

    pub fn into_data(self) -> Result<Data> {
        match self {
            Packet::Request(request) => Err(Error::UnexpectedPacket(Either::A(request))),
            Packet::Data(data) => Ok(data),
        }
    }

    pub fn from_bytes(buf: &mut BytesMut) -> Option<Result<Packet>> {
        if buf.len() <= 2 {
            return None;
        }

        let mut opcode = buf.split_to(2).into_buf();
        assert_eq!(0, opcode.get_u8());

        let packet = match opcode.get_u8() {
            1 => parse_request_body(AccessType::Read, buf).map(|request| {
                Packet::Request(request)
            }),
            2 => parse_request_body(AccessType::Write, buf).map(|request| {
                Packet::Request(request)
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

fn split_u16(buf: &mut BytesMut) -> u16 {
    assert!(buf.len() >= 2);
    let mut u16_buf = buf.split_to(2).into_buf();
    u16_buf.get_u16_be() // TODO: figure out if this is the right byte order
}

fn split_string(buf: &mut BytesMut) -> Result<String> {
    let zero_index = buf.as_ref()
        .iter()
        .position(|b| *b == b'\0')
        .ok_or(Error::MissingStringDelimiter)?;
    let str_buf = buf.split_to(zero_index);
    buf.advance(1);

    Ok(String::from_utf8_lossy(&str_buf).to_string())
}

fn parse_request_body(r#type: AccessType, buf: &mut BytesMut) -> Result<Request> {
    let filename = split_string(buf)?;
    let mode = split_string(buf)?;
    Ok(Request { r#type, filename, mode })
}

fn parse_data_body(buf: &mut BytesMut) -> Result<Block> {
    let block_num = split_u16(buf) as usize;
    let bytes = buf.take().to_vec();
    Ok(Block { block_num, bytes })
}

fn parse_ack_body(buf: &mut BytesMut) -> Result<Packet> {
    let block_num = split_u16(buf);
    Ok(Packet::Data(Data::Ack(block_num as usize)))
}

fn parse_error_body(buf: &mut BytesMut) -> Result<Packet> {
    let code = split_u16(buf);
    let message = split_string(buf)?;
    Err(Error::ClientErr { code, message })
}

pub struct TftpClient {
    received_end: bool,
}

impl TftpClient {
    pub fn new() -> TftpClient {
        TftpClient { received_end: false }
    }
}

impl Decoder for TftpClient {
    type Item = Result<Data>;
    type Error = io::Error;

    fn decode(&mut self, buf: &mut BytesMut) -> StdResult<Option<Self::Item>, io::Error> {
        if self.received_end {
            return Ok(None);
        }

        match Packet::from_bytes(buf) {
            None => Ok(None),
            Some(res) => {
                let data = res.and_then(|packet| {
                    let data_res: Result<Data> = Packet::into_data(packet);
                    data_res.map(|data: Data| {
                        if let Data::Data(block) = &data {
                            self.received_end = true;
                        }
                        data
                    })
                });
                Ok(Some(data))
            }
        }
    }
}

pub struct TftpServer {}

impl TftpServer {
    pub fn new() -> TftpServer {
        TftpServer {}
    }
}

impl Decoder for TftpServer {
    type Item = Result<Request>;
    type Error = io::Error;

    fn decode(&mut self, buf: &mut BytesMut) -> StdResult<Option<Self::Item>, io::Error> {
        match Packet::from_bytes(buf) {
            None => Ok(None),
            Some(res) => Ok(Some(res.and_then(Packet::into_request)))
        }
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
                let r#type = AccessType::Read;
                let filename = "Foo".into();
                let mode = "Bar".into();
                let request = Request { r#type, filename, mode };
                assert_eq!(packet, Packet::Request(request));
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
                let r#type = AccessType::Write;
                let filename = "Foo".into();
                let mode = "Bar".into();
                let request = Request { r#type, filename, mode };
                assert_eq!(packet, Packet::Request(request));
            }
        }
    }
}
