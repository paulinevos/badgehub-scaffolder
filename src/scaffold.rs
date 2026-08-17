use std::fs::{create_dir_all, write};
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
    pub fn beside(working_directory: &Path, project: &NewProject) -> Result<Self> {
        let root = working_directory.join(project.slug.to_string());
        if root.exists() {
            bail!("{} already exists", root.display());
        }
        let app_directory = root.join(project.slug.to_string());
        Ok(Self {
            root,
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

fn entrypoint_source(project: &NewProject) -> String {
    format!(
        "class {class_name}:\n    \"\"\"{description}\"\"\"\n\n    \
         def onCreate(self):\n        print(\"{name} starting\")\n",
        class_name = project.slug.suggested_class_name(),
        description = project.description,
        name = project.name,
    )
}
