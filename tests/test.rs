extern crate core;

use rand::random;
use std::io::{Cursor, Write};
use unowned_buf::{UnownedReadBuffer, UnownedWriteBuffer};

#[cfg(not(miri))]
const COUNT: usize = 0x1_00_00;
#[cfg(miri)]
const COUNT: usize = 0x4_00;

#[cfg(miri)]
const RAND_SIZE: usize = 63;

#[cfg(not(miri))]
const RAND_SIZE: usize = 4095;

#[test]
pub fn test_read() {
    let mut data = vec![0u8; COUNT];
    for j in data.iter_mut() {
        *j = random()
    }

    let copy = data.clone();
    let mut target = vec![0u8; COUNT];

    let mut src_cursor = Cursor::new(&mut data);
    let mut target_cursor = Cursor::new(&mut target);
    let mut buf = UnownedReadBuffer::default();
    loop {
        let buf_size = (random::<usize>() % RAND_SIZE) + 1;
        let mut cur_buf = vec![0u8; buf_size];
        let read = buf.read(&mut src_cursor, cur_buf.as_mut_slice()).unwrap();
        if read == 0 {
            break;
        }

        target_cursor.write_all(&cur_buf[..read]).unwrap();
    }

    drop(src_cursor);
    drop(target_cursor);

    assert_eq!(target, data);
    assert_eq!(copy, data);
}

#[test]
pub fn test_read_exact() {
    let mut data = vec![0u8; COUNT];
    for j in data.iter_mut() {
        *j = random()
    }

    let copy = data.clone();
    let mut target = vec![0u8; COUNT];

    let mut src_cursor = Cursor::new(&mut data);
    let mut target_cursor = Cursor::new(&mut target);
    let mut buf = UnownedReadBuffer::default();
    loop {
        let rem = COUNT as u64 - target_cursor.position();
        if rem == 0 {
            break;
        }
        let mut buf_size = (random::<usize>() % 4095) + 1;
        if buf_size > rem as usize {
            buf_size = rem as usize;
        }

        let mut cur_buf = vec![0u8; buf_size];
        buf.read_exact(&mut src_cursor, cur_buf.as_mut_slice())
            .unwrap();

        target_cursor.write_all(cur_buf.as_slice()).unwrap();
    }

    drop(src_cursor);
    drop(target_cursor);

    assert_eq!(target, data);
    assert_eq!(copy, data);
}

#[test]
pub fn test_read_until() {
    let mut data = vec![0u8; COUNT];
    for j in data.iter_mut() {
        *j = random()
    }

    let copy = data.clone();
    let mut target = vec![0u8; COUNT];

    let mut src_cursor = Cursor::new(&mut data);
    let mut target_cursor = Cursor::new(&mut target);
    let mut buf = UnownedReadBuffer::default();
    loop {
        let rem = COUNT as u64 - target_cursor.position();
        if rem == 0 {
            break;
        }
        let mut buf_size = (random::<usize>() % RAND_SIZE) + 1;
        if buf_size > rem as usize {
            buf_size = rem as usize;
        }

        let mut cur_buf = vec![0u8; buf_size];
        buf.read_exact(&mut src_cursor, cur_buf.as_mut_slice())
            .unwrap();

        target_cursor.write_all(cur_buf.as_slice()).unwrap();
    }

    drop(src_cursor);
    drop(target_cursor);

    assert_eq!(target, data);
    assert_eq!(copy, data);
}

fn ascii() -> Vec<u8> {
    let mut dta: Vec<u8> = Vec::new();
    for i in b'A'..b'Z' {
        dta.push(i);
    }

    for i in b'a'..b'z' {
        dta.push(i);
    }

    for i in b'0'..b'9' {
        dta.push(i);
    }

    dta.push(b'_');

    dta.push(b'-');

    dta.push(b'/');
    dta.push(b'\\');
    dta
}

#[test]
pub fn test_read_to_end() {
    let mut data = vec![0u8; COUNT];
    for j in data.iter_mut() {
        *j = random()
    }

    let copy = data.clone();
    let mut target = Vec::new();

    let mut src_cursor = Cursor::new(&mut data);
    let mut buf = UnownedReadBuffer::default();
    let size = buf
        .read_to_end(&mut src_cursor, &mut target)
        .expect("Error");
    assert_eq!(size, COUNT);
    assert_eq!(size, target.len());
    drop(src_cursor);

    if copy != data {
        panic!("copy != data");
    }

    if target != data {
        panic!("target != data");
    }
}

#[test]
pub fn test_read_string() {
    let characters = ascii();

    let mut data = vec![0u8; COUNT];
    for j in data.iter_mut() {
        *j = characters[random::<usize>() % characters.len()]
    }

    for i in data.iter_mut() {
        if random::<u8>() < 32 {
            *i = b'\n';
        }
    }

    let copy = data.clone();
    let mut target = vec![0u8; COUNT];

    let mut src_cursor = Cursor::new(&mut data);
    let mut target_cursor = Cursor::new(&mut target);
    let mut buf = UnownedReadBuffer::default();
    let mut str = String::new();
    let n = buf.read_to_string(&mut src_cursor, &mut str).unwrap();
    assert_eq!(n, COUNT);
    target_cursor.write_all(str.as_bytes()).unwrap();
    target_cursor.flush().unwrap();

    drop(src_cursor);
    drop(target_cursor);

    if copy != data {
        panic!("copy != data");
    }

    if target != data {
        panic!("target != data");
    }
}

#[test]
pub fn test_read_line() {
    let characters = ascii();

    let mut data = vec![0u8; COUNT];
    for j in data.iter_mut() {
        *j = characters[random::<usize>() % characters.len()]
    }

    for i in data.iter_mut() {
        if random::<u8>() < 32 {
            *i = b'\n';
        }
    }

    //let mut read_2 = Vec::new();

    let copy = data.clone();
    let mut target = vec![0u8; COUNT];

    let mut src_cursor = Cursor::new(&mut data);
    let mut target_cursor = Cursor::new(&mut target);
    let mut buf = UnownedReadBuffer::default();
    loop {
        let mut str = String::new();
        let n = buf.read_line(&mut src_cursor, &mut str).unwrap();
        assert_eq!(n, str.len());
        assert_eq!(
            &copy.as_slice()
                [target_cursor.position() as usize..target_cursor.position() as usize + str.len()],
            str.as_bytes()
        );

        if n == 0 {
            break;
        }
        //read_2.extend_from_slice(str.as_bytes());
        //assert_eq!(&copy.as_slice()[..read_2.len()], read_2.as_slice());

        target_cursor.write_all(str.as_bytes()).unwrap();
        target_cursor.flush().unwrap();
    }

    while target_cursor.position() != copy.len() as u64 {
        let d = copy[target_cursor.position() as usize];
        assert_ne!(d, b'\n');
        target_cursor.write_all(&[d]).unwrap();
    }

    drop(src_cursor);
    drop(target_cursor);

    if copy != data {
        panic!("copy != data");
    }

    if target != data {
        assert_eq!(target.len(), data.len());
        for x in 0..target.len() {
            if target[x] != data[x] {
                panic!("target != data {} {} {}", x, target[x], data[x]);
            }
        }
    }
}

#[test]
pub fn test_write_all() {
    let mut data = vec![0u8; COUNT];
    for j in data.iter_mut() {
        *j = random()
    }

    let copy = data.clone();
    let mut target = Vec::new();
    let mut buf = UnownedWriteBuffer::default();
    buf.write_all(&mut target, data.as_slice()).expect("ERR");
    buf.flush(&mut target).expect("ERR");

    if copy != data {
        panic!("copy != data");
    }

    if target != data {
        panic!("target != data");
    }
}

#[test]
pub fn test_write() {
    let mut data = vec![0u8; COUNT];
    for j in data.iter_mut() {
        *j = random()
    }

    let copy = data.clone();
    let mut target = Vec::new();
    let mut buf = UnownedWriteBuffer::default();
    let mut count = 0;
    loop {
        let len = buf
            .write(&mut target, &data.as_slice()[count..])
            .expect("ERR");
        count += len;
        if count == data.len() {
            break;
        }
    }
    buf.flush(&mut target).expect("ERR");

    if copy != data {
        panic!("copy != data");
    }

    if target != data {
        panic!("target != data");
    }
}
