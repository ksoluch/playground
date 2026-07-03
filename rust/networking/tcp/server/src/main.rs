use std::{
    collections::HashMap, io::{self, Read, Write}, net::{AddrParseError, SocketAddr}
};

#[derive(thiserror::Error, Debug)]
enum ServerError {
    #[error("IO error: {source}")]
    IO { #[from] source: std::io::Error, },
    #[error("IP address parsing error: {source}")]
    IpAddress { #[from] source: AddrParseError, },
}

use log::{error, trace};
use mio::{
    Events, Interest, Poll, Token,
    net::{TcpListener, TcpStream},
};

struct Connection {
    stream: TcpStream,
    socket: SocketAddr,
    input: [u8; 1024],
    read: usize,
}

impl Connection {
    fn new(params: (TcpStream, SocketAddr)) -> Self {
        Self {
            stream: params.0,
            socket: params.1,
            input: [0; 1024],
            read: usize::default(),
        }
    }
}

fn next(token: Token) -> Token {
    let index = token.0 + 1;
    Token(index)
}

fn run() -> Result<(), ServerError> {
    const SERVER: Token = Token(0);
    let mut token = next(SERVER);

    let mut poll = Poll::new()?;
    let mut events = Events::with_capacity(64);

    let addr = "127.0.0.1:8080a".parse()?;
    let mut server = TcpListener::bind(addr)?;

    poll.registry()
        .register(&mut server, SERVER, Interest::READABLE)?;

    let mut connections = HashMap::<Token, Connection>::new();

    loop {
        poll.poll(&mut events, None)?;
        for event in events.iter() {
            let mut disconnect = false;
            match event.token() {
                SERVER => {
                    let mut connection = Connection::new(server.accept()?);
                    poll.registry().register(
                        &mut connection.stream,
                        token,
                        Interest::READABLE.add(Interest::WRITABLE),
                    )?;
                    trace!("new connection: {} {}", token.0, connection.socket);
                    connections.insert(token, connection);
                    token = next(token);
                }
                session_token => {
                    if event.is_read_closed() {
                        let Some(connection) = connections.get_mut(&session_token) else {
                            error!("failed to find connection for token {}", session_token.0);
                            continue;
                        };
                        trace!("is_read_closed: {} {}", session_token.0, connection.socket);
                        connections.remove(&session_token);
                        continue;
                    }
                    if event.is_write_closed() {
                        let Some(connection) = connections.get_mut(&session_token) else {
                            error!("failed to find connection for token {}", session_token.0);
                            continue;
                        };
                        trace!("is_write_closed: {} {}", session_token.0, connection.socket);
                        connections.remove(&session_token);
                        continue;
                    }
                    if event.is_readable() {
                        let Some(connection) = connections.get_mut(&session_token) else {
                            error!("failed to find connection for token {}", session_token.0);
                            continue;
                        };
                        trace!("is_readable: {} {}", session_token.0, connection.socket);
                        match read_all(&mut connection.stream, &mut connection.input) {
                            Ok(n) => {
                                trace!("read {} bytes", n);
                            }
                            Err(_) => {
                                disconnect = true;
                            }
                        }
                    }
                    if event.is_writable() {
                        let Some(connection) = connections.get_mut(&session_token) else {
                            error!("failed to find connection for token {}", session_token.0);
                            continue;
                        };
                        trace!(
                            "is_writable: {} {} {}",
                            session_token.0, connection.socket, connection.read
                        );
                        if connection.read > 0 {
                            let message =
                                String::from_utf8_lossy(&connection.input[..connection.read])
                                    .to_string()
                                    .to_uppercase();
                            connection.read = 0;
                            let _ = (&connection.stream).write_all(message.as_bytes());
                        }
                    }
                }
            }

            if disconnect {
                let Some(connection) = connections.get_mut(&event.token()) else {
                    error!("failed to find connection for token {}", event.token().0);
                    continue;
                };
                let _ = poll.registry().deregister(&mut connection.stream);
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
    if let Err(error) = run() {
        // println!("{}", error);
        match error {
            ServerError::IO { source } => println!("{}", source),
            ServerError::IpAddress { source } => println!("{}", source),
        }
    }
}
