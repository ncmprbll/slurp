use chrono::{DateTime, Utc};
use std::{fs, io::Read, process::exit, time};

const BUFFER_SIZE: u64 = 1 << 14;
const REGEX: &str = r#"(?s)href="\/matches\/\d+?">(.+?)<.+?data-time-ago="(\d+?)".+?((?:R\d{1,3}|Round \?) · \d\d:\d\d\.\d\d).+?data-report-message-content="(.*?)""#;

#[derive(Debug)]
struct Match {
    map: String,
    date: DateTime<Utc>,
}

#[derive(Debug)]
enum Round {
    Unknown,
    Number(u8),
}

#[derive(Debug)]
struct Message {
    round: Round,
    date: DateTime<Utc>,
    content: String,
}

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
        let map = match capture.get(1) {
            Some(v) => v.as_str(),
            None => continue,
        };

        let match_date = match capture.get(2) {
            Some(v) => v.as_str(),
            None => continue,
        };

        let round_and_offset = match capture.get(3) {
            Some(v) => v.as_str(),
            None => continue,
        };

        let message = match capture.get(4) {
            Some(v) => v.as_str(),
            None => continue,
        };

        let match_date = match match_date.parse() {
            Ok(v) => v,
            Err(_) => continue,
        };

        let match_date = match DateTime::from_timestamp(match_date, 0) {
            Some(v) => v,
            None => continue,
        };

        let m = Match {
            map: map.to_string(),
            date: match_date,
        };

        let round_and_offset = round_and_offset.split(" · ").collect::<Vec<&str>>();

        if round_and_offset.len() != 2 {
            continue;
        }

        let round = round_and_offset[0];
        let offset = round_and_offset[1];

        let round = match round {
            "Round ?" => Round::Unknown,
            other => match other[1..].parse::<u8>() {
                Ok(v) => Round::Number(v),
                Err(_) => continue,
            },
        };

        let instants = offset
            .split(":")
            .flat_map(|instant| {
                return instant.split(".");
            })
            .collect::<Vec<&str>>();

        if instants.len() != 3 {
            continue;
        }

        let minutes_ms = match instants[0].parse::<i64>() {
            Ok(v) => v * 60000,
            Err(_) => continue,
        };

        let seconds_ms = match instants[1].parse::<i64>() {
            Ok(v) => v * 1000,
            Err(_) => continue,
        };

        let milliseconds = match instants[2].parse::<i64>() {
            Ok(v) => v * 10,
            Err(_) => continue,
        };

        let offset = chrono::Duration::milliseconds(minutes_ms + seconds_ms + milliseconds);

        let message = Message {
            round: round,
            date: match_date + offset,
            content: message.to_string(),
        };

        println!("{:?} {:?} {:?}", map, match_date, round_and_offset);

        println!("{:?}", m);
        println!("{:?}", message);
    }

    println!("First line:\n{:?}", data.lines().next());
}
