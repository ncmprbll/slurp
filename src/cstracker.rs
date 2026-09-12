use chrono::{DateTime, Utc};
use reqwest::Result;
use std::fmt::Display;

const MATCH_SECTION_REGEX: &str =
    r#"(?s)<section.+?href="\/matches\/\d+?">(.+?)<.+?data-time-ago="(\d+?)".+?<\/section>"#;
const MATCH_MESSAGE_REGEX: &str =
    r#"(?s)((?:R\d{1,3}|Round \?) · \d\d:\d\d\.\d\d).+?data-report-message-content="(.*?)""#;

#[derive(Debug)]
pub struct Match {
    pub map: String,
    pub date: DateTime<Utc>,
    pub messages: Vec<Message>,
}

#[derive(Debug)]
pub enum Round {
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
pub struct Message {
    pub round: Round,
    pub date: DateTime<Utc>,
    pub content: String,
}

pub fn get_match_history(steam_id: u64) -> Result<String> {
    let client = reqwest::blocking::Client::builder()
        .user_agent("PostmanRuntime/7.56.1")
        .build()?;

    client
        .get(format!(
            "https://cstracker.gg/players/{}/sections/chat",
            steam_id
        ))
        .send()?
        .text()
}

pub fn parse_match_history(
    data: &str,
) -> std::result::Result<Vec<Match>, Box<dyn std::error::Error>> {
    let match_section_re = regex::Regex::new(MATCH_SECTION_REGEX)?;
    let match_message_re = regex::Regex::new(MATCH_MESSAGE_REGEX)?;
    let mut matches: Vec<Match> = Vec::new();

    for capture in match_section_re.captures_iter(data) {
        let section = match capture.get(0) {
            Some(v) => v.as_str(),
            None => continue,
        };

        let map = match capture.get(1) {
            Some(v) => v.as_str(),
            None => continue,
        };

        let match_date = match capture.get(2) {
            Some(v) => v.as_str(),
            None => continue,
        };

        let match_date = match_date.parse()?;

        let match_date = match DateTime::from_timestamp(match_date, 0) {
            Some(v) => v,
            None => continue,
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
                None => continue,
            };

            let round_and_offset = round_and_offset.split(" · ").collect::<Vec<&str>>();

            if round_and_offset.len() != 2 {
                continue;
            }

            let round = round_and_offset[0];
            let offset = round_and_offset[1];

            let round = match round {
                "Round ?" => Round::Unknown,
                other => Round::Number(other[1..].parse::<u8>()?),
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

            let minutes_ms = instants[0].parse::<i64>()?;
            let seconds_ms = instants[1].parse::<i64>()?;
            let centiseconds_ms = instants[2].parse::<i64>()?;
            let offset = chrono::Duration::milliseconds(minutes_ms + seconds_ms + centiseconds_ms);

            m.messages.push(Message {
                round: round,
                date: match_date + offset,
                content: message.to_string(),
            });
        }

        matches.push(m);
    }

    Ok(matches)
}
