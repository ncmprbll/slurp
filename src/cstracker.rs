use chrono::{DateTime, SubsecRound, Utc};
use std::fmt::{self, Debug, Display};

const MATCH_SECTION_REGEX: &str =
    r#"(?s)<section.+?href="\/matches\/\d+?">(.+?)<.+?data-time-ago="(\d+?)".+?<\/section>"#;
const MATCH_MESSAGE_REGEX: &str =
    r#"(?s)((?:R\d{1,3}|Round \?) · \d\d:\d\d\.\d\d).+?data-report-message-content="(.*?)""#;

#[derive(Debug)]
pub enum MatchRequestError {
    Http(reqwest::Error),
    Other(&'static str),
}

impl fmt::Display for MatchRequestError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            MatchRequestError::Http(e) => write!(f, "{}", e),
            MatchRequestError::Other(s) => write!(f, "{}", s),
        }
    }
}

impl From<reqwest::Error> for MatchRequestError {
    fn from(err: reqwest::Error) -> Self {
        MatchRequestError::Http(err)
    }
}

#[derive(Debug, PartialEq)]
pub struct Match {
    pub map: String,
    pub date: DateTime<Utc>,
    pub messages: Vec<Message>,
}

#[derive(Debug, PartialEq)]
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

#[derive(Debug, PartialEq)]
pub struct Message {
    pub round: Round,
    pub date: DateTime<Utc>,
    pub content: String,
}

pub fn get_match_history(
    steam_id: u64,
    user_agent: Option<&str>,
) -> Result<String, MatchRequestError> {
    let client = reqwest::blocking::Client::builder()
        .user_agent(user_agent.unwrap_or("PostmanRuntime/7.56.1")) // They're okay with Postman's user agent
        .build()?;

    let text = client
        .get(format!(
            "https://cstracker.gg/players/{}/sections/chat",
            steam_id
        ))
        .send()?
        .text()?;

    if text.contains("Just a moment...") {
        return Err(MatchRequestError::Other(
            "hit by Cloudflare (try changing user agent or manually accessing cstracker.gg in the browser to whitelist your ip)",
        ));
    }

    Ok(text)
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
                date: (match_date + offset).round_subsecs(0),
                content: message.to_string(),
            });
        }

        matches.push(m);
    }

    Ok(matches)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parsing() {
        assert_eq!(
            parse_match_history(r#"<section id="player-chat-section" class="mt-6"><header class="mb-4 flex flex-col gap-2 sm:grid sm:grid-cols-[minmax(0,1fr)_minmax(0,2fr)_minmax(0,1fr)] sm:items-center sm:gap-3"><div class="flex items-center justify-between gap-3 sm:contents"><span class="mono shrink-0 text-[10px] uppercase tracking-[0.3em] text-amber-400/80 sm:col-start-1 sm:row-start-1">// 02 · chat</span><div class="flex shrink-0 items-center justify-end gap-3 text-right sm:col-start-3 sm:row-start-1"><span class="mono text-[10px] uppercase tracking-[0.18em] text-slate-500">70 messages · 17 matches</span></div></div><div class="flex min-w-0 items-center gap-3 sm:col-start-2 sm:row-start-1"><div class="h-px flex-1 bg-gradient-to-r from-transparent via-slate-800 to-slate-800"></div><h2 class="display-serif-off shrink-0 text-center text-lg tracking-tight text-white sm:text-xl">Chat history </h2><div class="h-px flex-1 bg-gradient-to-r from-slate-800 via-slate-800 to-transparent"></div></div></header><div class="grid gap-4 lg:grid-cols-2"><div class="min-w-0"><div class="max-h-[50vh] space-y-3 overflow-y-auto pr-1"><section class="border border-slate-800/80 bg-ink-2"><div class="flex items-center justify-between gap-3 border-b border-slate-800 px-4 py-3"><a class="font-medium text-cyan-200" href="/matches/41563062">Dust2</a><span class="text-xs text-slate-500"><span class="whitespace-nowrap" data-time-ago="1786595027" title="played 2026-08-13 04:23">played 2026-08-13 04:23</span></span></div><div class="divide-y divide-slate-800 py-2"><div class="grid grid-cols-[auto_minmax(0,1fr)_auto] items-start gap-3 px-4 py-1 hover:bg-slate-800"><a class="contents" href="/matches/41563062#timeline-chat-822092702"><span class="pt-1 text-xs text-slate-500">Round ? · 18:23.23</span><span class="whitespace-pre-wrap break-words pt-0.5 text-sm text-slate-100">ty</span></a><button type="button" class="group inline-flex h-7 w-7 shrink-0 items-center justify-center text-slate-500 transition-colors hover:text-red-500 focus:outline-none focus-visible:text-red-500 focus-visible:ring-2 focus-visible:ring-red-500/70" title="Report this message" aria-label="Report this message" aria-haspopup="dialog" aria-controls="report-message-modal" data-report-message-trigger="822092702" data-report-message-author="76561197999004010" data-report-message-content="ty" data-report-message-reported="false"><svg class="h-4 w-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" aria-hidden="true"><path stroke-linecap="round" stroke-linejoin="round" d="M5 21V4m0 0h10.5l-1.75 3L15.5 10H5m0-6v6"></path></svg></button></div></div></section></div></div><aside class="min-w-0 border border-slate-800/80 bg-ink-2" aria-label="Player chat word frequency map"><div class="flex flex-wrap items-center justify-between gap-2 border-b border-slate-800/80 px-4 py-3"><span class="mono text-[10px] uppercase tracking-[0.22em] text-amber-300">// word frequency</span><span class="mono text-[9px] uppercase tracking-[0.18em] text-slate-600">64 common words</span></div><div class="flex min-h-64 flex-wrap content-center items-center justify-center gap-x-4 gap-y-3 overflow-hidden px-5 py-8 text-center" role="list" aria-label="Word frequency map"><span class="mono inline-block leading-none transition text-amber-200 hover:text-amber-100" style="font-size:2.780rem" title="fuck · 8 mentions" aria-label="fuck: 8 mentions" role="listitem">fuck</span><span class="mono inline-block leading-none transition text-cyan-200 hover:text-cyan-100" style="font-size:2.446rem" title="bro · 6 mentions" aria-label="bro: 6 mentions" role="listitem">bro</span><span class="mono inline-block leading-none transition text-slate-200 hover:text-white" style="font-size:2.241rem" title="shit · 5 mentions" aria-label="shit: 5 mentions" role="listitem">shit</span><span class="mono inline-block leading-none transition text-orange-200 hover:text-orange-100" style="font-size:1.998rem" title="got · 4 mentions" aria-label="got: 4 mentions" role="listitem">got</span><span class="mono inline-block leading-none transition text-sky-300 hover:text-sky-200" style="font-size:1.998rem" title="hurry · 4 mentions" aria-label="hurry: 4 mentions" role="listitem">hurry</span><span class="mono inline-block leading-none transition text-amber-200 hover:text-amber-100" style="font-size:1.702rem" title="bless · 3 mentions" aria-label="bless: 3 mentions" role="listitem">bless</span><span class="mono inline-block leading-none transition text-cyan-200 hover:text-cyan-100" style="font-size:1.702rem" title="cheating · 3 mentions" aria-label="cheating: 3 mentions" role="listitem">cheating</span><span class="mono inline-block leading-none transition text-slate-200 hover:text-white" style="font-size:1.702rem" title="gg · 3 mentions" aria-label="gg: 3 mentions" role="listitem">gg</span><span class="mono inline-block leading-none transition text-orange-200 hover:text-orange-100" style="font-size:1.702rem" title="like · 3 mentions" aria-label="like: 3 mentions" role="listitem">like</span><span class="mono inline-block leading-none transition text-sky-300 hover:text-sky-200" style="font-size:1.702rem" title="one · 3 mentions" aria-label="one: 3 mentions" role="listitem">one</span><span class="mono inline-block leading-none transition text-amber-200 hover:text-amber-100" style="font-size:1.702rem" title="rofl · 3 mentions" aria-label="rofl: 3 mentions" role="listitem">rofl</span><span class="mono inline-block leading-none transition text-cyan-200 hover:text-cyan-100" style="font-size:1.319rem" title="bay · 2 mentions" aria-label="bay: 2 mentions" role="listitem">bay</span><span class="mono inline-block leading-none transition text-slate-200 hover:text-white" style="font-size:1.319rem" title="cheats · 2 mentions" aria-label="cheats: 2 mentions" role="listitem">cheats</span><span class="mono inline-block leading-none transition text-orange-200 hover:text-orange-100" style="font-size:1.319rem" title="dont · 2 mentions" aria-label="dont: 2 mentions" role="listitem">dont</span><span class="mono inline-block leading-none transition text-sky-300 hover:text-sky-200" style="font-size:1.319rem" title="entire · 2 mentions" aria-label="entire: 2 mentions" role="listitem">entire</span><span class="mono inline-block leading-none transition text-amber-200 hover:text-amber-100" style="font-size:1.319rem" title="game · 2 mentions" aria-label="game: 2 mentions" role="listitem">game</span><span class="mono inline-block leading-none transition text-cyan-200 hover:text-cyan-100" style="font-size:1.319rem" title="guys · 2 mentions" aria-label="guys: 2 mentions" role="listitem">guys</span><span class="mono inline-block leading-none transition text-slate-200 hover:text-white" style="font-size:1.319rem" title="lose · 2 mentions" aria-label="lose: 2 mentions" role="listitem">lose</span><span class="mono inline-block leading-none transition text-orange-200 hover:text-orange-100" style="font-size:1.319rem" title="name · 2 mentions" aria-label="name: 2 mentions" role="listitem">name</span><span class="mono inline-block leading-none transition text-sky-300 hover:text-sky-200" style="font-size:1.319rem" title="re · 2 mentions" aria-label="re: 2 mentions" role="listitem">re</span><span class="mono inline-block leading-none transition text-amber-200 hover:text-amber-100" style="font-size:1.319rem" title="sign · 2 mentions" aria-label="sign: 2 mentions" role="listitem">sign</span><span class="mono inline-block leading-none transition text-cyan-200 hover:text-cyan-100" style="font-size:1.319rem" title="sorry · 2 mentions" aria-label="sorry: 2 mentions" role="listitem">sorry</span><span class="mono inline-block leading-none transition text-slate-200 hover:text-white" style="font-size:1.319rem" title="start · 2 mentions" aria-label="start: 2 mentions" role="listitem">start</span><span class="mono inline-block leading-none transition text-orange-200 hover:text-orange-100" style="font-size:1.319rem" title="yo · 2 mentions" aria-label="yo: 2 mentions" role="listitem">yo</span><span class="mono inline-block leading-none transition text-sky-300 hover:text-sky-200" style="font-size:0.780rem" title="ahahaha · 1 mentions" aria-label="ahahaha: 1 mentions" role="listitem">ahahaha</span><span class="mono inline-block leading-none transition text-amber-200 hover:text-amber-100" style="font-size:0.780rem" title="ahahahhahahaha · 1 mentions" aria-label="ahahahhahahaha: 1 mentions" role="listitem">ahahahhahahaha</span><span class="mono inline-block leading-none transition text-cyan-200 hover:text-cyan-100" style="font-size:0.780rem" title="ahuahua · 1 mentions" aria-label="ahuahua: 1 mentions" role="listitem">ahuahua</span><span class="mono inline-block leading-none transition text-slate-200 hover:text-white" style="font-size:0.780rem" title="anythings · 1 mentions" aria-label="anythings: 1 mentions" role="listitem">anythings</span><span class="mono inline-block leading-none transition text-orange-200 hover:text-orange-100" style="font-size:0.780rem" title="appreciate · 1 mentions" aria-label="appreciate: 1 mentions" role="listitem">appreciate</span><span class="mono inline-block leading-none transition text-sky-300 hover:text-sky-200" style="font-size:0.780rem" title="area · 1 mentions" aria-label="area: 1 mentions" role="listitem">area</span><span class="mono inline-block leading-none transition text-amber-200 hover:text-amber-100" style="font-size:0.780rem" title="asians · 1 mentions" aria-label="asians: 1 mentions" role="listitem">asians</span><span class="mono inline-block leading-none transition text-cyan-200 hover:text-cyan-100" style="font-size:0.780rem" title="banh · 1 mentions" aria-label="banh: 1 mentions" role="listitem">banh</span><span class="mono inline-block leading-none transition text-slate-200 hover:text-white" style="font-size:0.780rem" title="better · 1 mentions" aria-label="better: 1 mentions" role="listitem">better</span><span class="mono inline-block leading-none transition text-orange-200 hover:text-orange-100" style="font-size:0.780rem" title="boring · 1 mentions" aria-label="boring: 1 mentions" role="listitem">boring</span><span class="mono inline-block leading-none transition text-sky-300 hover:text-sky-200" style="font-size:0.780rem" title="bot · 1 mentions" aria-label="bot: 1 mentions" role="listitem">bot</span><span class="mono inline-block leading-none transition text-amber-200 hover:text-amber-100" style="font-size:0.780rem" title="broken · 1 mentions" aria-label="broken: 1 mentions" role="listitem">broken</span><span class="mono inline-block leading-none transition text-cyan-200 hover:text-cyan-100" style="font-size:0.780rem" title="buck · 1 mentions" aria-label="buck: 1 mentions" role="listitem">buck</span><span class="mono inline-block leading-none transition text-slate-200 hover:text-white" style="font-size:0.780rem" title="buddy · 1 mentions" aria-label="buddy: 1 mentions" role="listitem">buddy</span><span class="mono inline-block leading-none transition text-orange-200 hover:text-orange-100" style="font-size:0.780rem" title="buy · 1 mentions" aria-label="buy: 1 mentions" role="listitem">buy</span><span class="mono inline-block leading-none transition text-sky-300 hover:text-sky-200" style="font-size:0.780rem" title="cali · 1 mentions" aria-label="cali: 1 mentions" role="listitem">cali</span><span class="mono inline-block leading-none transition text-amber-200 hover:text-amber-100" style="font-size:0.780rem" title="cant · 1 mentions" aria-label="cant: 1 mentions" role="listitem">cant</span><span class="mono inline-block leading-none transition text-cyan-200 hover:text-cyan-100" style="font-size:0.780rem" title="cheat · 1 mentions" aria-label="cheat: 1 mentions" role="listitem">cheat</span><span class="mono inline-block leading-none transition text-slate-200 hover:text-white" style="font-size:0.780rem" title="clock · 1 mentions" aria-label="clock: 1 mentions" role="listitem">clock</span><span class="mono inline-block leading-none transition text-orange-200 hover:text-orange-100" style="font-size:0.780rem" title="computer · 1 mentions" aria-label="computer: 1 mentions" role="listitem">computer</span><span class="mono inline-block leading-none transition text-sky-300 hover:text-sky-200" style="font-size:0.780rem" title="corner · 1 mentions" aria-label="corner: 1 mentions" role="listitem">corner</span><span class="mono inline-block leading-none transition text-amber-200 hover:text-amber-100" style="font-size:0.780rem" title="dammm · 1 mentions" aria-label="dammm: 1 mentions" role="listitem">dammm</span><span class="mono inline-block leading-none transition text-cyan-200 hover:text-cyan-100" style="font-size:0.780rem" title="deag · 1 mentions" aria-label="deag: 1 mentions" role="listitem">deag</span><span class="mono inline-block leading-none transition text-slate-200 hover:text-white" style="font-size:0.780rem" title="deags · 1 mentions" aria-label="deags: 1 mentions" role="listitem">deags</span><span class="mono inline-block leading-none transition text-orange-200 hover:text-orange-100" style="font-size:0.780rem" title="didnt · 1 mentions" aria-label="didnt: 1 mentions" role="listitem">didnt</span><span class="mono inline-block leading-none transition text-sky-300 hover:text-sky-200" style="font-size:0.780rem" title="disown · 1 mentions" aria-label="disown: 1 mentions" role="listitem">disown</span><span class="mono inline-block leading-none transition text-amber-200 hover:text-amber-100" style="font-size:0.780rem" title="dogwarts · 1 mentions" aria-label="dogwarts: 1 mentions" role="listitem">dogwarts</span><span class="mono inline-block leading-none transition text-cyan-200 hover:text-cyan-100" style="font-size:0.780rem" title="doing · 1 mentions" aria-label="doing: 1 mentions" role="listitem">doing</span><span class="mono inline-block leading-none transition text-slate-200 hover:text-white" style="font-size:0.780rem" title="don · 1 mentions" aria-label="don: 1 mentions" role="listitem">don</span><span class="mono inline-block leading-none transition text-orange-200 hover:text-orange-100" style="font-size:0.780rem" title="doug · 1 mentions" aria-label="doug: 1 mentions" role="listitem">doug</span><span class="mono inline-block leading-none transition text-sky-300 hover:text-sky-200" style="font-size:0.780rem" title="dude · 1 mentions" aria-label="dude: 1 mentions" role="listitem">dude</span><span class="mono inline-block leading-none transition text-amber-200 hover:text-amber-100" style="font-size:0.780rem" title="embarrassing · 1 mentions" aria-label="embarrassing: 1 mentions" role="listitem">embarrassing</span><span class="mono inline-block leading-none transition text-cyan-200 hover:text-cyan-100" style="font-size:0.780rem" title="every · 1 mentions" aria-label="every: 1 mentions" role="listitem">every</span><span class="mono inline-block leading-none transition text-slate-200 hover:text-white" style="font-size:0.780rem" title="existence · 1 mentions" aria-label="existence: 1 mentions" role="listitem">existence</span><span class="mono inline-block leading-none transition text-orange-200 hover:text-orange-100" style="font-size:0.780rem" title="faceit · 1 mentions" aria-label="faceit: 1 mentions" role="listitem">faceit</span><span class="mono inline-block leading-none transition text-sky-300 hover:text-sky-200" style="font-size:0.780rem" title="family · 1 mentions" aria-label="family: 1 mentions" role="listitem">family</span><span class="mono inline-block leading-none transition text-amber-200 hover:text-amber-100" style="font-size:0.780rem" title="fenom · 1 mentions" aria-label="fenom: 1 mentions" role="listitem">fenom</span><span class="mono inline-block leading-none transition text-cyan-200 hover:text-cyan-100" style="font-size:0.780rem" title="filet · 1 mentions" aria-label="filet: 1 mentions" role="listitem">filet</span><span class="mono inline-block leading-none transition text-slate-200 hover:text-white" style="font-size:0.780rem" title="filetmignonnn · 1 mentions" aria-label="filetmignonnn: 1 mentions" role="listitem">filetmignonnn</span><span class="mono inline-block leading-none transition text-orange-200 hover:text-orange-100" style="font-size:0.780rem" title="forget · 1 mentions" aria-label="forget: 1 mentions" role="listitem">forget</span></div></aside></div></section>"#).unwrap(),
            vec![Match { map: "Dust2".to_owned(), date: DateTime::parse_from_str("2026-08-13T04:23:47 +00:00", "%Y-%m-%dT%H:%M:%S %z").unwrap().into(), messages: vec![Message{ round: Round::Unknown, date: DateTime::parse_from_str("2026-08-13T04:23:47 +00:00", "%Y-%m-%dT%H:%M:%S %z").unwrap().into(), content: "ty".to_string() }] }],
        )
    }
}
