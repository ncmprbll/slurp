use std::{fs, io::Read, process::exit};

const BUFFER_SIZE: u64 = 1 << 14;

fn main() {
    let file = fs::File::open("reference.html").unwrap_or_else(|err| {
        eprintln!("Failed to open the file: {err}");
        exit(1);
    });
    let mut limited_reader = file.take(BUFFER_SIZE);
    let mut data = String::new();

    if let Err(err) = limited_reader.read_to_string(&mut data) {
        eprintln!("failed to read the file: {err}");
        exit(1);
    };

    println!("First line:\n{:?}", data.lines().next());
}
