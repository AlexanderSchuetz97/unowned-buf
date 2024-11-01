#![allow(dead_code)]
use std::io::{ErrorKind};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::{io, thread};
use unowned_buf::{UnownedReadBuffer, UnownedWriteBuffer};

#[derive(Debug)]
struct DuplexBufferedTcpStream {
    stream: TcpStream,
    read_buf: Mutex<UnownedReadBuffer<0x4000>>,
    write_buf: Mutex<UnownedWriteBuffer<0x4000>>,
}

impl DuplexBufferedTcpStream {
    fn new(stream: TcpStream) -> DuplexBufferedTcpStream {
        Self {
            stream,
            read_buf: Mutex::new(UnownedReadBuffer::new()),
            write_buf: Mutex::new(UnownedWriteBuffer::new()),
        }
    }

    fn read(&self, buf: &mut [u8]) -> io::Result<usize> {
        self.read_buf
            .try_lock()
            .map_err(|_| io::Error::from(ErrorKind::WouldBlock))?
            .read(&mut &self.stream, buf)
    }

    fn write(&self, buf: &[u8]) -> io::Result<usize> {
        self.write_buf
            .try_lock()
            .map_err(|_| io::Error::from(ErrorKind::WouldBlock))?
            .write(&mut &self.stream, buf)
    }

    fn flush(&self) -> io::Result<()> {
        self.write_buf
            .try_lock()
            .map_err(|_| io::Error::from(ErrorKind::WouldBlock))?
            .flush(&mut &self.stream)
    }

    //Add other fn delegates from BufRead, Read or Write as needed or implement the traits for these directly.
}

/// This serves as an example that reads from one thread and
pub fn main() {
    let listen = TcpListener::bind("127.0.0.1:0").unwrap();
    let stream = listen.accept().unwrap().0;

    let duplex = Arc::new(DuplexBufferedTcpStream::new(stream));

    {
        let duplex = duplex.clone();
        thread::spawn(move || {
            let mut buf = vec![0u8; 512];
            duplex.read(buf.as_mut_slice()).expect("failed to read");
        });
    }

    let buf = vec![0u8; 512];
    duplex.write(buf.as_slice()).expect("failed to write");
    duplex.flush().expect("failed to flush");
}

#[test]
pub fn dummy() {}
