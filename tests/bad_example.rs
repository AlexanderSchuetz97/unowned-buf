#![allow(dead_code)]
use std::io::{BufReader, BufWriter, ErrorKind, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::ops::Deref;
use std::sync::{Arc, Mutex};
use std::{io, thread};

struct DuplexBufferedTcpStream {
    stream: ArcTcpStream,
    read_buf: Mutex<BufReader<ArcTcpStream>>,
    write_buf: Mutex<BufWriter<ArcTcpStream>>,
}

impl DuplexBufferedTcpStream {
    fn new(stream: TcpStream) -> Self {
        let inner_arc = ArcTcpStream(Arc::new(stream));
        Self {
            stream: inner_arc.clone(),
            read_buf: Mutex::new(BufReader::new(inner_arc.clone())),
            write_buf: Mutex::new(BufWriter::new(inner_arc.clone())),
        }
    }

    fn read(&self, buf: &mut [u8]) -> io::Result<usize> {
        self.read_buf
            .try_lock()
            .map_err(|_| io::Error::from(ErrorKind::WouldBlock))?
            .read(buf)
    }

    fn write(&self, buf: &[u8]) -> io::Result<usize> {
        self.write_buf
            .try_lock()
            .map_err(|_| io::Error::from(ErrorKind::WouldBlock))?
            .write(buf)
    }

    fn flush(&self) -> io::Result<()> {
        self.write_buf
            .try_lock()
            .map_err(|_| io::Error::from(ErrorKind::WouldBlock))?
            .flush()
    }
}

#[derive(Debug, Clone)]
struct ArcTcpStream(Arc<TcpStream>);

impl Read for ArcTcpStream {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.0.deref().read(buf)
    }
}

impl Write for ArcTcpStream {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.deref().write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.0.deref().flush()
    }
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
