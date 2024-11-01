#![no_main]

use std::io::{Cursor, Write};
use libfuzzer_sys::fuzz_target;
use unowned_buf::UnownedReadBuffer;

fuzz_target!(|fuzz: &[u8]| {
    if fuzz.len() < 6 {
        return;
    }

    let mut buf_size = u32::from_be_bytes([0, fuzz[0], fuzz[1], fuzz[2]]) as usize;
    let mut read_size = u32::from_be_bytes([0, fuzz[3], fuzz[4], fuzz[5]]) as usize;
    if buf_size == 0 {
        buf_size = 1;
    }
    if read_size == 0 {
        read_size = 1;
    }

    let mut data = vec![0u8; buf_size];
    for (idx, j) in data.iter_mut().enumerate() {
        *j = fuzz[idx % fuzz.len()]
    }

    let copy = data.clone();
    let mut target = vec![0u8; buf_size];

    let mut src_cursor = Cursor::new(&mut data);
    let mut target_cursor = Cursor::new(&mut target);
    let mut buf = UnownedReadBuffer::default();
    loop {
        let mut cur_buf = vec![0u8; read_size];
        let read = buf.read(&mut src_cursor, cur_buf.as_mut_slice()).unwrap();
        if read == 0 {
            break;
        }

        target_cursor.write_all(&cur_buf[..read]).unwrap();
    }

    drop(src_cursor);
    drop(target_cursor);

    if target != data {
        panic!("target != data {} {}", buf_size, read_size);
    }

    if copy != data {
        panic!("copy != data {} {}", buf_size, read_size);
    }
});
