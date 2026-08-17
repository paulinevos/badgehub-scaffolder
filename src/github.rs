use std::env::var;
use std::fmt::{Display, Formatter, Result as FormatResult};

use anyhow::{Context, Error, Result, bail};
use clap::ValueEnum;
use serde::Serialize;
use serde_json::Value;

use crate::repository_url::RepositoryUrl;
use crate::settings::GithubToken;

const PUBLIC_API: &str = "https://api.github.com";
/// GitHub's own name for this, exported by Actions and pointed at the appliance
/// on GitHub Enterprise — so honouring it serves Enterprise users and the tests
/// with one mechanism.
const OVERRIDE: &str = "GITHUB_API_URL";
const AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum Visibility {
    Public,
    Private,
}

impl Visibility {
    pub const ALL: [Visibility; 2] = [Visibility::Public, Visibility::Private];

    fn is_private(self) -> bool {
        self == Visibility::Private
    }
}

impl Display for Visibility {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> FormatResult {
        let name = match self {
            Visibility::Public => "public",
            Visibility::Private => "private",
        };
        write!(formatter, "{name}")
    }
}

#[derive(Serialize)]
pub struct NewRepository {
    name: String,
    description: String,
    private: bool,
}

impl NewRepository {
    pub fn named(name: &str, description: &str, visibility: Visibility) -> Self {
        Self {
            name: name.to_owned(),
            description: description.to_owned(),
            private: visibility.is_private(),
        }
    }
}

/// Where GitHub lives, so a test or an Enterprise install can put it somewhere
/// else without every caller knowing.
pub struct GithubApi(String);

impl GithubApi {
    pub fn wherever_configured() -> Self {
        Self::at(&var(OVERRIDE).unwrap_or_else(|_| PUBLIC_API.to_owned()))
    }

    pub fn at(base: &str) -> Self {
        Self(base.trim_end_matches('/').to_owned())
    }

    /// Unlike the catalogue, this never falls back quietly: a swallowed failure
    /// here would leave the user believing a repository exists.
    pub fn create(&self, repository: &NewRepository, token: &GithubToken) -> Result<RepositoryUrl> {
        // Serialised here rather than through ureq's `json` feature, which
        // would pull a cookie store and a URL parser in for one POST.
        let answer = ureq::post(format!("{}/user/repos", self.0))
            .header("Authorization", token.authorisation())
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .header("Content-Type", "application/json")
            .header("User-Agent", AGENT)
            .send(serde_json::to_string(repository)?);
        RepositoryUrl::parse(&clone_url(&body_of(answer)?)?)
    }
}

fn body_of(answer: Result<ureq::http::Response<ureq::Body>, ureq::Error>) -> Result<String> {
    let mut response = answer.map_err(explained)?;
    response
        .body_mut()
        .read_to_string()
        .context("reading GitHub's answer")
}

fn explained(problem: ureq::Error) -> Error {
    match problem {
        ureq::Error::StatusCode(401) => Error::msg(
            "GitHub rejected the token. Check it has not expired, then re-run `bh config`.",
        ),
        ureq::Error::StatusCode(403) => Error::msg(
            "GitHub refused: the token is not allowed to create repositories. \
             It needs the Administration: read and write permission.",
        ),
        ureq::Error::StatusCode(422) => {
            Error::msg("GitHub already has a repository by that name on this account.")
        }
        other => Error::new(other).context("asking GitHub to create the repository"),
    }
}

fn clone_url(body: &str) -> Result<String> {
    let answer: Value = serde_json::from_str(body).context("reading GitHub's answer")?;
    let Some(url) = answer["clone_url"].as_str() else {
        bail!("GitHub created something, but its answer carried no clone_url");
    };
    Ok(url.to_owned())
}
