use std::fs::{create_dir_all, read_dir, write};
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};
use serde::Serialize;

use crate::answers::NewProject;
use crate::license::LicenseFile;
use crate::manifest::Manifest;
use crate::metadata::Metadata;
use crate::release_action::ReleaseWorkflow;
use crate::repository_url::RepositoryUrl;

const GITIGNORE: &str = "__pycache__/\n*.pyc\n*.mpk\n.venv/\n.DS_Store\n";

pub struct Scaffold {
    root: PathBuf,
    app_directory: PathBuf,
}

impl Scaffold {
    /// Scaffolds into the given directory itself, rather than making one to
    /// hold it. That directory has to be empty: a project laid over anything
    /// already there would be two things sharing a root.
    pub fn here(directory: &Path, project: &NewProject) -> Result<Self> {
        refuse_unless_empty(directory)?;
        let app_directory = directory.join(project.slug.to_string());
        Ok(Self {
            root: directory.to_owned(),
            app_directory,
        })
    }

    pub fn write_out(&self, project: &NewProject) -> Result<&Path> {
        create_dir_all(&self.app_directory)
            .with_context(|| format!("creating {}", self.app_directory.display()))?;
        self.write_repository_files(project)?;
        self.write_app_files(project)?;
        self.write_release_workflow(project)?;
        // Last, so everything above is there to be committed.
        self.set_up_git(project)?;
        Ok(&self.root)
    }

    fn write_release_workflow(&self, project: &NewProject) -> Result<()> {
        if !project.release_workflow {
            return Ok(());
        }
        ReleaseWorkflow::releasing(&project.slug).write_into(&self.root, false)?;
        Ok(())
    }

    fn set_up_git(&self, project: &NewProject) -> Result<()> {
        let git = Git::in_directory(&self.root);
        git.initialise()?;
        let Some(url) = &project.git_url else {
            return Ok(());
        };
        git.point_origin_at(url)
    }

    fn write_repository_files(&self, project: &NewProject) -> Result<()> {
        self.put(self.root.join(".gitignore"), GITIGNORE)?;
        self.put(self.root.join("README.md"), &project.readme())?;
        self.write_license(project)
    }

    fn write_license(&self, project: &NewProject) -> Result<()> {
        let Some(license) = &project.license_type else {
            return Ok(());
        };
        LicenseFile::at(self.root.join("LICENSE")).write_stub_for(license)
    }

    fn write_app_files(&self, project: &NewProject) -> Result<()> {
        self.put_json(
            self.app_directory.join("metadata.json"),
            &Metadata::describing(project),
        )?;
        self.put_json(
            self.app_directory.join("MANIFEST.JSON"),
            &Manifest::describing(project),
        )?;
        self.put(
            self.app_directory.join(NewProject::entrypoint()),
            &entrypoint_source(project),
        )
    }

    fn put_json(&self, path: PathBuf, content: &impl Serialize) -> Result<()> {
        let rendered = serde_json::to_string_pretty(content)?;
        self.put(path, &format!("{rendered}\n"))
    }

    fn put(&self, path: PathBuf, content: &str) -> Result<()> {
        write(&path, content).with_context(|| format!("writing {}", path.display()))
    }
}

struct Git<'a> {
    directory: &'a Path,
}

impl<'a> Git<'a> {
    fn in_directory(directory: &'a Path) -> Self {
        Self { directory }
    }

    fn initialise(&self) -> Result<()> {
        self.run(&["init", "--quiet"])
    }

    fn point_origin_at(&self, url: &RepositoryUrl) -> Result<()> {
        self.run(&["remote", "add", "origin", &url.to_string()])
    }

    fn run(&self, arguments: &[&str]) -> Result<()> {
        let outcome = Command::new("git")
            .arg("-C")
            .arg(self.directory)
            .args(arguments)
            .status()
            .context("running git — is git installed?")?;
        if !outcome.success() {
            bail!(
                "`git {}` failed in {}",
                arguments.join(" "),
                self.directory.display()
            );
        }
        Ok(())
    }
}

/// An empty directory only. Refusing beforehand beats writing half a project
/// over whatever was already living here.
pub fn refuse_unless_empty(directory: &Path) -> Result<()> {
    let occupied = occupants(directory)?;
    if occupied.is_empty() {
        return Ok(());
    }
    bail!(
        "{} is not empty: it holds {}. Scaffold into an empty directory.",
        directory.display(),
        occupied.join(", ")
    )
}

fn occupants(directory: &Path) -> Result<Vec<String>> {
    let listing =
        read_dir(directory).with_context(|| format!("reading {}", directory.display()))?;
    let mut found: Vec<String> = listing
        .filter_map(Result::ok)
        .filter_map(|entry| entry.file_name().into_string().ok())
        .collect();
    found.sort();
    Ok(found)
}

fn entrypoint_source(project: &NewProject) -> String {
    format!(
        "class {class_name}:\n    \"\"\"{description}\"\"\"\n\n    \
         def onCreate(self):\n        print(\"{name} starting\")\n",
        class_name = project.slug.suggested_class_name(),
        description = project.description,
        name = project.name,
    )
}

#[cfg(test)]
mod tests {
    use std::fs::{create_dir, write};

    use tempfile::TempDir;

    use super::{occupants, refuse_unless_empty};

    #[test]
    fn an_empty_directory_is_what_this_wants() {
        let kept = TempDir::new().unwrap();

        assert!(refuse_unless_empty(kept.path()).is_ok());
    }

    #[test]
    fn a_file_someone_else_put_there_is_refused() {
        let kept = TempDir::new().unwrap();
        write(kept.path().join("notes.txt"), "mine\n").unwrap();

        let refused = refuse_unless_empty(kept.path()).unwrap_err().to_string();

        assert!(refused.contains("not empty"), "{refused}");
        assert!(refused.contains("notes.txt"), "{refused}");
    }

    /// Not even a git directory earns an exception.
    #[test]
    fn a_directory_that_is_already_a_repository_is_refused() {
        let kept = TempDir::new().unwrap();
        create_dir(kept.path().join(".git")).unwrap();

        let refused = refuse_unless_empty(kept.path()).unwrap_err().to_string();

        assert!(refused.contains(".git"), "{refused}");
    }

    #[test]
    fn the_files_github_starts_a_repository_with_are_refused_too() {
        let kept = TempDir::new().unwrap();
        write(kept.path().join("README.md"), "# demo\n").unwrap();
        write(kept.path().join("LICENSE"), "MIT\n").unwrap();
        write(kept.path().join(".gitignore"), "*.pyc\n").unwrap();

        let refused = refuse_unless_empty(kept.path()).unwrap_err().to_string();

        assert!(refused.contains("README.md"), "{refused}");
        assert!(refused.contains("LICENSE"), "{refused}");
        assert!(refused.contains(".gitignore"), "{refused}");
    }

    #[test]
    fn a_project_scaffolded_here_already_is_in_the_way() {
        let kept = TempDir::new().unwrap();
        create_dir(kept.path().join("org.fri3d.hwtest")).unwrap();

        let refused = refuse_unless_empty(kept.path()).unwrap_err().to_string();

        assert!(refused.contains("org.fri3d.hwtest"), "{refused}");
    }

    #[test]
    fn what_is_in_the_way_is_listed_in_a_settled_order() {
        let kept = TempDir::new().unwrap();
        write(kept.path().join("zebra"), "").unwrap();
        write(kept.path().join("apple"), "").unwrap();

        assert_eq!(vec!["apple", "zebra"], occupants(kept.path()).unwrap());
    }
}
