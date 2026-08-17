use std::fs::{read_to_string, write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

/// The project's .gitignore, added to rather than replaced: whatever is already
/// in there was put there on purpose.
pub struct Gitignore {
    path: PathBuf,
}

impl Gitignore {
    pub fn at(directory: &Path) -> Self {
        Self {
            path: directory.join(".gitignore"),
        }
    }

    /// Adds the rule unless some line already says it, and reports whether it
    /// had to. A build artefact nobody asked to commit should not need a second
    /// step to keep out of the repository.
    pub fn ensure(&self, rule: &str) -> Result<Added> {
        let Ok(present) = read_to_string(&self.path) else {
            self.put(&format!("{rule}\n"))?;
            return Ok(Added::Yes);
        };
        if present.lines().any(|line| line.trim() == rule) {
            return Ok(Added::AlreadyThere);
        }
        let separator = if present.ends_with('\n') { "" } else { "\n" };
        self.put(&format!("{present}{separator}{rule}\n"))?;
        Ok(Added::Yes)
    }

    fn put(&self, contents: &str) -> Result<()> {
        write(&self.path, contents).with_context(|| format!("writing {}", self.path.display()))
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum Added {
    Yes,
    AlreadyThere,
}

#[cfg(test)]
mod tests {
    use std::fs::{read_to_string, write};

    use tempfile::TempDir;

    use super::{Added, Gitignore};

    #[test]
    fn a_project_without_a_gitignore_gets_one() {
        let kept = TempDir::new().unwrap();

        let added = Gitignore::at(kept.path()).ensure("*.mpk").unwrap();

        assert_eq!(Added::Yes, added);
        assert_eq!(
            "*.mpk\n",
            read_to_string(kept.path().join(".gitignore")).unwrap()
        );
    }

    #[test]
    fn a_rule_already_there_is_left_alone() {
        let kept = TempDir::new().unwrap();
        write(kept.path().join(".gitignore"), "target/\n*.mpk\n").unwrap();

        let added = Gitignore::at(kept.path()).ensure("*.mpk").unwrap();

        assert_eq!(Added::AlreadyThere, added);
        assert_eq!(
            "target/\n*.mpk\n",
            read_to_string(kept.path().join(".gitignore")).unwrap()
        );
    }

    #[test]
    fn a_missing_rule_is_appended_to_what_is_there() {
        let kept = TempDir::new().unwrap();
        write(kept.path().join(".gitignore"), "target/\n").unwrap();

        Gitignore::at(kept.path()).ensure("*.mpk").unwrap();

        assert_eq!(
            "target/\n*.mpk\n",
            read_to_string(kept.path().join(".gitignore")).unwrap()
        );
    }

    #[test]
    fn a_file_without_a_final_newline_does_not_run_the_rules_together() {
        let kept = TempDir::new().unwrap();
        write(kept.path().join(".gitignore"), "target/").unwrap();

        Gitignore::at(kept.path()).ensure("*.mpk").unwrap();

        assert_eq!(
            "target/\n*.mpk\n",
            read_to_string(kept.path().join(".gitignore")).unwrap()
        );
    }

    #[test]
    fn a_rule_written_with_trailing_space_still_counts() {
        let kept = TempDir::new().unwrap();
        write(kept.path().join(".gitignore"), "*.mpk  \n").unwrap();

        let added = Gitignore::at(kept.path()).ensure("*.mpk").unwrap();

        assert_eq!(Added::AlreadyThere, added);
    }
}
