use std::env::current_dir;
use std::path::PathBuf;

use anyhow::Result;
use clap::{Args, Parser, Subcommand};

use crate::answers::{NewProject, ProjectType, Text};
use crate::catalogue::{Catalogue, Choices};
use crate::existing_project::ExistingProject;
use crate::release_action::what_is_left_to_do;
use crate::repository::RepositoryOptions;
use crate::scaffold::Scaffold;
use crate::set::{self, Changes};
use crate::settings::{ConfigPath, Settings};
use crate::slug::Slug;
use crate::version::SemanticVersion;
use crate::wizard::Wizard;

/// Scaffold a BadgeHub app: metadata.json, MANIFEST.JSON and the directory
/// layout the store expects.
#[derive(Parser)]
#[command(name = "bh", version, about)]
pub struct Cli {
    #[command(subcommand)]
    command: Command,
}

// Boxing the flags to even the variants out would buy nothing: exactly one of
// these is built, once, at startup.
#[allow(clippy::large_enum_variant)]
#[derive(Subcommand)]
enum Command {
    /// Scaffold a new BadgeHub project
    New(NewOptions),
    /// Change the name, author, description or licence of a project
    Set(SetOptions),
    /// Add the BadgeHub release workflow to an existing project
    ReleaseAction(ReleaseActionOptions),
    /// Set the defaults every new project starts from
    Config,
}

impl Cli {
    pub fn run(self, wizard: &Wizard) -> Result<()> {
        match self.command {
            Command::New(options) => scaffold(options, wizard),
            Command::Set(options) => amend(options, wizard),
            Command::ReleaseAction(options) => add_release_workflow(options),
            Command::Config => configure(wizard),
        }
    }
}

#[derive(Args)]
pub struct SetOptions {
    /// The project to change; defaults to looking in the current directory
    #[arg(long)]
    app_directory: Option<PathBuf>,
    #[command(flatten)]
    changes: Changes,
}

#[derive(Args)]
pub struct ReleaseActionOptions {
    /// The project to add the workflow to; defaults to the current directory
    #[arg(long)]
    app_directory: Option<PathBuf>,
    /// Replace an existing workflow
    #[arg(long)]
    force: bool,
}

#[derive(Args)]
pub struct NewOptions {
    /// Project slug, e.g. org.fri3d.hwtest
    #[arg(long)]
    slug: Option<String>,
    /// Display name shown in the launcher and on BadgeHub
    #[arg(long)]
    name: Option<String>,
    /// One-line description
    #[arg(long)]
    description: Option<String>,
    /// Author name
    #[arg(long)]
    author: Option<String>,
    /// Semantic version, e.g. 0.1.0
    #[arg(long, default_value = "0.1.0")]
    app_version: String,
    /// One of: app, library, firmware, other
    #[arg(long)]
    project_type: Option<String>,
    /// Category; repeat for several
    #[arg(long = "category")]
    categories: Vec<String>,
    /// Badge slug; repeat for several
    #[arg(long = "badge")]
    badges: Vec<String>,
    /// Licence identifier, e.g. MIT
    #[arg(long)]
    license: Option<String>,
    #[command(flatten)]
    repository: RepositoryOptions,
    /// Add the BadgeHub release workflow without being asked
    #[arg(long, conflicts_with = "no_release_action")]
    release_action: bool,
    /// Skip the release workflow without being asked
    #[arg(long)]
    no_release_action: bool,
}

impl NewOptions {
    pub fn answer(
        self,
        wizard: &Wizard,
        catalogue: &Catalogue,
        settings: &Settings,
    ) -> Result<NewProject> {
        let NewOptions {
            slug,
            name,
            description,
            author,
            app_version,
            project_type,
            categories,
            badges,
            license,
            repository,
            release_action,
            no_release_action,
        } = self;
        // Settled up front, in the order they are asked: the repository
        // question is put in terms of the slug and the description.
        let slug = Slug::parse(&wizard.text("Project slug", "--slug", slug)?)?;
        let name = Text::parse(&wizard.text("Display name", "--name", name)?)?;
        let description =
            Text::parse(&wizard.text("Description", "--description", description)?)?;
        Ok(NewProject {
            author: Text::parse(&wizard.text(
                "Author",
                "--author",
                settings.defaulting_author(author),
            )?)?,
            version: SemanticVersion::parse(&app_version)?,
            project_type: chosen_project_type(wizard, project_type)?,
            categories: chosen_from(
                wizard,
                &catalogue.categories,
                "Categories",
                "--category",
                categories,
            )?,
            badges: chosen_from(
                wizard,
                &catalogue.badges,
                "Badges this app runs on",
                "--badge",
                badges,
            )?,
            license_type: wizard
                .optional_text("Licence, e.g. MIT", settings.defaulting_license(license))?,
            git_url: repository.settled(wizard, settings, &slug, &description)?,
            release_workflow: wanted_release_workflow(wizard, release_action, no_release_action)?,
            slug,
            name,
            description,
        })
    }
}

fn scaffold(options: NewOptions, wizard: &Wizard) -> Result<()> {
    let settings = Settings::of_this_user()?;
    let project = options.answer(wizard, &Catalogue::load(), &settings)?;
    let scaffold = Scaffold::beside(&current_dir()?, &project)?;
    let root = scaffold.write_out(&project)?;
    println!("Scaffolded {}", root.display());
    Ok(())
}

fn amend(options: SetOptions, wizard: &Wizard) -> Result<()> {
    let project = found(options.app_directory)?;
    set::apply(options.changes, &project, wizard)
}

fn add_release_workflow(options: ReleaseActionOptions) -> Result<()> {
    let written = found(options.app_directory)?.add_release_workflow(options.force)?;
    println!("Wrote {}", written.display());
    println!("{}", what_is_left_to_do());
    Ok(())
}

fn found(given: Option<PathBuf>) -> Result<ExistingProject> {
    let directory = match given {
        Some(directory) => directory,
        None => current_dir()?,
    };
    ExistingProject::found_at(&directory)
}

fn configure(wizard: &Wizard) -> Result<()> {
    let path = ConfigPath::of_this_user()?;
    Settings::read_from(&path)?
        .amended_by(wizard)?
        .write_to(&path)?;
    println!("Saved {path}");
    Ok(())
}

/// Either flag settles it. Otherwise it is a question, and with no terminal to
/// ask on the answer is no — the same default the repository question takes.
fn wanted_release_workflow(wizard: &Wizard, wanted: bool, refused: bool) -> Result<bool> {
    if wanted || refused {
        return Ok(wanted);
    }
    wizard.confirm("Add the BadgeHub release workflow?")
}

fn chosen_project_type(wizard: &Wizard, given: Option<String>) -> Result<ProjectType> {
    let parsed = given.as_deref().map(ProjectType::parse).transpose()?;
    wizard.choose(
        "Project type",
        "--project-type",
        ProjectType::ALL.to_vec(),
        parsed,
    )
}

fn chosen_from(
    wizard: &Wizard,
    choices: &Choices,
    label: &str,
    flag: &str,
    given: Vec<String>,
) -> Result<Vec<String>> {
    let chosen = wizard.choose_several(label, flag, choices.options(), choices.accept(given)?)?;
    choices.accept(chosen)
}
