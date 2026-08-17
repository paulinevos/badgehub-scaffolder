use std::fs::write;
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
    use super::stub_for;

    #[test]
    fn a_stub_names_the_licence_and_where_to_get_the_text() {
        let stub = stub_for("MIT");

        assert!(stub.starts_with("MIT"), "{stub}");
        assert!(stub.contains("spdx.org"), "{stub}");
    }
}
