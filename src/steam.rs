use std::str::FromStr;

const STEAM_UNIVERSE: &str = "STEAM_";
const STEAM_BASE_ID: u64 = 76561197960265728;

enum SteamId {
    Format32(u32),
    Format64(u64),
}

impl SteamId {
    pub fn from_32(s: &str) -> Self {
        todo!();
    }

    pub fn from_64(s: &str) -> Self {
        todo!();
    }
}

impl FromStr for SteamId {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.len() < STEAM_UNIVERSE.len() {
            return Err("too short".to_string());
        };

        if !s[..STEAM_UNIVERSE.len()].eq_ignore_ascii_case(STEAM_UNIVERSE) {}

        todo!()
    }
}
