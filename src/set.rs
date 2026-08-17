use anyhow::{Result, bail};
use clap::Args;

use crate::answers::Text;
use crate::existing_project::ExistingProject;
use crate::json_document::JsonDocument;
use crate::manifest::Manifest;
use crate::metadata::Metadata;
use crate::wizard::Wizard;

/// What a scaffolded project can still change. The slug is not here: it is the
/// directory name, the manifest's fullname, the BadgeHub project identity and
/// the release action's app-directory all at once. Neither is the version —
/// the release action writes that from the git tag — nor the licence, which is
/// a file to edit as much as a field to set.
#[derive(Args, Default)]
pub struct Changes {
    /// Display name shown in the launcher and on BadgeHub
    #[arg(long)]
    pub name: Option<String>,
    /// Author name
    #[arg(long)]
    pub author: Option<String>,
    /// One-line description
    #[arg(long)]
    pub description: Option<String>,
}

impl Changes {
    pub fn nothing_given(&self) -> bool {
        self.name.is_none() && self.author.is_none() && self.description.is_none()
    }

    /// Flags win; a bare `bh set` walks the same fields with what is already
    /// there prefilled, the way `bh config` does.
    pub fn or_asked(self, wizard: &Wizard, present: &JsonDocument) -> Result<Self> {
        if !self.nothing_given() {
            return Ok(self);
        }
        wizard.refuse_when_silent("--name, --author or --description")?;
        Ok(Self {
            name: wizard.optional_text_defaulting_to("Display name", present.text_at("name"))?,
            author: wizard.optional_text_defaulting_to("Author", present.text_at("author"))?,
            description: wizard
                .optional_text_defaulting_to("Description", present.text_at("description"))?,
        })
    }

    /// Checked before anything is written, so a blank answer cannot leave one
    /// file changed and the other not.
    fn checked(&self) -> Result<()> {
        for (label, given) in [
            ("name", &self.name),
            ("author", &self.author),
            ("description", &self.description),
        ] {
            let Some(value) = given else { continue };
            Text::parse(value).map_err(|problem| problem.context(format!("the {label}")))?;
        }
        Ok(())
    }
}

pub fn apply(changes: Changes, project: &ExistingProject, wizard: &Wizard) -> Result<()> {
    let mut metadata = project.metadata()?;
    let changes = changes.or_asked(wizard, &metadata)?;
    if changes.nothing_given() {
        bail!("nothing to change");
    }
    changes.checked()?;
    Metadata::amend(&mut metadata, &changes);
    metadata.write_out()?;
    amend_manifest(project, &changes)
}

fn amend_manifest(project: &ExistingProject, changes: &Changes) -> Result<()> {
    let mut manifest = project.manifest()?;
    Manifest::amend(&mut manifest, changes);
    manifest.write_out()
}
