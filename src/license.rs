use std::fs::{read_to_string, write};
use std::path::PathBuf;

use anyhow::{Context, Result};

/// The LICENSE file beside a project. Only ever a placeholder while this tool
/// owns it: only the user can say who the copyright holder is, and inventing
/// licence wording would be worse than an obvious stub.
pub struct LicenseFile {
    path: PathBuf,
}

impl LicenseFile {
    pub fn at(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn write_stub_for(&self, license: &str) -> Result<()> {
        write(&self.path, stub_for(license))
            .with_context(|| format!("writing {}", self.path.display()))
    }

    /// Rewritten only while it is still the stub this tool wrote: once someone
    /// has pasted the real licence text in, replacing it would destroy work
    /// that cannot be recovered.
    pub fn replace_stub(&self, was: &str, becomes: &str) -> Result<Replaced> {
        let Ok(present) = read_to_string(&self.path) else {
            self.write_stub_for(becomes)?;
            return Ok(Replaced::Written);
        };
        if present != stub_for(was) {
            return Ok(Replaced::LeftAlone);
        }
        self.write_stub_for(becomes)?;
        Ok(Replaced::Written)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum Replaced {
    Written,
    LeftAlone,
}

pub fn stub_for(license: &str) -> String {
    format!(
        "{license}\n\nReplace this file with the full text of the {license} \
         licence, from <https://spdx.org/licenses/>, and fill in the \
         copyright holder and year.\n"
    )
}

#[cfg(test)]
mod tests {
    use std::fs::{read_to_string, write};

    use tempfile::TempDir;

    use super::{LicenseFile, Replaced, stub_for};

    fn holding(contents: Option<&str>) -> (TempDir, LicenseFile) {
        let kept = TempDir::new().unwrap();
        let path = kept.path().join("LICENSE");
        if let Some(contents) = contents {
            write(&path, contents).unwrap();
        }
        let file = LicenseFile::at(path);
        (kept, file)
    }

    #[test]
    fn an_untouched_stub_is_rewritten_for_the_new_licence() {
        let (_kept, file) = holding(Some(&stub_for("MIT")));

        let outcome = file.replace_stub("MIT", "Apache-2.0").unwrap();

        assert_eq!(Replaced::Written, outcome);
    }

    #[test]
    fn a_stub_someone_has_edited_is_left_alone() {
        let (_kept, file) = holding(Some("MIT\n\nCopyright 2026 Pauline Vos\n"));

        let outcome = file.replace_stub("MIT", "Apache-2.0").unwrap();

        assert_eq!(Replaced::LeftAlone, outcome);
    }

    #[test]
    fn a_project_that_had_no_licence_gets_a_stub() {
        let (kept, file) = holding(None);

        let outcome = file.replace_stub("MIT", "Apache-2.0").unwrap();

        assert_eq!(Replaced::Written, outcome);
        let written = read_to_string(kept.path().join("LICENSE")).unwrap();
        assert!(written.starts_with("Apache-2.0"), "{written}");
    }

    #[test]
    fn a_stub_names_the_licence_and_where_to_get_the_text() {
        let stub = stub_for("MIT");

        assert!(stub.starts_with("MIT"), "{stub}");
        assert!(stub.contains("spdx.org"), "{stub}");
    }
}
