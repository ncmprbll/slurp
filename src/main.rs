mod cstracker;
mod steam;

use clap::Parser;
use std::process::exit;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Target SteamID
    #[arg(short, long, required = true)]
    target: String,

    /// User agent for a web request
    #[arg(short, long)]
    user_agent: Option<String>,
}

fn main() {
    let args = Args::parse();

    let steam_id = args.target;
    let steam_id = match steam_id.parse::<steam::SteamId>() {
        Ok(v) => v,
        Err(err) => {
            eprintln!("{err}");
            exit(1)
        }
    };

    let chat_history = cstracker::get_match_history(steam_id.0, args.user_agent.as_deref())
        .unwrap_or_else(|err| {
            eprintln!("failed to get the chat history: {err}");
            exit(1)
        });

    let matches = cstracker::parse_match_history(&chat_history).unwrap_or_else(|err| {
        eprintln!("failed to parse chat history: {err}");
        exit(1)
    });

    if matches.len() == 0 {
        eprintln!("no matches found");
        exit(1)
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
