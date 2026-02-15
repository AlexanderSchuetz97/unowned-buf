extern crate core;

use rand::random;
use std::io::{Cursor, Write};
use std::panic;
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

#[test]
pub fn test_read_until_limit_bug() {
    let mut data = vec![0xA, 0xB, 0xC, 0xD, 0xB, 0xE, 0xF];
    let mut src_cursor = Cursor::new(&mut data);
    let mut buf = UnownedReadBuffer::default();
    let mut target: Vec<u8> = Vec::new();
    assert_eq!(
        2,
        buf.read_until_limit(&mut src_cursor, 0xB, 16, &mut target)
            .expect("ERR")
    );
    assert_eq!(target, vec![0xA, 0xB]);
    assert_eq!(
        3,
        buf.read_until_limit(&mut src_cursor, 0xB, 16, &mut target)
            .expect("ERR")
    );
    assert_eq!(target, vec![0xA, 0xB, 0xC, 0xD, 0xB]);
    assert_eq!(
        2,
        buf.read_until_limit(&mut src_cursor, 0xB, 16, &mut target)
            .expect("ERR")
    );
    assert_eq!(target, vec![0xA, 0xB, 0xC, 0xD, 0xB, 0xE, 0xF]);
}

#[test]
pub fn test_read_until_limit_large() {
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
        let buf_size = (random::<usize>() % RAND_SIZE) + 1;

        let mut cur_buf = vec![];
        buf.read_until_limit(&mut src_cursor, random(), buf_size, &mut cur_buf)
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

#[test]
pub fn test_append_read_full() {
    let mut buffer = UnownedReadBuffer::<64>::new();
    buffer.copy_into_internal_buffer(&[0; 64]);
    let pnk = panic::catch_unwind(move || {
        _ = buffer.read_into_internal_buffer(&mut Cursor::new(vec![]));
    });

    assert!(pnk.is_err());
}

#[test]
pub fn test_append_copy_full() {
    let mut buffer = UnownedReadBuffer::<64>::new();
    buffer.copy_into_internal_buffer(&[0; 64]);
    buffer.copy_into_internal_buffer(&[]);

    let mut buffer = UnownedReadBuffer::<64>::new();
    buffer.copy_into_internal_buffer(&[0; 64]);

    let pnk = panic::catch_unwind(move || {
        buffer.copy_into_internal_buffer(&[0; 1]);
    });

    assert!(pnk.is_err());

    let mut buffer = UnownedReadBuffer::<64>::new();
    buffer.copy_into_internal_buffer(&[0; 32]);
    let pnk = panic::catch_unwind(move || {
        buffer.copy_into_internal_buffer(&[0; 33]);
    });

    assert!(pnk.is_err());
}

#[test]
pub fn test_append_read() {
    let mut buffer = UnownedReadBuffer::<64>::new();
    assert_eq!(buffer.read_count(), 0);
    assert_eq!(buffer.fill_count(), 0);
    assert_eq!(buffer.available_space(), 64);
    assert_eq!(buffer.internal_buffer_mut().len(), 0);
    assert_eq!(buffer.internal_buffer().len(), 0);

    assert_eq!(
        buffer
            .read_into_internal_buffer(&mut Cursor::new(vec![4, 1, 2, 3]))
            .unwrap(),
        4
    );

    assert_eq!(buffer.read_count(), 0);
    assert_eq!(buffer.fill_count(), 4);
    assert_eq!(buffer.available_space(), 60);

    let mut rbuf = [0u8; 32];
    let mut expected = [0u8; 32];
    expected[0] = 4;
    expected[1] = 1;
    expected[2] = 2;
    expected[3] = 3;

    assert_eq!(buffer.try_read(&mut rbuf), 4);
    assert_eq!(rbuf, expected);
    assert_eq!(buffer.read_count(), 0);
    assert_eq!(buffer.fill_count(), 0);
    assert_eq!(buffer.available_space(), 64);

    assert_eq!(
        buffer
            .read_into_internal_buffer(&mut Cursor::new(vec![6, 4, 3, 2]))
            .unwrap(),
        4
    );
    assert_eq!(buffer.read_count(), 0);
    assert_eq!(buffer.fill_count(), 4);
    assert_eq!(buffer.available_space(), 60);

    let mut rbuf = [0u8; 3];
    let expected = [6u8, 4, 3];
    assert_eq!(buffer.try_read(&mut rbuf), 3);
    assert_eq!(rbuf, expected);
    assert_eq!(buffer.read_count(), 3);
    assert_eq!(buffer.fill_count(), 4);
    assert_eq!(buffer.available_space(), 60);

    assert_eq!(
        buffer
            .read_into_internal_buffer(&mut Cursor::new(vec![7, 8, 9, 10]))
            .unwrap(),
        4
    );
    let mut rbuf = [0u8; 3];
    let expected = [2u8, 7, 8];
    assert_eq!(buffer.read_count(), 3);
    assert_eq!(buffer.fill_count(), 8);
    assert_eq!(buffer.available_space(), 56);
    assert_eq!(buffer.try_read(&mut rbuf), 3);
    assert_eq!(rbuf, expected);
    assert_eq!(buffer.read_count(), 6);
    assert_eq!(buffer.fill_count(), 8);
    assert_eq!(buffer.available_space(), 56);

    buffer.compact();
    assert_eq!(buffer.read_count(), 0);
    assert_eq!(buffer.fill_count(), 2);
    assert_eq!(buffer.available_space(), 62);

    let mut rbuf = [0u8; 3];
    let expected = [9u8, 10, 0];
    assert_eq!(buffer.try_read(&mut rbuf), 2);
    assert_eq!(rbuf, expected);
    assert_eq!(buffer.read_count(), 0);
    assert_eq!(buffer.fill_count(), 0);
    assert_eq!(buffer.available_space(), 64);

    assert_eq!(
        buffer
            .read_into_internal_buffer(&mut Cursor::new(vec![]))
            .unwrap(),
        0
    );
    assert_eq!(buffer.read_count(), 0);
    assert_eq!(buffer.fill_count(), 0);
    assert_eq!(buffer.available_space(), 64);
    assert_eq!(buffer.internal_buffer_mut().len(), 0);
    assert_eq!(buffer.internal_buffer().len(), 0);

    assert_eq!(
        buffer
            .read_into_internal_buffer(&mut Cursor::new(vec![64; 128]))
            .unwrap(),
        buffer.size()
    );
    assert_eq!(buffer.read_count(), 0);
    assert_eq!(buffer.fill_count(), 64);
    assert_eq!(buffer.available_space(), 0);
}

#[test]
pub fn test_append() {
    let mut buffer = UnownedReadBuffer::<64>::new();
    assert_eq!(buffer.read_count(), 0);
    assert_eq!(buffer.fill_count(), 0);
    assert_eq!(buffer.available_space(), 64);

    buffer.copy_into_internal_buffer(&[4, 1, 2, 3]);
    assert_eq!(buffer.read_count(), 0);
    assert_eq!(buffer.fill_count(), 4);
    assert_eq!(buffer.available_space(), 60);

    let mut rbuf = [0u8; 32];
    let mut expected = [0u8; 32];
    expected[0] = 4;
    expected[1] = 1;
    expected[2] = 2;
    expected[3] = 3;

    assert_eq!(buffer.try_read(&mut rbuf), 4);
    assert_eq!(rbuf, expected);
    assert_eq!(buffer.read_count(), 0);
    assert_eq!(buffer.fill_count(), 0);
    assert_eq!(buffer.available_space(), 64);

    buffer.copy_into_internal_buffer(&[6, 4, 3, 2]);
    assert_eq!(buffer.read_count(), 0);
    assert_eq!(buffer.fill_count(), 4);
    assert_eq!(buffer.available_space(), 60);

    let mut rbuf = [0u8; 3];
    let expected = [6u8, 4, 3];
    assert_eq!(buffer.try_read(&mut rbuf), 3);
    assert_eq!(rbuf, expected);
    assert_eq!(buffer.read_count(), 3);
    assert_eq!(buffer.fill_count(), 4);
    assert_eq!(buffer.available_space(), 60);

    buffer.copy_into_internal_buffer(&[7, 8, 9, 10]);
    let mut rbuf = [0u8; 3];
    let expected = [2u8, 7, 8];
    assert_eq!(buffer.read_count(), 3);
    assert_eq!(buffer.fill_count(), 8);
    assert_eq!(buffer.available_space(), 56);
    assert_eq!(buffer.internal_buffer().len(), 5);
    assert_eq!(buffer.internal_buffer_mut().len(), 5);
    assert_eq!(buffer.internal_buffer(), &[2, 7, 8, 9, 10]);
    assert_eq!(buffer.internal_buffer_mut(), &[2, 7, 8, 9, 10]);
    buffer.internal_buffer_mut()[4] = 129;

    assert_eq!(buffer.try_read(&mut rbuf), 3);
    assert_eq!(rbuf, expected);
    assert_eq!(buffer.read_count(), 6);
    assert_eq!(buffer.fill_count(), 8);
    assert_eq!(buffer.available_space(), 56);

    buffer.compact();
    assert_eq!(buffer.read_count(), 0);
    assert_eq!(buffer.fill_count(), 2);
    assert_eq!(buffer.available_space(), 62);

    buffer.skip(1);
    assert_eq!(buffer.read_count(), 1);
    assert_eq!(buffer.fill_count(), 2);
    assert_eq!(buffer.available_space(), 62);

    let mut rbuf = [0u8; 3];
    let expected = [129, 0, 0];
    assert_eq!(buffer.try_read(&mut rbuf), 1);
    assert_eq!(rbuf, expected);
    assert_eq!(buffer.read_count(), 0);
    assert_eq!(buffer.fill_count(), 0);
    assert_eq!(buffer.available_space(), 64);
    assert_eq!(buffer.internal_buffer_mut().len(), 0);
    assert_eq!(buffer.internal_buffer().len(), 0);
}

#[test]
pub fn test_try_write() {
    let mut buffer = UnownedWriteBuffer::<64>::new();
    assert_eq!(buffer.flushable(), 0);
    assert_eq!(buffer.available(), 64);

    assert_eq!(buffer.try_write(&[0, 1, 2, 3]), 4);
    assert_eq!(buffer.flushable(), 4);
    assert_eq!(buffer.available(), 60);

    let mut data = Vec::new();
    buffer.flush(&mut data).unwrap();
    assert_eq!(data.as_slice(), &[0, 1, 2, 3]);
    assert_eq!(buffer.flushable(), 0);
    assert_eq!(buffer.available(), 64);

    assert_eq!(buffer.try_write(&[5, 6, 7, 8]), 4);
    assert_eq!(buffer.try_write(&[9, 10, 11, 12]), 4);
    assert_eq!(buffer.internal_buffer().len(), 8);
    assert_eq!(buffer.internal_buffer_mut().len(), 8);

    assert_eq!(buffer.internal_buffer(), &[5, 6, 7, 8, 9, 10, 11, 12]);
    assert_eq!(buffer.internal_buffer_mut(), &[5, 6, 7, 8, 9, 10, 11, 12]);
    buffer.internal_buffer_mut()[6] = 44;

    assert_eq!(buffer.flushable(), 8);
    assert_eq!(buffer.available(), 56);
    assert_eq!(buffer.try_write(&[57; 57]), 56);
    assert_eq!(buffer.flushable(), 64);
    assert_eq!(buffer.available(), 0);

    data.truncate(0);
    buffer.flush(&mut data).unwrap();
    assert_eq!(&data.as_slice()[..8], &[5, 6, 7, 8, 9, 10, 44, 12]);
    assert_eq!(&data.as_slice()[8..], &[57; 56]);
    assert_eq!(buffer.flushable(), 0);
    assert_eq!(buffer.available(), 64);
}
