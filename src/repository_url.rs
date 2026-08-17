use std::fmt::{Display, Formatter, Result as FormatResult};
use std::sync::LazyLock;

use anyhow::{Result, bail};
use regex::Regex;

/// Both forms git itself accepts as a remote: `https://host/owner/repo` and
/// `git@host:owner/repo`. Anything else would be recorded in metadata.json and
/// handed to `git remote add`, where it fails far from where it was typed.
static PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(https://[^\s/@]+/[^\s]+|git@[^\s:/]+:[^\s]+)$").unwrap());

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RepositoryUrl(String);

impl RepositoryUrl {
    pub fn parse(candidate: &str) -> Result<Self> {
        let trimmed = candidate.trim().trim_end_matches('/');
        if !PATTERN.is_match(trimmed) {
            bail!(
                "'{candidate}' is not a repository URL: expected https://host/owner/repo \
                 or git@host:owner/repo"
            );
        }
        Ok(Self(trimmed.to_owned()))
    }
}

impl Display for RepositoryUrl {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> FormatResult {
        write!(formatter, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::RepositoryUrl;

    #[test]
    fn accepts_an_https_url() {
        assert!(RepositoryUrl::parse("https://github.com/fri3d/agenda").is_ok());
    }

    #[test]
    fn accepts_an_ssh_url() {
        assert!(RepositoryUrl::parse("git@github.com:fri3d/agenda.git").is_ok());
    }

    #[test]
    fn drops_a_trailing_slash() {
        let url = RepositoryUrl::parse("https://github.com/fri3d/agenda/").unwrap();
        assert_eq!("https://github.com/fri3d/agenda", url.to_string());
    }

    #[test]
    fn rejects_a_bare_host() {
        assert!(RepositoryUrl::parse("https://github.com").is_err());
    }

    #[test]
    fn rejects_a_sentence() {
        assert!(RepositoryUrl::parse("I will add one later").is_err());
    }
}
