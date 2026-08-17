use std::fmt::{Display, Formatter, Result as FormatResult};

use anyhow::Result;
use clap::Args;

use crate::answers::Text;
use crate::github::{GithubApi, NewRepository, Visibility};
use crate::repository_url::RepositoryUrl;
use crate::settings::{ConfigPath, GithubToken, Settings};
use crate::slug::Slug;
use crate::wizard::Wizard;

const HOW_TO_GET_A_TOKEN: &str = "\nCreating a repository needs a GitHub token.\n\
     Make a fine-grained personal access token at \
     <https://github.com/settings/personal-access-tokens/new>, give it \
     Administration: read and write on your account, and paste it below.\n";

/// The three ways a project can end up with a repository, whether picked from
/// a menu or spelled out in flags.
#[derive(Args)]
pub struct RepositoryOptions {
    /// URL of a repository that already exists
    #[arg(long, conflicts_with = "create_repo")]
    git_url: Option<String>,
    /// Create the repository on GitHub instead of naming an existing one
    #[arg(long)]
    create_repo: bool,
    /// Name for the created repository; defaults to the project slug
    #[arg(long, requires = "create_repo")]
    repo_name: Option<String>,
    /// Whether the created repository is public or private
    #[arg(long, requires = "create_repo", value_enum, default_value_t = Visibility::Public)]
    visibility: Visibility,
}

impl RepositoryOptions {
    /// A flag answers the question outright; without one, and without a
    /// terminal to ask on, no repository is the answer this tool has always
    /// given.
    pub fn settled(
        self,
        wizard: &Wizard,
        settings: &Settings,
        slug: &Slug,
        description: &Text,
    ) -> Result<Option<RepositoryUrl>> {
        if let Some(url) = &self.git_url {
            return Ok(Some(RepositoryUrl::parse(url)?));
        }
        if self.create_repo {
            return self.create(wizard, settings, slug, description).map(Some);
        }
        self.ask(wizard, settings, slug, description)
    }

    fn ask(
        self,
        wizard: &Wizard,
        settings: &Settings,
        slug: &Slug,
        description: &Text,
    ) -> Result<Option<RepositoryUrl>> {
        let Some(source) = wizard.choose_optional("Repository", RepositorySource::ALL.to_vec())?
        else {
            return Ok(None);
        };
        match source {
            RepositorySource::None => Ok(None),
            RepositorySource::ExistingUrl => typed_url(wizard),
            RepositorySource::NewOnGithub => {
                self.create(wizard, settings, slug, description).map(Some)
            }
        }
    }

    fn create(
        &self,
        wizard: &Wizard,
        settings: &Settings,
        slug: &Slug,
        description: &Text,
    ) -> Result<RepositoryUrl> {
        let token = self.token(wizard, settings)?;
        let name = self.name(wizard, slug)?;
        let repository = NewRepository::named(&name, &description.to_string(), self.visibility);
        let url = GithubApi::wherever_configured().create(&repository, &token)?;
        println!("Created {url}");
        Ok(url)
    }

    fn name(&self, wizard: &Wizard, slug: &Slug) -> Result<String> {
        if let Some(name) = &self.repo_name {
            return Ok(name.clone());
        }
        wizard.text_defaulting_to("Repository name", &slug.to_string())
    }

    fn token(&self, wizard: &Wizard, settings: &Settings) -> Result<GithubToken> {
        if let Some(token) = settings.available_token() {
            return Ok(token);
        }
        println!("{HOW_TO_GET_A_TOKEN}");
        let typed = wizard.secret_or_refuse(
            "GitHub token",
            "no GitHub token: set GITHUB_TOKEN or run `bh config`",
        )?;
        let token = GithubToken::parse(&typed)?;
        remember(wizard, settings, &token)?;
        Ok(token)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RepositorySource {
    NewOnGithub,
    ExistingUrl,
    None,
}

impl RepositorySource {
    const ALL: [RepositorySource; 3] = [
        RepositorySource::NewOnGithub,
        RepositorySource::ExistingUrl,
        RepositorySource::None,
    ];
}

impl Display for RepositorySource {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> FormatResult {
        let name = match self {
            RepositorySource::NewOnGithub => "create a new GitHub repository",
            RepositorySource::ExistingUrl => "use an existing repository URL",
            RepositorySource::None => "no repository for now",
        };
        write!(formatter, "{name}")
    }
}

fn typed_url(wizard: &Wizard) -> Result<Option<RepositoryUrl>> {
    let Some(typed) = wizard.optional_text("Repository URL", None)? else {
        return Ok(None);
    };
    Ok(Some(RepositoryUrl::parse(&typed)?))
}

/// Asked rather than assumed: writing someone's credential to disk without
/// saying so is not a favour.
fn remember(wizard: &Wizard, settings: &Settings, token: &GithubToken) -> Result<()> {
    if !wizard.confirm("Save this token for next time?")? {
        return Ok(());
    }
    let path = ConfigPath::of_this_user()?;
    settings.remembering(token).write_to(&path)?;
    println!("Saved {path}");
    Ok(())
}
