use std::fmt::{Display, Formatter, Result as FormatResult};

use anyhow::{Result, bail};

/// BadgeHub stores `version` as a semver *string*; the integer revision number
/// is assigned server-side and is not this.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SemanticVersion(String);

impl SemanticVersion {
    pub fn parse(candidate: &str) -> Result<Self> {
        let parts: Vec<&str> = candidate.split('.').collect();
        if parts.len() != 3 {
            bail!("'{candidate}' is not a semantic version: expected major.minor.patch");
        }
        if parts.iter().any(|part| !is_number(part)) {
            bail!("'{candidate}' is not a semantic version: every part must be a number");
        }
        Ok(Self(candidate.to_owned()))
    }
}

fn is_number(part: &str) -> bool {
    !part.is_empty() && part.chars().all(|character| character.is_ascii_digit())
}

impl Display for SemanticVersion {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> FormatResult {
        write!(formatter, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::SemanticVersion;

    #[test]
    fn accepts_three_numeric_parts() {
        assert!(SemanticVersion::parse("0.1.11").is_ok());
    }

    #[test]
    fn rejects_two_parts() {
        assert!(SemanticVersion::parse("1.0").is_err());
    }

    #[test]
    fn rejects_non_numeric_parts() {
        assert!(SemanticVersion::parse("1.0.x").is_err());
    }

    #[test]
    fn rejects_empty_parts() {
        assert!(SemanticVersion::parse("1..0").is_err());
    }
}
