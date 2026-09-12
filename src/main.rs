mod cstracker;
mod steam;

use chrono::{DateTime, Utc};
use std::{env, fmt::Display, fs, io::Read, process::exit};

const BUFFER_SIZE: u64 = 1 << 18;
const MATCH_SECTION_REGEX: &str =
    r#"(?s)<section.+?href="\/matches\/\d+?">(.+?)<.+?data-time-ago="(\d+?)".+?<\/section>"#;
const MATCH_MESSAGE_REGEX: &str =
    r#"(?s)((?:R\d{1,3}|Round \?) · \d\d:\d\d\.\d\d).+?data-report-message-content="(.*?)""#;

#[derive(Debug)]
struct Match {
    map: String,
    date: DateTime<Utc>,
    messages: Vec<Message>,
}

#[derive(Debug)]
enum Round {
    Unknown,
    Number(u8),
}

impl Display for Round {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Round::Unknown => "??".to_string(),
                Round::Number(n) => format!("{:0>2}", n),
            }
        )
    }
}

#[derive(Debug)]
struct Message {
    round: Round,
    date: DateTime<Utc>,
    content: String,
}

fn main() {
    let mut args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        return;
    }

    let steam_id = args.swap_remove(1);
    let steam_id = match steam_id.parse::<steam::SteamId>() {
        Ok(v) => v,
        Err(err) => {
            eprintln!("{err}");
            exit(1)
        }
    };

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

    let match_section_re = regex::Regex::new(MATCH_SECTION_REGEX).unwrap_or_else(|err| {
        eprintln!("section regex failed: {err}");
        exit(1);
    });

    let match_message_re = regex::Regex::new(MATCH_MESSAGE_REGEX).unwrap_or_else(|err| {
        eprintln!("message regex failed: {err}");
        exit(1);
    });

    let mut matches: Vec<Match> = Vec::new();

    for capture in match_section_re.captures_iter(&data) {
        let section = match capture.get(0) {
            Some(v) => v.as_str(),
            None => {
                eprintln!("section is none");
                continue;
            }
        };

        let map = match capture.get(1) {
            Some(v) => v.as_str(),
            None => {
                eprintln!("map is none");
                continue;
            }
        };

        let match_date = match capture.get(2) {
            Some(v) => v.as_str(),
            None => {
                eprintln!("match_date is none");
                continue;
            }
        };

        let match_date = match match_date.parse() {
            Ok(v) => v,
            Err(err) => {
                eprintln!("failed to parse match date {err}");
                continue;
            }
        };

        let match_date = match DateTime::from_timestamp(match_date, 0) {
            Some(v) => v,
            None => {
                eprintln!("failed to create datetime");
                continue;
            }
        };

        let mut m = Match {
            map: map.to_string(),
            date: match_date,
            messages: Vec::new(),
        };

        for capture in match_message_re.captures_iter(&section) {
            let round_and_offset = match capture.get(1) {
                Some(v) => v.as_str(),
                None => {
                    eprintln!("round_and_offset is none");
                    continue;
                }
            };

            let message = match capture.get(2) {
                Some(v) => v.as_str(),
                None => {
                    eprintln!("message is none");
                    continue;
                }
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
                    Err(err) => {
                        eprintln!("failed to parse round number {err}");
                        continue;
                    }
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
                Err(err) => {
                    eprintln!("failed to parse minutes {err}");
                    continue;
                }
            };

            let seconds_ms = match instants[1].parse::<i64>() {
                Ok(v) => v * 1000,
                Err(err) => {
                    eprintln!("failed to parse seconds {err}");
                    continue;
                }
            };

            let centiseconds_ms = match instants[2].parse::<i64>() {
                Ok(v) => v * 10,
                Err(err) => {
                    eprintln!("failed to parse centiseconds {err}");
                    continue;
                }
            };

            let offset = chrono::Duration::milliseconds(minutes_ms + seconds_ms + centiseconds_ms);

            m.messages.push(Message {
                round: round,
                date: match_date + offset,
                content: message.to_string(),
            });
        }

        matches.push(m);
    }

    for m in matches {
        println!(
            "| {} at {}",
            m.map,
            format!("{}", m.date.format("%Y-%m-%d %H:%M:%S"))
        );

        for message in m.messages {
            println!(
                "* R{} at {}: {}",
                message.round,
                format!("{}", message.date.format("%H:%M:%S")),
                message.content
            )
        }
    }
}
