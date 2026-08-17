use std::env::{VarError, var};
use std::fmt::{Debug, Display, Formatter, Result as FormatResult};
use std::fs::{OpenOptions, create_dir_all, read_to_string, rename};
use std::io::{ErrorKind, Write};
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

use crate::wizard::Wizard;

const DIRECTORY: &str = "badgehub";
const FILE: &str = "config.json";
/// Read before the config file so a shell already carrying a token — CI, a
/// `gh auth` export — wins over whatever was saved months ago.
const ENVIRONMENT_NAMES: [&str; 2] = ["GITHUB_TOKEN", "GH_TOKEN"];

pub struct ConfigPath(PathBuf);

impl ConfigPath {
    pub fn of_this_user() -> Result<Self> {
        Ok(Self::at(config_home()?.join(DIRECTORY).join(FILE)))
    }

    pub fn at(path: PathBuf) -> Self {
        Self(path)
    }

    fn read(&self) -> Result<Option<String>> {
        match read_to_string(&self.0) {
            Ok(contents) => Ok(Some(contents)),
            Err(problem) if problem.kind() == ErrorKind::NotFound => Ok(None),
            Err(problem) => Err(problem).with_context(|| format!("reading {self}")),
        }
    }

    /// Written to a scratch file and renamed so an interrupted write leaves the
    /// previous config intact rather than a truncated one.
    fn write(&self, contents: &str) -> Result<()> {
        create_dir_all(self.directory()).with_context(|| format!("creating {self}"))?;
        let scratch = self.0.with_extension("json.tmp");
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(&scratch)
            .with_context(|| format!("writing {}", scratch.display()))?;
        file.write_all(contents.as_bytes())
            .with_context(|| format!("writing {}", scratch.display()))?;
        rename(&scratch, &self.0).with_context(|| format!("saving {self}"))
    }

    fn directory(&self) -> &Path {
        self.0.parent().unwrap_or(Path::new("."))
    }
}

impl Display for ConfigPath {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> FormatResult {
        write!(formatter, "{}", self.0.display())
    }
}

fn config_home() -> Result<PathBuf> {
    if let Some(configured) = spoken(var("XDG_CONFIG_HOME")) {
        return Ok(PathBuf::from(configured));
    }
    let home = spoken(var("HOME")).context("no HOME set: nowhere to keep a config file")?;
    Ok(PathBuf::from(home).join(".config"))
}

fn spoken(value: Result<String, VarError>) -> Option<String> {
    value.ok().filter(|value| !value.trim().is_empty())
}

/// Never `Display`, and `Debug` shows nothing: a token that can create
/// repositories should not be one panic message away from a log file.
#[derive(Clone, Deserialize, Serialize)]
pub struct GithubToken(String);

impl GithubToken {
    pub fn parse(candidate: &str) -> Result<Self> {
        let trimmed = candidate.trim();
        if trimmed.is_empty() {
            bail!("a GitHub token cannot be blank");
        }
        Ok(Self(trimmed.to_owned()))
    }

    pub fn authorisation(&self) -> String {
        format!("Bearer {}", self.0)
    }
}

impl Debug for GithubToken {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> FormatResult {
        write!(formatter, "GithubToken(hidden)")
    }
}

/// What this user wants every scaffold to start from, so the wizard stops
/// asking the same three questions.
#[derive(Default, Deserialize, Serialize)]
pub struct Settings {
    #[serde(skip_serializing_if = "Option::is_none")]
    author: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    license: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    github_token: Option<GithubToken>,
}

impl Settings {
    pub fn of_this_user() -> Result<Self> {
        Self::read_from(&ConfigPath::of_this_user()?)
    }

    /// A missing file is how most people run this; only an unreadable or
    /// malformed one is worth stopping for, since silently ignoring a typo
    /// would look like the defaults were never saved.
    pub fn read_from(path: &ConfigPath) -> Result<Self> {
        let Some(contents) = path.read()? else {
            return Ok(Self::default());
        };
        serde_json::from_str(&contents).with_context(|| format!("reading {path}"))
    }

    pub fn write_to(&self, path: &ConfigPath) -> Result<()> {
        path.write(&format!("{}\n", serde_json::to_string_pretty(self)?))
    }

    pub fn defaulting_author(&self, given: Option<String>) -> Option<String> {
        given.or_else(|| self.author.clone())
    }

    pub fn defaulting_license(&self, given: Option<String>) -> Option<String> {
        given.or_else(|| self.license.clone())
    }

    pub fn available_token(&self) -> Option<GithubToken> {
        token_from_environment().or_else(|| self.github_token.clone())
    }

    pub fn remembering(&self, token: &GithubToken) -> Self {
        Self {
            author: self.author.clone(),
            license: self.license.clone(),
            github_token: Some(token.clone()),
        }
    }

    pub fn amended_by(self, wizard: &Wizard) -> Result<Self> {
        Ok(Self {
            author: wizard.optional_text_defaulting_to("Default author", self.author)?,
            license: wizard
                .optional_text_defaulting_to("Default licence, e.g. MIT", self.license)?,
            github_token: amended_token(wizard, self.github_token)?,
        })
    }
}

fn amended_token(wizard: &Wizard, current: Option<GithubToken>) -> Result<Option<GithubToken>> {
    let Some(typed) = wizard.secret("GitHub token")? else {
        return Ok(current);
    };
    Ok(Some(GithubToken::parse(&typed)?))
}

fn token_from_environment() -> Option<GithubToken> {
    ENVIRONMENT_NAMES
        .iter()
        .find_map(|name| GithubToken::parse(&spoken(var(name))?).ok())
}

#[cfg(test)]
mod tests {
    use std::env::{temp_dir, var};
    use std::fs::create_dir_all;

    use super::{ConfigPath, GithubToken, Settings};

    fn scratch(name: &str) -> ConfigPath {
        let directory = temp_dir().join("badgehub-settings-tests").join(name);
        create_dir_all(&directory).unwrap();
        ConfigPath::at(directory.join("config.json"))
    }

    #[test]
    fn a_missing_file_leaves_every_default_unset() {
        let settings = Settings::read_from(&scratch("missing")).unwrap();
        assert_eq!(None, settings.defaulting_author(None));
    }

    #[test]
    fn malformed_json_is_an_error_rather_than_silence() {
        let path = scratch("malformed");
        path.write("{ not json").unwrap();
        assert!(Settings::read_from(&path).is_err());
    }

    #[test]
    fn a_saved_author_fills_in_an_unanswered_one() {
        let path = scratch("author");
        path.write(r#"{"author": "Pauline"}"#).unwrap();
        let settings = Settings::read_from(&path).unwrap();
        assert_eq!(Some("Pauline".to_owned()), settings.defaulting_author(None));
    }

    #[test]
    fn a_flag_beats_the_saved_author() {
        let path = scratch("flag");
        path.write(r#"{"author": "Pauline"}"#).unwrap();
        let settings = Settings::read_from(&path).unwrap();
        assert_eq!(
            Some("Someone Else".to_owned()),
            settings.defaulting_author(Some("Someone Else".to_owned()))
        );
    }

    #[test]
    fn a_saved_token_survives_a_round_trip_without_being_printed() {
        let path = scratch("token");
        let saved = Settings::default().remembering(&GithubToken::parse("ghp_secret").unwrap());
        saved.write_to(&path).unwrap();
        let token = Settings::read_from(&path)
            .unwrap()
            .available_token()
            .unwrap();
        // The environment wins over the file, so on a machine that already
        // exports one this asserts the precedence rather than the saved value.
        let expected = var("GITHUB_TOKEN")
            .or_else(|_| var("GH_TOKEN"))
            .unwrap_or("ghp_secret".to_owned());
        assert_eq!(format!("Bearer {expected}"), token.authorisation());
        assert_eq!("GithubToken(hidden)", format!("{token:?}"));
    }

    #[test]
    fn a_blank_token_is_not_a_token() {
        assert!(GithubToken::parse("   ").is_err());
    }

    #[test]
    fn the_file_is_readable_only_by_its_owner() {
        use std::os::unix::fs::PermissionsExt;

        let path = scratch("permissions");
        Settings::default().write_to(&path).unwrap();
        let directory = temp_dir().join("badgehub-settings-tests/permissions/config.json");
        let mode = std::fs::metadata(directory).unwrap().permissions().mode();
        assert_eq!(0o600, mode & 0o777);
    }

    #[test]
    fn writing_leaves_no_scratch_file_behind() {
        let path = scratch("scratch");
        Settings::default().write_to(&path).unwrap();
        let leftover = temp_dir().join("badgehub-settings-tests/scratch/config.json.tmp");
        assert!(!leftover.exists());
    }
}
