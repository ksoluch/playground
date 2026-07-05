use std::{backtrace::Backtrace, io::{Read, Write}, net::TcpStream};

enum ClientError {
    IO(std::io::Error, std::backtrace::Backtrace),
}

impl std::fmt::Display for ClientError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", match self {
            ClientError::IO(error, backtrace) => {
                let mut desc = error.to_string();
                let callstack = backtrace.to_string();
                desc += "\n";
                desc += callstack.as_str();
                desc
            },
        })
    }
}

impl From<std::io::Error> for ClientError {
    fn from(value: std::io::Error) -> Self {
        ClientError::IO(value, Backtrace::capture())
    }
}

fn run_client() -> Result<(), ClientError> {
    let mut connection = TcpStream::connect("127.0.0.1:8080")?;
    let message = String::from("Hello");
    connection.write(message.as_bytes())?;
    println!("Sent {}", message);
    let mut response: [u8;512] = [0;512];
    connection.read(&mut response[..])?;
    println!("Sent {}", String::from_utf8_lossy(&response));
    Ok(())
}

fn main() {
    if let Err(error) = run_client()  {
        eprintln!("{}", error);
    }
}
