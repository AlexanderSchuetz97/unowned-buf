#![no_main]
extern crate core;

use std::io::{Cursor, Write};
use libfuzzer_sys::fuzz_target;
use unowned_buf::UnownedReadBuffer;

const COUNT: usize = 0x16_00_00;

fuzz_target!(|str: &str| {
    if str.len() == 0 {
        return;
    }
    let mut data = str.as_bytes().to_vec();
    while data.len() < COUNT {
        data.extend_from_slice(str.as_bytes());
    }


    let copy = data.clone();
    let mut target = vec![0u8; copy.len()];

    let mut src_cursor = Cursor::new(&mut data);
    let mut target_cursor = Cursor::new(&mut target);
    let mut buf = UnownedReadBuffer::default();
    let mut str = String::new();
    let n = buf.read_to_string(&mut src_cursor, &mut str).unwrap();
    assert_eq!(n, copy.len());
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
});
