use std::{
    collections::HashMap,
    error::Error,
    fs::read,
    io::{self, Read},
    net::SocketAddr,
};

use log::{error, trace};
use mio::{
    Events, Interest, Poll, Token,
    net::{TcpListener, TcpStream},
};

struct Connection {
    stream: TcpStream,
    socket: SocketAddr,
    input: [u8; 1024],
    output: [u8; 1024],
}

impl Connection {
    fn new(params: (TcpStream, SocketAddr)) -> Self {
        Self {
            stream: params.0,
            socket: params.1,
            input: [0; 1024],
            output: [0; 1024],
        }
    }
}

fn next(mut token: Token) -> Token {
    let index = token.0;
    token.0 = index + 1;
    Token(index)
}

fn run() -> Result<(), Box<dyn Error>> {
    const SERVER: Token = Token(0);
    let token = next(SERVER);

    let mut poll = Poll::new()?;
    let mut events = Events::with_capacity(64);

    let addr = "127.0.0.1:8080".parse()?;
    let mut server = TcpListener::bind(addr)?;

    poll.registry()
        .register(&mut server, SERVER, Interest::READABLE)?;

    let mut connections = HashMap::<Token, Connection>::new();

    'main: loop {
        poll.poll(&mut events, None)?;
        for event in events.iter() {
            let mut disconnect = false;
            match event.token() {
                SERVER => {
                    let mut connection = Connection::new(server.accept()?);
                    let session_token = next(token);
                    poll.registry().register(
                        &mut connection.stream,
                        session_token,
                        Interest::READABLE.add(Interest::WRITABLE),
                    )?;
                    trace!("new connection: {} {}", session_token.0, connection.socket);
                    connections.insert(next(token), connection);
                }
                session_token => {
                    let Some(connection) = connections.get_mut(&session_token) else {
                        error!("failed to find connection for token {}", session_token.0);
                        continue;
                    };
                    if event.is_read_closed() {
                        trace!("is_read_closed: {} {}", session_token.0, connection.socket);
                        connections.remove(&session_token);
                        continue;
                    }
                    if event.is_write_closed() {
                        trace!("is_write_closed: {} {}", session_token.0, connection.socket);
                        connections.remove(&session_token);
                        continue;
                    }
                    if event.is_readable() {
                        trace!("is_readable: {} {}", session_token.0, connection.socket);
                        match read_all(&mut connection.stream, &mut connection.input) {
                            Ok(0) => {
                                if let Some(mut connection) = connections.remove(&session_token) {
                                    poll.registry().deregister(&mut connection.stream);
                                }
                            },
                            Ok(read) => {},
                            Err(_) => {},
                        }
                    }
                    if event.is_writable() {
                        trace!("is_writable: {} {}", session_token.0, connection.socket);
                        // write data
                    }
                }
            }

            if disconnect {
                let Some(connection) = connections.get_mut(&session_token) else {
                    error!("failed to find connection for token {}", session_token.0);
                    continue;
                };
                poll.registry().deregister(&mut connection.stream);
            }

        }
    }
}

fn read_all(stream: &mut TcpStream, buf: &mut [u8]) -> io::Result<usize> {
    let mut index = 0;
    loop {
        match stream.read(&mut buf[index..]) {
            Ok(0) => {
                return Ok(0);
            }
            Ok(n) => {
                index += n;
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                return Ok(index);
            }
            Err(e) => {
                return Err(e);
            }
        };
    }
}

fn main() {
    env_logger::init();
    match run() {
        Ok(_) => {}
        Err(e) => trace!("failed {}", e),
    }
}
