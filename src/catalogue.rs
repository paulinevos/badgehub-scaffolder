use std::env::var;

use anyhow::{Result, bail};

const API: &str = "https://badgehub.eu/api/v3";
/// Lets a test — or someone running their own BadgeHub — point this elsewhere.
const OVERRIDE: &str = "BADGEHUB_API_URL";
/// badgehub.eu sits behind Cloudflare, which answers 403 to a default agent.
const AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));

/// Baked in as the offline fallback only. These enums are configured per
/// BadgeHub instance (`packages/shared/src/config/sharedConfig.ts`), so the
/// live lists win whenever they can be reached.
const KNOWN_BADGES: &[&str] = &[
    "mpos_api_0",
    "mch2022",
    "tanmatsu",
    "cz20",
    "brucon_0x10",
    "fri3d_2026",
    "fri3d_2024",
    "fri3d_2022",
];
const KNOWN_CATEGORIES: &[&str] = &[
    "Audio",
    "Communication",
    "Data",
    "Development",
    "Driver",
    "Event-related",
    "Finance",
    "Game",
    "Graphics",
    "Hacking",
    "Hardware",
    "Interpreter",
    "Knowledge",
    "Network",
    "SAO",
    "Silly",
    "System",
    "Troll",
    "Uncategorised",
    "Utility",
    "Virus",
    "Wearable",
];

pub struct Choices {
    label: &'static str,
    values: Vec<String>,
}

impl Choices {
    fn fetched_or_known(label: &'static str, path: &str, known: &[&str]) -> Self {
        let values = fetch(path).unwrap_or_else(|_| known.iter().map(|&v| v.to_owned()).collect());
        Self { label, values }
    }

    pub fn options(&self) -> Vec<String> {
        self.values.clone()
    }

    pub fn accept(&self, chosen: Vec<String>) -> Result<Vec<String>> {
        let unknown: Vec<&String> = chosen
            .iter()
            .filter(|value| !self.values.contains(value))
            .collect();
        if !unknown.is_empty() {
            bail!(
                "unknown {}: {:?}. Known values: {}",
                self.label,
                unknown,
                self.values.join(", ")
            );
        }
        Ok(chosen)
    }
}

fn api() -> String {
    var(OVERRIDE)
        .unwrap_or_else(|_| API.to_owned())
        .trim_end_matches('/')
        .to_owned()
}

fn fetch(path: &str) -> Result<Vec<String>> {
    let body = ureq::get(format!("{}/{path}", api()))
        .header("User-Agent", AGENT)
        .call()?
        .body_mut()
        .read_to_string()?;
    Ok(serde_json::from_str(&body)?)
}

pub struct Catalogue {
    pub badges: Choices,
    pub categories: Choices,
}

impl Catalogue {
    pub fn load() -> Self {
        Self {
            badges: Choices::fetched_or_known("badges", "badges", KNOWN_BADGES),
            categories: Choices::fetched_or_known("categories", "categories", KNOWN_CATEGORIES),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Choices, KNOWN_BADGES};

    fn known_badges() -> Choices {
        Choices {
            label: "badges",
            values: KNOWN_BADGES.iter().map(|&v| v.to_owned()).collect(),
        }
    }

    #[test]
    fn accepts_a_known_value() {
        assert!(known_badges().accept(vec!["mch2022".to_owned()]).is_ok());
    }

    #[test]
    fn rejects_an_unknown_value() {
        assert!(known_badges().accept(vec!["why2025".to_owned()]).is_err());
    }
}
