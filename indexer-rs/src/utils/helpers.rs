use std::{
    fs::File,
    io::{self, Write},
};

use alloy::{
    primitives::{hex},
};

pub fn save_bytes_to_file(path: &str, bytes: Vec<u8>) -> io::Result<()> {
    let mut file = File::create(path)?;
    file.write_all(hex::encode(bytes).as_bytes())?;
    file.flush()?;
    Ok(())
}

