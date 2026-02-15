//! Buffered Read+BufRead and Write for Rust that does not own the underlying Read/Write
//!
//! # Example usage
//! ```rust
//! use std::io;
//! use std::io::ErrorKind;
//! use std::net::TcpStream;
//! use std::sync::Mutex;
//! use unowned_buf::{UnownedReadBuffer, UnownedWriteBuffer};
//!
//! #[derive(Debug)]
//! struct DuplexBufferedTcpStream {
//!     stream: TcpStream,
//!     read_buf: Mutex<UnownedReadBuffer<0x4000>>,
//!     write_buf: Mutex<UnownedWriteBuffer<0x4000>>,
//! }
//!
//! impl DuplexBufferedTcpStream {
//!     fn new(stream: TcpStream) -> DuplexBufferedTcpStream {
//!         Self {
//!             stream,
//!             read_buf: Mutex::new(UnownedReadBuffer::new()),
//!             write_buf: Mutex::new(UnownedWriteBuffer::new()),
//!         }
//!     }
//!
//!     fn read(&self, buf: &mut [u8]) -> io::Result<usize> {
//!         self.read_buf.try_lock()
//!             .map_err(|_| io::Error::from(ErrorKind::WouldBlock))?
//!             .read(&mut &self.stream, buf)
//!     }
//!
//!     fn write(&self, buf: &[u8]) -> io::Result<usize> {
//!         self.write_buf.try_lock()
//!             .map_err(|_| io::Error::from(ErrorKind::WouldBlock))?
//!             .write(&mut &self.stream, buf)
//!     }
//!
//!     fn flush(&self) -> io::Result<()> {
//!         self.write_buf.try_lock()
//!             .map_err(|_| io::Error::from(ErrorKind::WouldBlock))?
//!             .flush(&mut &self.stream)
//!     }
//!
//!     //Add other fn delegates from BufRead, Read or Write as needed or implement the traits for these directly.
//!     //Or add set/get timeout fns that delete to the TcpStream.
//! }
//! ```

#![deny(clippy::correctness, unsafe_code)]
#![warn(
    clippy::perf,
    clippy::complexity,
    clippy::style,
    clippy::nursery,
    clippy::pedantic,
    clippy::clone_on_ref_ptr,
    clippy::decimal_literal_representation,
    clippy::float_cmp_const,
    clippy::missing_docs_in_private_items,
    clippy::multiple_inherent_impl,
    clippy::unwrap_used,
    clippy::cargo_common_metadata,
    clippy::used_underscore_binding
)]

use std::fmt::{Debug, Formatter};
use std::io;
use std::io::{BufRead, ErrorKind, Read, Write};

///
/// Unowned Write buffer.
///
/// # S Generic: Size of the buffer.
/// beware that if this size is too large, and you stack allocate this struct
/// then you will hit the guard page and your program will crash.
/// If you must use very large buffers then Box this struct.
///
///
#[derive(Debug)]
pub struct UnownedWriteBuffer<const S: usize> {
    /// How many bytes in the buffer have we filled and must still be sent to a `Write` impl?
    fill_count: usize,
    /// The buffer
    buffer: [u8; S],
}

impl<const S: usize> UnownedWriteBuffer<S> {
    /// Construct a new Buffer
    /// # Panics
    /// if S is smaller than 16.
    #[must_use]
    pub const fn new() -> Self {
        let buf = Self {
            fill_count: 0,
            buffer: [0; S],
        };

        assert!(buf.buffer.len() >= 16, "UnownedWriteBuffer is too small");

        buf
    }
}

impl Default for UnownedWriteBuffer<0x4000> {
    fn default() -> Self {
        Self {
            fill_count: 0,
            buffer: [0; 0x4000],
        }
    }
}

impl<const S: usize> UnownedWriteBuffer<S> {
    /// Returns the amount of bytes that can still be written into the internal buffer.
    #[must_use]
    pub const fn available(&self) -> usize {
        self.buffer.len() - self.fill_count
    }

    /// Returns the amount of bytes that can be flushed from the internal buffer to a underlying Write.
    #[must_use]
    pub const fn flushable(&self) -> usize {
        self.fill_count
    }

    /// Returns the bytes currently stored in the internal buffer that are not yet flushed.
    #[must_use]
    pub fn internal_buffer(&self) -> &[u8] {
        &self.buffer[..self.fill_count]
    }

    /// Returns the bytes currently stored in the internal buffer that are not yet flushed.
    ///
    /// Modifying the returned data will directly modify the bytes in the internal buffer.
    #[must_use]
    pub fn internal_buffer_mut(&mut self) -> &mut [u8] {
        &mut self.buffer[..self.fill_count]
    }

    #[must_use]
    pub const fn size(&self) -> usize {
        S
    }

    /// Push some bytes to the Write impl.
    fn push<T: Write>(&mut self, write: &mut T) -> io::Result<()> {
        if self.fill_count == 0 {
            return Ok(());
        }

        let mut count = 0usize;
        while count < self.fill_count {
            match write.write(&self.buffer[count..self.fill_count]) {
                Ok(cnt) => {
                    count += cnt;
                }
                Err(e) => {
                    if count == 0 {
                        return Err(e);
                    }
                    self.buffer.copy_within(count..self.fill_count, 0);
                    self.fill_count -= count;
                    return Err(e);
                }
            }
        }

        self.fill_count = 0;
        Ok(())
    }

    /// Flush all bytes to the underlying Write impl. This call also calls `Write::flush` afterward.
    /// # Errors
    /// Propagated from `Write` impl
    pub fn flush<T: Write>(&mut self, write: &mut T) -> io::Result<()> {
        self.push(write)?;
        write.flush()
    }

    /// Write as many bytes as can still fit to the internal buffer.
    /// This function returns 0 if the internal buffer is full.
    /// If the supplied buffer is only partially written then this fn guarantees that
    /// the entire internal buffer has been filled and subsequent calls to `try_write` are pointless
    /// unless flush or `write`/`write_all` are first called.
    pub fn try_write(&mut self, buffer: &[u8]) -> usize {
        if buffer.is_empty() {
            return 0;
        }
        let available = self.available();
        if available == 0 {
            return 0;
        }

        if available < buffer.len() {
            //PARTIAL WRITE
            self.buffer[self.fill_count..].copy_from_slice(&buffer[..available]);
            self.fill_count += available;
            return available;
        }

        //FULL WRITE
        self.buffer[self.fill_count..self.fill_count + buffer.len()].copy_from_slice(buffer);
        self.fill_count += buffer.len();
        buffer.len()
    }

    /// Write as many bytes as can still fit to the internal buffer.
    /// This call will not push the internal buffer to the Write impl if the internal buffer
    /// still had room for at least one byte. It is only guaranteed to at least "write" 1 byte.
    /// This fn might call the underlying write impl several times.
    ///
    /// # Errors
    /// Propagated from `Write` impl
    ///
    pub fn write<T: Write>(&mut self, write: &mut T, buffer: &[u8]) -> io::Result<usize> {
        if buffer.is_empty() {
            return Ok(0);
        }
        let mut available = self.available();
        if available == 0 {
            self.push(write)?;
            available = self.buffer.len();
        }

        if available < buffer.len() {
            //PARTIAL WRITE
            self.buffer[self.fill_count..].copy_from_slice(&buffer[..available]);
            self.fill_count += available;
            return Ok(available);
        }

        //FULL WRITE
        self.buffer[self.fill_count..self.fill_count + buffer.len()].copy_from_slice(buffer);
        self.fill_count += buffer.len();
        Ok(buffer.len())
    }

    /// Writes all bytes to the internal buffer if they fit,
    /// otherwise all excess bytes are flushed to the underlying Write impl.
    ///
    /// This fn only returns `Ok()` if all bytes are either in the internal buffer or already
    /// written to the underlying Write impl.
    ///
    /// # Errors
    /// Propagated from `Write` impl
    ///
    pub fn write_all<T: Write>(&mut self, write: &mut T, buffer: &[u8]) -> io::Result<()> {
        if buffer.is_empty() {
            return Ok(());
        }

        let mut count = 0usize;
        loop {
            let rem = buffer.len() - count;
            let mut available = self.available();

            if available == 0 {
                self.push(write)?;
                available = self.buffer.len();
            }

            if available < rem {
                //PARTIAL WRITE
                self.buffer[self.fill_count..].copy_from_slice(&buffer[count..count + available]);
                self.fill_count += available;
                count += available;
                if count >= buffer.len() {
                    return Ok(());
                }
                continue;
            }

            //FULL WRITE
            self.buffer[self.fill_count..self.fill_count + rem].copy_from_slice(&buffer[count..]);
            self.fill_count += rem;
            return Ok(());
        }
    }

    /// This fn "borrows"/associates this buffer with a Write impl. The returned `BorrowedWriteBuffer`
    /// has the same lifetime as the Write impl and &mut self combined and can be used as a dyn Write.
    /// This might be required to call some library functions which demand a dyn Write as parameter.
    pub fn borrow<'a, T: Write>(&'a mut self, write: &'a mut T) -> BorrowedWriteBuffer<'a, T, S> {
        BorrowedWriteBuffer {
            buffer: self,
            write,
        }
    }
}

/// Borrowed dyn Write of a `UnownedWriteBuffer`.
/// This borrowed version is directly associated with a Write impl, but is subject to lifetimes.
pub struct BorrowedWriteBuffer<'a, T: Write, const S: usize> {
    /// Read ref
    buffer: &'a mut UnownedWriteBuffer<S>,
    /// Write ref
    write: &'a mut T,
}

impl<T: Write, const S: usize> Debug for BorrowedWriteBuffer<'_, T, S> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&self.buffer, f)
    }
}

impl<T: Write, const S: usize> Write for BorrowedWriteBuffer<'_, T, S> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.buffer.write(self.write, buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.buffer.flush(self.write)
    }

    fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
        self.buffer.write_all(self.write, buf)
    }
}

///
/// Unowned Read buffer.
///
/// # S Generic: Size of the buffer.
/// beware that if this size is too large, and you stack allocate this struct
/// then you will hit the guard page and your program will crash.
///
///
#[derive(Debug)]
pub struct UnownedReadBuffer<const S: usize> {
    /// How much have we read?
    read_count: usize,
    /// How much can we read?
    fill_count: usize,
    /// The buffer
    buffer: [u8; S],
}

impl<const S: usize> UnownedReadBuffer<S> {
    /// Construct a new Buffer
    ///
    /// # Panics
    /// if S is smaller than 16
    #[must_use]
    pub const fn new() -> Self {
        let buf = Self {
            read_count: 0,
            fill_count: 0,
            buffer: [0; S],
        };

        assert!(buf.buffer.len() >= 16, "UnownedReadBuffer is too small");

        buf
    }
}

impl Default for UnownedReadBuffer<0x4000> {
    fn default() -> Self {
        Self {
            read_count: 0,
            fill_count: 0,
            buffer: [0; 0x4000],
        }
    }
}

impl<const S: usize> UnownedReadBuffer<S> {
    /// reads some bytes from the read impl.
    fn feed<T: Read>(&mut self, read: &mut T) -> io::Result<bool> {
        self.compact();

        let count = read.read(&mut self.buffer.as_mut_slice()[self.fill_count..])?;
        if count == 0 {
            return Ok(false);
        }

        self.fill_count += count;
        Ok(true)
    }

    /// returns the amount of bytes that can still be read from the internal buffer.
    #[must_use]
    pub const fn available(&self) -> usize {
        self.fill_count - self.read_count
    }

    /// returns the size of the internal buffer.
    #[must_use]
    pub const fn size(&self) -> usize {
        S
    }

    /// Returns the number of bytes in the buffer already read.
    /// This is essentially the read cursor position in the internal buffer.
    /// calls to compact will reset this to 0.
    #[must_use]
    pub const fn read_count(&self) -> usize {
        self.read_count
    }

    /// Returns the writing cursor position of the internal buffer.
    /// I.e. how many bytes in the internal buffer are or were at some point filled with data.
    #[must_use]
    pub const fn fill_count(&self) -> usize {
        self.fill_count
    }

    /// Returns the number of bytes at the end of the buffer that can still be filled with data.
    /// This function does not take the number of bytes that were
    /// already read from the start of the buffer into account.
    #[must_use]
    pub const fn available_space(&self) -> usize {
        S - self.fill_count
    }

    /// This function compacts the internal buffer, discarding already read bytes
    /// and moving all remaining bytes to the start of the internal buffer.
    ///
    /// It is guaranteed that the `read_count` is 0 after this function returns.
    /// This function has no effect if the read count is already 0 prior to calling this function.
    pub fn compact(&mut self) {
        if self.read_count > 0 {
            if self.read_count < self.fill_count {
                self.buffer.copy_within(self.read_count..self.fill_count, 0);
            }
            self.fill_count -= self.read_count;
            self.read_count = 0;
        }
    }

    /// This function will perform a single read and append the bytes read at the end of the internal buffer if there is still space.
    ///
    /// This function will not compact the buffer and only append data to the end of the buffer.
    /// If buffer compaction is needed, call `compact` before calling this function.
    ///
    /// # Panics
    /// This function will panic if `available_space` returns 0 because in this case
    /// no further bytes can be appended to the end of the buffer.
    ///
    /// # Errors
    /// All other errors are propagated from the Read.
    ///
    /// # Returns
    /// Number of bytes read, 0 indicating EOF on the Read impl.
    pub fn read_into_internal_buffer<T: Read>(&mut self, read: &mut T) -> io::Result<usize> {
        debug_assert!(self.fill_count <= S);

        assert_ne!(self.available_space(), 0, "internal buffer is full.");

        let count = read.read(&mut self.buffer.as_mut_slice()[self.fill_count..])?;
        self.fill_count += count;
        Ok(count)
    }

    /// This function will copy/append the given bytes to the end of the internal buffer
    /// so that they are picked up by a later read.
    ///
    /// This function does not compact the buffer.
    /// If buffer compaction is needed, call `compact` before calling this function.
    ///
    /// # Panics
    /// if the internal buffer does not have enough space to hold the given number of bytes.
    pub fn copy_into_internal_buffer(&mut self, data: &[u8]) {
        let space = self.available_space();
        let needed = data.len();
        assert!(space >= needed, "internal buffer is too small. The internal buffer can currently only hold {space} more bytes. This can't fit {needed} more bytes.");

        self.buffer[self.fill_count..self.fill_count + needed].copy_from_slice(data);
        self.fill_count += needed;
    }

    /// This function returns the portion of the internal buffer that is filled with data for inspection.
    /// The next call to read would read the first byte of the returned slice.
    #[must_use]
    pub fn internal_buffer(&self) -> &[u8] {
        &self.buffer[self.read_count..self.fill_count]
    }

    /// This function returns the portion of the internal buffer that is filled with data for inspection and modification.
    /// The next call to read would read the first byte of the returned slice.
    /// Modification of the returned slice directly modifies the data stored in the internal buffer.
    #[must_use]
    pub fn internal_buffer_mut(&mut self) -> &mut [u8] {
        &mut self.buffer[self.read_count..self.fill_count]
    }

    /// Skips/Discards number bytes of the internal buffer.
    ///
    /// # Panics
    /// If the amount to skip is greater than the number of bytes in the internal buffer.
    /// Call `available` to check the maximum number of bytes that can be skipped.
    pub fn skip(&mut self, amount: usize) {
        let available = self.available();
        assert!(
            available >= amount,
            "attempted to skip {amount} bytes, but only {available} bytes are available"
        );

        if available == amount {
            // The buffer is empty now.
            self.read_count = 0;
            self.fill_count = 0;
            return;
        }

        self.read_count += amount;
    }

    /// This fn will return true if at least one byte can be read.
    /// If the internal buffer is not empty, then this fn immediately returns true.
    /// If the internal buffer is empty, then it will call `read()` once and return true if the read did not return Ok(0).
    /// The data that was read is stored in the internal buffer.
    ///
    /// # Errors
    /// propagated from Read, including `TimedOut` and `WouldBlock`
    pub fn ensure_readable<T: Read>(&mut self, read: &mut T) -> io::Result<bool> {
        if self.available() > 0 {
            return Ok(true);
        }

        self.feed(read)
    }

    /// This fn reads as many bytes as possible from the internal buffer.
    /// it returns 0 if the internal buffer is empty.
    ///
    pub fn try_read(&mut self, buffer: &mut [u8]) -> usize {
        if buffer.is_empty() {
            return 0;
        }

        let available = self.available();
        if available == 0 {
            return 0;
        }

        if available >= buffer.len() {
            //FULL READ
            buffer.copy_from_slice(
                &self.buffer.as_slice()[self.read_count..self.read_count + buffer.len()],
            );
            self.read_count += buffer.len();
            return buffer.len();
        }

        //PARTIAL READ
        buffer[..available]
            .copy_from_slice(&self.buffer.as_slice()[self.read_count..self.fill_count]);
        //The buffer is empty now.
        self.read_count = 0;
        self.fill_count = 0;
        available
    }

    /// This fn will read as many bytes as possible from the internal buffer.
    /// If the internal buffer is empty when this fn is called then 1 call to the `Read` impl is made to fill the buffer.
    /// This fn only returns Ok(0) if the 1 call to the underlying read impl returned 0.
    /// This fn does not call the read impl if `available()` != 0.
    /// # Errors
    /// Propagated from the `Read` impl
    ///
    pub fn read<T: Read>(&mut self, read: &mut T, buffer: &mut [u8]) -> io::Result<usize> {
        if buffer.is_empty() {
            return Ok(0);
        }

        let mut available = self.available();
        if available == 0 {
            if !self.feed(read)? {
                return Ok(0);
            }

            available = self.available();
        }

        if available >= buffer.len() {
            //FULL READ
            buffer.copy_from_slice(
                &self.buffer.as_slice()[self.read_count..self.read_count + buffer.len()],
            );
            self.read_count += buffer.len();
            return Ok(buffer.len());
        }

        //PARTIAL READ
        buffer[..available]
            .copy_from_slice(&self.buffer.as_slice()[self.read_count..self.fill_count]);
        //The buffer is empty now.
        self.read_count = 0;
        self.fill_count = 0;
        Ok(available)
    }

    /// This fn will read the entire buffer from either the internal buffer or the
    /// `Read` impl. Multiple calls to the read impl may be made if necessary to fill the buffer.
    ///
    /// # Errors
    /// Propagated from the `Read` impl
    /// `ErrorKind::UnexpectedEof` if the `Read` impl returns Ok(0) before the buffer was filled.
    ///
    pub fn read_exact<T: Read>(&mut self, read: &mut T, buffer: &mut [u8]) -> io::Result<()> {
        if buffer.is_empty() {
            return Ok(());
        }

        let mut buffer = buffer;

        if self.available() == 0 && !self.feed(read)? {
            return Err(io::Error::from(ErrorKind::UnexpectedEof));
        }

        loop {
            let available = self.available();
            if available >= buffer.len() {
                //FULL read
                buffer.copy_from_slice(
                    &self.buffer.as_slice()[self.read_count..self.read_count + buffer.len()],
                );
                self.read_count += buffer.len();
                return Ok(());
            }

            //PARTIAL READ
            buffer[..available].copy_from_slice(
                &self.buffer.as_slice()[self.read_count..self.read_count + available],
            );
            //The buffer is empty now.
            self.read_count = 0;
            self.fill_count = 0;
            if !self.feed(read)? {
                return Err(io::Error::from(io::ErrorKind::UnexpectedEof));
            }
            buffer = &mut buffer[available..];
        }
    }

    /// Reads until either EOF happens or the desired byte is found.
    /// This fn may call the underlying `Read` impl multiple times until the buffer is filled.
    ///
    /// # Errors
    /// Propagated from the `Read` impl
    ///
    pub fn read_until<T: Read>(
        &mut self,
        read: &mut T,
        byte: u8,
        buf: &mut Vec<u8>,
    ) -> io::Result<usize> {
        let mut count: usize = 0;

        if self.available() == 0 && !self.feed(read)? {
            return Ok(0);
        }

        loop {
            for idx in self.read_count..self.fill_count {
                if self.buffer[idx] == byte {
                    let to_push = &self.buffer[self.read_count..=idx];
                    buf.extend_from_slice(to_push);
                    self.read_count += to_push.len();
                    return Ok(count + to_push.len());
                }
            }

            let to_push = &self.buffer[self.read_count..self.fill_count];
            buf.extend_from_slice(to_push);
            count += to_push.len();
            self.read_count = 0;
            self.fill_count = 0;
            if !self.feed(read)? {
                return Ok(count);
            }
        }
    }

    /// Reads until either EOF happens or the desired byte is found or limit bytes have been appended to buf.
    /// The actual read impl may supply more bytes than limit, the excess is stored in the internal buffer in this case.
    /// Returns the amount of bytes appended to the buf vec.
    ///
    /// # Errors
    /// Propagated from the `Read` impl
    ///
    pub fn read_until_limit<T: Read>(
        &mut self,
        read: &mut T,
        byte: u8,
        limit: usize,
        buf: &mut Vec<u8>,
    ) -> io::Result<usize> {
        let mut count: usize = 0;

        if limit == 0 {
            return Ok(0);
        }

        if self.available() == 0 && !self.feed(read)? {
            return Ok(0);
        }

        loop {
            let mut to_push = &self.buffer[self.read_count..self.fill_count];
            if count + to_push.len() > limit {
                to_push = &to_push[..limit - count];
            }

            debug_assert!(count + to_push.len() <= limit);

            for idx in 0..to_push.len() {
                if to_push[idx] == byte {
                    to_push = &to_push[..=idx];
                    buf.extend_from_slice(to_push);
                    self.read_count += to_push.len();
                    return Ok(count + to_push.len());
                }
            }

            buf.extend_from_slice(to_push);
            count += to_push.len();
            self.read_count += to_push.len();
            if count >= limit {
                return Ok(count);
            }

            if !self.feed(read)? {
                return Ok(count);
            }
        }
    }

    /// Reads all remaining bytes into the buffer.
    /// Those bytes may be from the internal buffer and then from the underlying `Read` impl.
    /// # Errors
    /// Propagated from the `Read` impl
    ///
    pub fn read_to_end<T: Read>(&mut self, read: &mut T, buf: &mut Vec<u8>) -> io::Result<usize> {
        if self.available() == 0 && !self.feed(read)? {
            return Ok(0);
        }

        let mut count = 0usize;

        loop {
            let push = &self.buffer.as_slice()[self.read_count..self.fill_count];
            buf.extend_from_slice(push);
            count += push.len();
            self.fill_count = 0;
            self.read_count = 0;
            if !self.feed(read)? {
                return Ok(count);
            }
        }
    }

    /// Reads all remaining bytes into the String.
    /// Those bytes may be from the internal buffer and then from the underlying `Read` impl.
    /// If the `Read` or buffer contained non-valid utf-8 sequences then this fn returns an `io::Error` with Kind `InvalidData`.
    /// No data is lost in this case as bytes read and already placed in the buf are, in the buf and all remaining bytes
    /// starting with those that were not valid utf-8 are in the internal buffer and can be fetched with a call to `try_read()`.
    ///
    /// # Errors
    /// Propagated from the `Read` impl
    /// `ErrorKind::InvalidData` if invalid utf-8 is found.
    ///
    pub fn read_to_string<T: Read>(&mut self, read: &mut T, buf: &mut String) -> io::Result<usize> {
        let mut count = 0usize;
        if self.available() == 0 && !self.feed(read)? {
            return Ok(0);
        }

        loop {
            let to_push = &self.buffer[self.read_count..self.fill_count];
            let mut utf_index = 0;
            //We leave up to 4 bytes in the buffer for the next cycle because those may be part of an incomplete multibyte sequence.
            while utf_index + 4 < to_push.len() {
                utf_index += next_utf8(to_push, utf_index)?;
            }

            if utf_index > 0 {
                buf.push_str(read_utf8(&to_push[..utf_index])?);
                count += utf_index;
                self.read_count += utf_index; //feed will compact the buffer.
            }

            if self.feed(read)? {
                continue;
            }

            //EOF, the rest we must check multibyte for bounds. We must carefully analyze!
            let to_push = &self.buffer[self.read_count..self.fill_count];

            //Bounds: we have at least 1 byte remaining at this point.
            debug_assert!(!to_push.is_empty() && to_push.len() <= 4);

            let mut utf_index = 0;
            loop {
                if utf_index >= to_push.len() {
                    break;
                }
                let len = utf8_len(to_push[utf_index]);
                if len > to_push.len() - utf_index {
                    return Err(io::Error::new(
                        ErrorKind::InvalidData,
                        "stream did not contain valid utf-8",
                    ));
                }
                match len {
                    1 => (),
                    2 => utf8_cont_assert(to_push[utf_index + 1])?,
                    3 => {
                        utf8_cont_assert(to_push[utf_index + 1])?;
                        utf8_cont_assert(to_push[utf_index + 2])?;
                    }
                    4 => {
                        utf8_cont_assert(to_push[utf_index + 1])?;
                        utf8_cont_assert(to_push[utf_index + 2])?;
                        utf8_cont_assert(to_push[utf_index + 3])?;
                    }
                    _ => unreachable!(),
                }

                utf_index += len;
            }

            buf.push_str(read_utf8(to_push)?);
            return Ok(count + to_push.len());
        }
    }

    ///
    /// Reads all bytes into the string until \n is found, or EOF occurred.
    /// Data is first taken from the internal buffer and then taken from the `Read` impl.
    ///
    /// # Major difference from `BufRead`'s `read_line` fn:
    /// this fn guarantees that no data is discarded when invalid utf-8 is encountered.
    /// buf may contain some valid bytes in this case but all invalid bytes are retained in the internal buffer
    /// so that the next call to `read()` can pick them up. call `available()` so you know how many bytes to read!
    /// `BufRead`'s `read_line` fn just discards the last chunk of invalid data.
    ///
    /// # Errors
    /// Propagated from the `Read` impl
    /// `ErrorKind::InvalidData` if invalid utf-8 is found.
    ///
    pub fn read_line<T: Read>(&mut self, read: &mut T, buf: &mut String) -> io::Result<usize> {
        let mut count = 0usize;
        if self.available() == 0 && !self.feed(read)? {
            return Ok(0);
        }

        loop {
            for idx in self.read_count..self.fill_count {
                if self.buffer[idx] == b'\n' {
                    //We found it!
                    let to_push = &self.buffer[self.read_count..=idx];

                    let mut utf_index = 0usize;
                    while utf_index < to_push.len() {
                        //Panic safety, we do not need to check for bounds here,
                        //The last byte in the buffer is known to be \n where utf8_len does return 1!
                        //\n is not a valid continuation so a call to utf8_cont_assert(\n) will always fail.
                        utf_index += next_utf8(to_push, utf_index)?;
                    }
                    buf.push_str(read_utf8(to_push)?);
                    self.read_count += to_push.len();
                    return Ok(count + to_push.len());
                }
            }

            let to_push = &self.buffer[self.read_count..self.fill_count];
            let mut utf_index = 0;
            //We leave up to 4 bytes in the buffer for the next cycle because those may be part of an incomplete multibyte sequence.
            while utf_index + 4 < to_push.len() {
                utf_index += next_utf8(to_push, utf_index)?;
            }

            if utf_index > 0 {
                buf.push_str(read_utf8(&to_push[..utf_index])?);
                count += utf_index;
                self.read_count += utf_index;
            }

            if !self.feed(read)? {
                return Ok(count);
            }
        }
    }

    /// `ReadBuf`'s fill buf equivalent. This will only pull data from the underlying read if the internal buffer is empty.
    /// # Errors
    /// Propagated from the `Read` impl
    pub fn fill_buf<T: Read>(&mut self, read: &mut T) -> io::Result<&[u8]> {
        if self.available() == 0 && !self.feed(read)? {
            return Ok(&[]);
        }

        Ok(&self.buffer.as_slice()[self.read_count..self.fill_count])
    }

    /// `ReadBuf`'s consume fn.
    /// In general, it should be paired with calls to `fill_buf`
    /// # Panics
    /// This function will panic if amt is > available
    ///
    pub fn consume(&mut self, amt: usize) {
        assert!(self.read_count + amt <= self.fill_count);
        self.read_count += amt;
    }

    /// Borrows this unowned buffer and associates it with `Read` impl.
    /// The returned `BorrowedReadBuffer` is both dyn `Read` and dyn `ReadBuf`.
    /// This may be necessary to call some api function from a library that expects such datatypes.
    /// The returned `BorrowedReadBuffer` is subject to the lifetime of both the read and self.
    ///
    pub fn borrow<'a, T: Read>(&'a mut self, read: &'a mut T) -> BorrowedReadBuffer<'a, T, S> {
        BorrowedReadBuffer { buffer: self, read }
    }
}

/// Borrowed dyn Read/ReadBuf of a `UnownedReadBuffer`.
/// This borrowed version is directly associated with a `Read` impl, but is subject to lifetimes.
pub struct BorrowedReadBuffer<'a, T: Read, const S: usize> {
    /// buffer ref
    buffer: &'a mut UnownedReadBuffer<S>,
    /// read ref
    read: &'a mut T,
}

impl<T: Read, const S: usize> Debug for BorrowedReadBuffer<'_, T, S> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&self.buffer, f)
    }
}

impl<T: Read, const S: usize> Read for BorrowedReadBuffer<'_, T, S> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.buffer.read(self.read, buf)
    }

    fn read_to_end(&mut self, buf: &mut Vec<u8>) -> io::Result<usize> {
        self.buffer.read_to_end(self.read, buf)
    }

    fn read_to_string(&mut self, buf: &mut String) -> io::Result<usize> {
        self.buffer.read_to_string(self.read, buf)
    }

    fn read_exact(&mut self, buf: &mut [u8]) -> io::Result<()> {
        self.buffer.read_exact(self.read, buf)
    }
}

impl<T: Read, const S: usize> BufRead for BorrowedReadBuffer<'_, T, S> {
    fn fill_buf(&mut self) -> io::Result<&[u8]> {
        self.buffer.fill_buf(self.read)
    }

    fn consume(&mut self, amt: usize) {
        self.buffer.consume(amt);
    }

    fn read_until(&mut self, byte: u8, buf: &mut Vec<u8>) -> io::Result<usize> {
        self.buffer.read_until(self.read, byte, buf)
    }

    fn read_line(&mut self, buf: &mut String) -> io::Result<usize> {
        self.buffer.read_line(self.read, buf)
    }
}

/// This fn returns the size of the next utf-8 character in bytes.
/// this can return 1,2,3,4 or Err.
/// Err is returned if the bit for an utf-8 continuation byte is set on the first byte.
/// Err is returned if any of the subsequent bytes do NOT have the utf-8 continuation bit set.
///
/// This fn does not check the buffer for bounds and assumes the caller ensures that at least 4 bytes
/// or an invalid utf-8 sequence is encountered before end of buffer.
fn next_utf8(to_push: &[u8], count: usize) -> io::Result<usize> {
    Ok(match utf8_len(to_push[count]) {
        1 => 1,
        2 => {
            utf8_cont_assert(to_push[count + 1])?;
            2
        }
        3 => {
            utf8_cont_assert(to_push[count + 1])?;
            utf8_cont_assert(to_push[count + 2])?;
            3
        }
        4 => {
            utf8_cont_assert(to_push[count + 1])?;
            utf8_cont_assert(to_push[count + 2])?;
            utf8_cont_assert(to_push[count + 3])?;
            4
        }
        _ => {
            return Err(io::Error::new(
                ErrorKind::InvalidData,
                "stream did not contain valid utf-8",
            ))
        }
    })
}

/// This fn does a `utf::from_utf8` safety check,
/// and then converts errors that should never exist (`Utf8Error`) to `io::Error`
fn read_utf8(to_push: &[u8]) -> io::Result<&str> {
    core::str::from_utf8(to_push).map_or_else(
        |_| {
            Err(io::Error::new(
                ErrorKind::InvalidData,
                "Unvalid UTF-8 detected",
            ))
        },
        Ok,
    )
}

/// This fn returns err if the given byte does not have the utf-8 continuation bits set.
fn utf8_cont_assert(cont: u8) -> io::Result<()> {
    if cont & 0b1100_0000 == 0b1000_0000 {
        return Ok(());
    }

    Err(io::Error::new(
        ErrorKind::InvalidData,
        "stream did not contain valid utf-8",
    ))
}

/// This fn returns the length in bytes the first utf-8 byte suggests.
/// 0 is returned for invalid first utf-8 bytes.
const fn utf8_len(first: u8) -> usize {
    if first & 0b1000_0000 == 0 {
        return 1;
    }

    if first & 0b1110_0000 == 0b1100_0000 {
        return 2;
    }

    if first & 0b1111_0000 == 0b1110_0000 {
        return 3;
    }

    if first & 0b1111_1000 == 0b1111_0000 {
        return 4;
    }

    //INVALID
    0
}
