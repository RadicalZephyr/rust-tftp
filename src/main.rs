#![feature(await_macro, async_await, futures_api)]
#![allow(dead_code, unused_variables)]

use std::net::SocketAddr;

use log::{error, info};

use failure::Fail;

use rand::prelude::*;

use tokio::await;
use tokio::net::{UdpFramed, UdpSocket};
use tokio::prelude::*;

use tftp::*;

#[derive(Debug, Fail)]
enum Error {
    #[fail(display = "decode error")]
    Decode(#[cause] DecodeError),

    #[fail(display = "registry error")]
    Registry(#[cause] RegistryError),
}

impl From<DecodeError> for Error {
    fn from(error: DecodeError) -> Error {
        Error::Decode(error)
    }
}

impl From<RegistryError> for Error {
    fn from(error: RegistryError) -> Error {
        Error::Registry(error)
    }
}

#[derive(Clone, Debug, PartialEq)]
struct Tid(u16);

impl Tid {
    pub fn new(val: u16) -> Tid {
        Tid(val)
    }

    fn random() -> Tid {
        Tid(random())
    }
}

struct Connection {
    tid: Tid,
    client_addr: SocketAddr,
}

impl Connection {
    pub fn new(client_addr: SocketAddr) -> Connection {
        let tid = Tid::random();
        Connection { tid, client_addr }
    }
}

fn handle_read(conn: Connection, file: ReadFile) -> () {
    tokio::run_async(async move {
        let listener = UdpSocket::bind(&conn.client_addr).unwrap();
        let mut stream = UdpFramed::new(listener, TftpClient::new());
        while let Some(Ok((request, addr))) = await!(stream.next()) {

        }
    })
}

fn handle_write(conn: Connection, file: WriteFile) -> () {
    tokio::run_async(async move {
        let listener = UdpSocket::bind(&conn.client_addr).unwrap();
        let mut stream = UdpFramed::new(listener, TftpClient::new());
        while let Some(Ok((request, addr))) = await!(stream.next()) {

        }
    })
}

fn handle_request(registry: &mut FileRegistry, addr: SocketAddr, request: Result<Request, DecodeError>) -> Result<(), Error> {
    let request = request?;
    info!("{:?}", request);
    let connection = Connection::new(addr);

    match request.r#type() {
        AccessType::Read => {
            handle_read(connection, registry.read_file(request.filename())?);
        },
        AccessType::Write => {
            handle_write(connection, registry.write_file(request.filename())?);
        },
    }
    Ok(())
}

fn main() {
    tokio::run_async(async {
        let addr: SocketAddr = "0.0.0.0:69".parse().unwrap();
        let listener = UdpSocket::bind(&addr).unwrap();
        let mut stream = UdpFramed::new(listener, TftpServer::new());
        let mut registry = FileRegistry::new();

        while let Some(Ok((request, addr))) = await!(stream.next()) {
            match handle_request(&mut registry, addr, request) {
                Err(e) => error!("error: {}", e),
                Ok(()) => (),
            }
        }
    });
}
