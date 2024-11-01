# Unowned Buf
Buffered Read+BufRead and Write for Rust that does not own the underlying Read/Write

# Purpose and Example
My motivation for making this crate was to reduce the number of Arc's needed when writing 
Buffered Stateful Duplex Connection structs where the underlying connection T is Read for &T and Write for &T.

In the rust standard library this is the case for:
* UnixStream
* TcpStream

## Example
I would like to mention that this example is simplified and uses TcpStream which has a try_clone()
which can be used to bypass the need for multiple layers Arc's.
Unfortunately the underlying connection I am actually working with does not have this.
In addition, the number of open file descriptors are a concern for me,
so I would prefer only having 1 fd per TcpStream.

```rust
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
        self.read_buf.try_lock()
            .map_err(|_| io::Error::from(ErrorKind::WouldBlock))?
            .read(&mut &self.stream, buf)
    }

    fn write(&self, buf: &[u8]) -> io::Result<usize> {
        self.write_buf.try_lock()
            .map_err(|_| io::Error::from(ErrorKind::WouldBlock))?
            .write(&mut &self.stream, buf)
    }

    fn flush(&self) -> io::Result<()> {
        self.write_buf.try_lock()
            .map_err(|_| io::Error::from(ErrorKind::WouldBlock))?
            .flush(&mut &self.stream)
    }

    //Add other fn delegates from BufRead, Read or Write as needed or implement the traits for these directly.
    //Or add set/get timeout fns that delete to the TcpStream.
}

/// This serves as an example that reads from one thread and 
pub fn main() {
    let listen = TcpListener::bind("127.0.0.1:0").unwrap();
    let stream = listen.accept().unwrap().0;

    let duplex = Arc::new(DuplexBufferedTcpStream::new(stream));

    {
        let duplex = duplex.clone();
        thread::spawn(|| {
            let mut buf = vec![0u8; 512];
            duplex.read(buf.as_mut_slice()).expect("failed to read");
        });
    }

    let buf = vec![0u8; 512];
    duplex.write(buf.as_slice()).expect("failed to write");
    duplex.flush().expect("failed to flush");
}
```

### Original Code I wanted to improve
Again this example is simplified.

As you will be able to observe to effectively use this type of connection struct you need 2 layers of Arc's.

It also needs a helper struct that wraps the Arc to satisfy Read/Write requirement of the standard libraries BufReader/BufWriter's
owned type requirement.

```rust
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
        self.read_buf.try_lock()
            .map_err(|_| io::Error::from(ErrorKind::WouldBlock))?
            .read(buf)
    }

    fn write(&self, buf: &[u8]) -> io::Result<usize> {
        self.write_buf.try_lock()
            .map_err(|_| io::Error::from(ErrorKind::WouldBlock))?
            .write(buf)
    }

    fn flush(&self) -> io::Result<()> {
        self.write_buf.try_lock()
            .map_err(|_| io::Error::from(ErrorKind::WouldBlock))?
            .flush()
    }
}

#[derive(Debug, Clone)]
struct ArcTcpStream(Arc<TcpStream>);

impl Read for ArcTcpStream {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.deref().read(buf)
    }
}

impl Write for ArcTcpStream {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.deref().write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.deref().flush()
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
```

# Caveats
Using BufReader/BufWriter + Arc is probably still faster than my implementation of BufRead/Read/Write.
I simply did not have time yet to properly optimize it.

Some tests exist that test the functions to ensure they work as advertised.
Use at your own discretion.