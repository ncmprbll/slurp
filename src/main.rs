use std::{fs, io::Read, process::exit};

const BUFFER_SIZE: u64 = 1 << 14;
const REGEX: &str = r#"(?s)href="\/matches\/\d+?">(.+?)<.+?data-time-ago="(\d+?)".+?((?:R\d{1,3}|Round \?) · \d\d:\d\d\.\d\d).+?data-report-message-content="(.*?)""#;

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

    let re = regex::Regex::new(REGEX).unwrap_or_else(|err| {
        eprintln!("regex failed: {err}");
        exit(1);
    });

    for capture in re.captures_iter(&data) {
        let map = capture.get(1).unwrap();
        let match_date = capture.get(2).unwrap();
        let round_and_offset = capture.get(3).unwrap();
        let message = capture.get(4).unwrap();

        println!(
            "{:?} {:?} {:?} {:?}",
            map, match_date, round_and_offset, message
        );
    }

    println!("First line:\n{:?}", data.lines().next());
}
