#![feature(await_macro, async_await, futures_api)]
#![allow(dead_code, unused_variables)]

mod decode;
pub use self::decode::{Packet, Request, AccessType, Data, Block, Error as DecodeError};

mod registry;
pub use self::registry::{FileRegistry, ReadFile, WriteFile, Error as RegistryError};

use std::io;
use std::result::Result as StdResult;

use bytes::BytesMut;

use tokio_io::codec::Decoder;


pub struct TftpClient {
    received_end: bool,
}

impl TftpClient {
    pub fn new() -> TftpClient {
        TftpClient { received_end: false }
    }
}

impl Decoder for TftpClient {
    type Item = decode::Result<Data>;
    type Error = io::Error;

    fn decode(&mut self, buf: &mut BytesMut) -> StdResult<Option<Self::Item>, io::Error> {
        if self.received_end {
            return Ok(None);
        }

        match Packet::from_bytes(buf) {
            None => Ok(None),
            Some(res) => {
                let data = res.and_then(|packet| {
                    let data_res: decode::Result<Data> = Packet::into_data(packet);
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
    type Item = decode::Result<Request>;
    type Error = io::Error;

    fn decode(&mut self, buf: &mut BytesMut) -> StdResult<Option<Self::Item>, io::Error> {
        match Packet::from_bytes(buf) {
            None => Ok(None),
            Some(res) => Ok(Some(res.and_then(Packet::into_request)))
        }
    }
}
