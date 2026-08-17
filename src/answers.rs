use std::fmt::{Display, Formatter, Result as FormatResult};

use anyhow::{Result, bail};

use crate::repository_url::RepositoryUrl;
use crate::slug::Slug;
use crate::version::SemanticVersion;

/// One wrapper serves name, author and description alike: they are all "a
/// required free-form line the user typed", and the only rule any of them
/// carries is that a blank answer is not an answer.
#[derive(Clone, Debug)]
pub struct Text(String);

impl Text {
    pub fn parse(candidate: &str) -> Result<Self> {
        let trimmed = candidate.trim();
        if trimmed.is_empty() {
            bail!("this cannot be blank");
        }
        Ok(Self(trimmed.to_owned()))
    }
}

impl Display for Text {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> FormatResult {
        write!(formatter, "{}", self.0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProjectType {
    App,
    Library,
    Firmware,
    Other,
}

impl ProjectType {
    pub const ALL: [ProjectType; 4] = [
        ProjectType::App,
        ProjectType::Library,
        ProjectType::Firmware,
        ProjectType::Other,
    ];

    pub fn parse(candidate: &str) -> Result<Self> {
        match candidate {
            "app" => Ok(ProjectType::App),
            "library" => Ok(ProjectType::Library),
            "firmware" => Ok(ProjectType::Firmware),
            "other" => Ok(ProjectType::Other),
            _ => bail!("'{candidate}' is not one of: app, library, firmware, other"),
        }
    }
}

impl Display for ProjectType {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> FormatResult {
        let name = match self {
            ProjectType::App => "app",
            ProjectType::Library => "library",
            ProjectType::Firmware => "firmware",
            ProjectType::Other => "other",
        };
        write!(formatter, "{name}")
    }
}

/// Everything the wizard and the flags together established, settled once and
/// then only read.
pub struct NewProject {
    pub slug: Slug,
    pub name: Text,
    pub description: Text,
    pub author: Text,
    pub version: SemanticVersion,
    pub project_type: ProjectType,
    pub categories: Vec<String>,
    pub badges: Vec<String>,
    pub license_type: Option<String>,
    pub git_url: Option<RepositoryUrl>,
    pub release_workflow: bool,
}

impl NewProject {
    pub fn entrypoint() -> &'static str {
        "__init__.py"
    }

    pub fn readme(&self) -> String {
        format!(
            "# {name}\n\n{description}\n\nA BadgeHub app.\n\n\
             ## Layout\n\n\
             Everything published to BadgeHub lives in `{slug}/`. \
             `metadata.json` is the BadgeHub store listing; `MANIFEST.JSON` is \
             the MicroPythonOS launcher manifest. The `icon-*.png` files are \
             placeholders generated from the slug — replace them before you \
             publish.\n\n\
             ## Publishing\n\n\
             Create the project and generate an API token at \
             <https://badgehub.eu>, then upload the contents of `{slug}/` to \
             the draft revision and publish it.\n",
            name = self.name,
            description = self.description,
            slug = self.slug,
        )
    }
}
