mod cstracker;
mod steam;

use std::{env, process::exit};

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

    let chat_history = cstracker::get_match_history(steam_id.0, None).unwrap_or_else(|err| {
        eprintln!("failed to get the chat history: {err}");
        exit(1)
    });

    let matches = cstracker::parse_match_history(&chat_history).unwrap_or_else(|err| {
        eprintln!("failed to parse chat history: {err}");
        exit(1)
    });

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
