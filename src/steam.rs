use std::str::FromStr;

const STEAM_UNIVERSE: &str = "STEAM_";
const STEAM_BASE_ID: u64 = 76561197960265728;

pub struct SteamId(u64);

impl SteamId {
    fn from_32(s: &str) -> Option<Self> {
        let parts: Vec<&str> = s.split(":").collect();
        if parts.len() != 3 {
            return None;
        }

        let bit = parts[1].parse::<u8>().ok()?;
        let id = parts[2].parse::<u64>().ok()?;

        Some(SteamId(STEAM_BASE_ID + id * 2 + bit as u64))
    }

    fn from_64(s: &str) -> Option<Self> {
        Some(SteamId(s.parse().ok()?))
    }
}

impl FromStr for SteamId {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.len() < STEAM_UNIVERSE.len() {
            return Err("too short".to_string());
        };

        if !s[..STEAM_UNIVERSE.len()].eq_ignore_ascii_case(STEAM_UNIVERSE) {
            return match Self::from_64(s) {
                Some(v) => Ok(v),
                None => Err("doesn't look like SteamID64".to_string()),
            };
        }

        match Self::from_32(s) {
            Some(v) => Ok(v),
            None => Err("doesn't look like SteamID32".to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_parsing_from_steamid32() {
        assert_eq!(
            "STEAM_0:1:3645504".parse::<SteamId>().unwrap().0,
            76561197967556737
        )
    }

    #[test]
    fn test_simple_insensitive_parsing_from_steamid64() {
        assert_eq!(
            "steAm_0:1:3645504".parse::<SteamId>().unwrap().0,
            76561197967556737
        )
    }

    #[test]
    fn test_simple_parsing_from_steamid64() {
        assert_eq!(
            "76561197967556737".parse::<SteamId>().unwrap().0,
            76561197967556737
        )
    }
}
