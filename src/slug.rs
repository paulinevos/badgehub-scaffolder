use std::fmt::{Display, Formatter, Result as FormatResult};
use std::sync::LazyLock;

use anyhow::{Result, bail};
use regex::Regex;

/// Mirrors `packages/shared/src/contracts/slug.ts` in badgehub-app. A slug that
/// fails this is rejected by the API at project-creation time, so it is worth
/// refusing here rather than after the user has a directory tree.
static PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[a-z][.a-z_0-9-]{2,100}$").unwrap());

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Slug(String);

impl Slug {
    pub fn parse(candidate: &str) -> Result<Self> {
        if !PATTERN.is_match(candidate) {
            bail!(
                "'{candidate}' is not a valid BadgeHub slug: start with a lowercase letter, \
                 then 2 to 100 of a-z, 0-9, '.', '_' or '-'"
            );
        }
        Ok(Self(candidate.to_owned()))
    }

    pub fn suggested_class_name(&self) -> String {
        self.0
            .split(['.', '_', '-'])
            .filter(|part| !part.is_empty())
            .map(capitalise)
            .collect()
    }
}

fn capitalise(word: &str) -> String {
    let mut letters = word.chars();
    match letters.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().chain(letters).collect(),
    }
}

impl Display for Slug {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> FormatResult {
        write!(formatter, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::Slug;

    #[test]
    fn accepts_reverse_dns() {
        assert!(Slug::parse("be.lim-it.fri3dagenda").is_ok());
    }

    #[test]
    fn rejects_leading_digit() {
        assert!(Slug::parse("2fast").is_err());
    }

    #[test]
    fn rejects_uppercase() {
        assert!(Slug::parse("Bomberboy").is_err());
    }

    #[test]
    fn rejects_two_characters() {
        assert!(Slug::parse("ab").is_err());
    }

    #[test]
    fn accepts_three_characters() {
        assert!(Slug::parse("abc").is_ok());
    }

    #[test]
    fn derives_a_class_name_from_the_final_segments() {
        let slug = Slug::parse("org.fri3d.hw-test").unwrap();
        assert_eq!("OrgFri3dHwTest", slug.suggested_class_name());
    }
}
