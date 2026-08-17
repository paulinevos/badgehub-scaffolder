use std::fs::read_dir;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

use crate::gitignore::{Added, Gitignore};
use crate::json_document::JsonDocument;
use crate::mpk::Mpk;
use crate::release_action::ReleaseWorkflow;
use crate::slug::Slug;

const METADATA: &str = "metadata.json";
const MANIFEST: &str = "MANIFEST.JSON";

/// A project this tool scaffolded earlier, found again on disk. The layout is
/// the one `Scaffold::beside` writes: a root holding README, LICENSE and git,
/// and one directory inside it named for the slug holding what BadgeHub sees.
#[derive(Debug)]
pub struct ExistingProject {
    root: PathBuf,
    app_directory: PathBuf,
    slug: Slug,
}

impl ExistingProject {
    /// Run from the project root or from inside the app directory itself —
    /// both are places someone reasonably types `bh set`.
    pub fn found_at(directory: &Path) -> Result<Self> {
        let here = directory
            .canonicalize()
            .with_context(|| format!("looking for a project in {}", directory.display()))?;
        if here.join(METADATA).exists() {
            return Self::rooted_at(parent_of(&here)?, &here);
        }
        Self::found_below(&here)
    }

    fn found_below(root: &Path) -> Result<Self> {
        let candidates = app_directories_in(root)?;
        match candidates.as_slice() {
            [] => bail!(
                "no BadgeHub project in {}: expected a {METADATA} here or in a \
                 directory below it",
                root.display()
            ),
            [only] => Self::rooted_at(root.to_owned(), only),
            several => bail!(
                "several projects in {}: {}. Name one with --app-directory",
                root.display(),
                named(several)
            ),
        }
    }

    fn rooted_at(root: PathBuf, app_directory: &Path) -> Result<Self> {
        let name = app_directory
            .file_name()
            .and_then(|name| name.to_str())
            .with_context(|| format!("{} has no usable name", app_directory.display()))?;
        // The directory name is the slug, and MicroPythonOS requires that of
        // the manifest's fullname too, so a name that is not a slug means this
        // is not a project this tool can safely edit.
        let slug = Slug::parse(name)?;
        Ok(Self {
            root,
            app_directory: app_directory.to_owned(),
            slug,
        })
    }

    pub fn metadata(&self) -> Result<JsonDocument> {
        JsonDocument::read(&self.app_directory.join(METADATA))
    }

    pub fn manifest(&self) -> Result<JsonDocument> {
        JsonDocument::read(&self.app_directory.join(MANIFEST))
    }

    /// Built beside the app directory by default, and kept out of the
    /// repository: an .mpk is an artefact of the source next to it.
    pub fn bundle_into(&self, output_directory: Option<PathBuf>) -> Result<(PathBuf, Added)> {
        let mpk = Mpk::of(&self.app_directory, &self.manifest()?)?;
        let directory = output_directory.unwrap_or_else(|| self.root.clone());
        let written = mpk.write_into(&directory)?;
        Ok((written, Gitignore::at(&self.root).ensure("*.mpk")?))
    }

    pub fn add_release_workflow(&self, force: bool) -> Result<PathBuf> {
        ReleaseWorkflow::releasing(&self.slug).write_into(&self.root, force)
    }
}

fn parent_of(app_directory: &Path) -> Result<PathBuf> {
    app_directory
        .parent()
        .map(Path::to_owned)
        .with_context(|| format!("{} has no parent directory", app_directory.display()))
}

fn app_directories_in(root: &Path) -> Result<Vec<PathBuf>> {
    let listing = read_dir(root).with_context(|| format!("reading {}", root.display()))?;
    let mut found: Vec<PathBuf> = listing
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.join(METADATA).is_file())
        .collect();
    found.sort();
    Ok(found)
}

fn named(directories: &[PathBuf]) -> String {
    directories
        .iter()
        .filter_map(|path| path.file_name())
        .filter_map(|name| name.to_str())
        .collect::<Vec<&str>>()
        .join(", ")
}

#[cfg(test)]
mod tests {
    use std::fs::{create_dir_all, write};
    use std::path::Path;

    use tempfile::TempDir;

    use super::ExistingProject;

    fn project_in(root: &Path, slug: &str) {
        let app_directory = root.join(slug);
        create_dir_all(&app_directory).unwrap();
        write(
            app_directory.join("metadata.json"),
            r#"{"name": "HW Test"}"#,
        )
        .unwrap();
        write(
            app_directory.join("MANIFEST.JSON"),
            r#"{"name": "HW Test"}"#,
        )
        .unwrap();
    }

    #[test]
    fn found_from_the_project_root() {
        let kept = TempDir::new().unwrap();
        project_in(kept.path(), "org.fri3d.hwtest");

        let project = ExistingProject::found_at(kept.path()).unwrap();

        assert!(project.metadata().is_ok());
    }

    #[test]
    fn found_from_inside_the_app_directory() {
        let kept = TempDir::new().unwrap();
        project_in(kept.path(), "org.fri3d.hwtest");

        let project = ExistingProject::found_at(&kept.path().join("org.fri3d.hwtest")).unwrap();

        assert!(project.metadata().is_ok());
    }

    #[test]
    fn refused_where_there_is_no_project() {
        let kept = TempDir::new().unwrap();

        let refused = ExistingProject::found_at(kept.path())
            .unwrap_err()
            .to_string();

        assert!(refused.contains("no BadgeHub project"), "{refused}");
    }

    #[test]
    fn several_projects_are_named_rather_than_guessed_between() {
        let kept = TempDir::new().unwrap();
        project_in(kept.path(), "org.fri3d.hwtest");
        project_in(kept.path(), "org.fri3d.agenda");

        let refused = ExistingProject::found_at(kept.path())
            .unwrap_err()
            .to_string();

        assert!(refused.contains("org.fri3d.agenda"), "{refused}");
        assert!(refused.contains("org.fri3d.hwtest"), "{refused}");
        assert!(refused.contains("--app-directory"), "{refused}");
    }

    #[test]
    fn a_directory_name_that_is_not_a_slug_is_refused() {
        let kept = TempDir::new().unwrap();
        project_in(kept.path(), "Not A Slug");

        let refused = ExistingProject::found_at(kept.path())
            .unwrap_err()
            .to_string();

        assert!(refused.contains("not a valid BadgeHub slug"), "{refused}");
    }

    #[test]
    fn a_directory_that_does_not_exist_says_so() {
        let kept = TempDir::new().unwrap();

        let refused = ExistingProject::found_at(&kept.path().join("nowhere"));

        assert!(refused.is_err());
    }
}
