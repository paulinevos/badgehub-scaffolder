use std::fs::create_dir_all;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

use crate::slug::Slug;

const WORKFLOW: &str = ".github/workflows/release.yml";
/// The only tag the action publishes. Pinned rather than floating on a branch
/// so a release is built by the action the project was set up against.
const ACTION: &str = "paulinevos/badgehub-release-action@v1";

const ACTION_HOLE: &str = "ACTION_REF";
const APP_DIRECTORY_HOLE: &str = "APP_DIRECTORY";
const TEMPLATE: &str = r#"name: release

on:
  release:
    types: [published]

permissions:
  contents: write

jobs:
  release:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-python@v5
        with:
          python-version: '3.12'
      - uses: ACTION_REF
        with:
          version: ${{ github.event.release.tag_name }}
          app-directory: APP_DIRECTORY
          badgehub-token: ${{ secrets.BADGEHUB_API_TOKEN }}
"#;

const WHAT_IS_LEFT_TO_DO: &str = "\
The workflow publishes to BadgeHub with a project API token. Create the project
at <https://badgehub.eu>, generate its token, then:

    gh secret set BADGEHUB_API_TOKEN

Without that secret the workflow still builds the .mpk on every release and
warns instead of publishing, so this is not urgent.";

/// The GitHub Actions workflow that releases a BadgeHub app: it hands the tag
/// and the app directory to badgehub-release-action, which writes the version
/// into MANIFEST.JSON, builds the .mpk and publishes it.
pub struct ReleaseWorkflow {
    app_directory: String,
}

impl ReleaseWorkflow {
    /// The action requires that the directory holding MANIFEST.JSON is named
    /// for the manifest's fullname, which is the slug — so the slug is the
    /// only thing this needs to know.
    pub fn releasing(slug: &Slug) -> Self {
        Self {
            app_directory: slug.to_string(),
        }
    }

    pub fn write_into(&self, root: &Path, force: bool) -> Result<PathBuf> {
        let path = root.join(WORKFLOW);
        if path.exists() && !force {
            bail!(
                "{} already exists. Pass --force to replace it.",
                path.display()
            );
        }
        let directory = path
            .parent()
            .with_context(|| format!("{} has no parent directory", path.display()))?;
        create_dir_all(directory).with_context(|| format!("creating {}", directory.display()))?;
        std::fs::write(&path, self.yaml())
            .with_context(|| format!("writing {}", path.display()))?;
        Ok(path)
    }

    // Written out as literal YAML rather than built from a serialiser: what a
    // reader wants to check is that this matches the action's documented
    // example, and it only does that if it looks like it.
    fn yaml(&self) -> String {
        TEMPLATE
            .replace(ACTION_HOLE, ACTION)
            .replace(APP_DIRECTORY_HOLE, &self.app_directory)
    }
}

pub fn what_is_left_to_do() -> &'static str {
    WHAT_IS_LEFT_TO_DO
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::ReleaseWorkflow;
    use crate::slug::Slug;

    fn hwtest() -> ReleaseWorkflow {
        ReleaseWorkflow::releasing(&Slug::parse("org.fri3d.hwtest").unwrap())
    }

    #[test]
    fn the_slug_is_the_app_directory_the_action_is_given() {
        let written = hwtest().yaml();

        assert!(
            written.contains("app-directory: org.fri3d.hwtest"),
            "{written}"
        );
    }

    #[test]
    fn the_action_is_pinned_to_a_released_tag() {
        assert!(hwtest().yaml().contains("badgehub-release-action@v1"));
    }

    #[test]
    fn the_tag_and_the_token_are_passed_through_as_expressions() {
        let written = hwtest().yaml();

        assert!(
            written.contains("version: ${{ github.event.release.tag_name }}"),
            "{written}"
        );
        assert!(
            written.contains("badgehub-token: ${{ secrets.BADGEHUB_API_TOKEN }}"),
            "{written}"
        );
    }

    #[test]
    fn it_lands_where_github_actions_looks_for_it() {
        let kept = TempDir::new().unwrap();

        let path = hwtest().write_into(kept.path(), false).unwrap();

        assert!(path.ends_with(".github/workflows/release.yml"));
        assert!(path.is_file());
    }

    #[test]
    fn an_existing_workflow_is_not_replaced_by_accident() {
        let kept = TempDir::new().unwrap();
        hwtest().write_into(kept.path(), false).unwrap();

        let refused = hwtest()
            .write_into(kept.path(), false)
            .unwrap_err()
            .to_string();

        assert!(refused.contains("--force"), "{refused}");
    }

    #[test]
    fn forcing_replaces_it() {
        let kept = TempDir::new().unwrap();
        hwtest().write_into(kept.path(), false).unwrap();

        assert!(hwtest().write_into(kept.path(), true).is_ok());
    }
}
