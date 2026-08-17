//! `bh set` and `bh release-action`, run as the binary against a project this
//! tool scaffolded a moment earlier.

use std::fs::{read, read_to_string, write};
use std::path::{Path, PathBuf};
use std::process::Output;

use assert_cmd::Command;
use serde_json::Value;
use tempfile::TempDir;

const SLUG: &str = "org.fri3d.hwtest";
/// Nothing listens here, so no test reaches BadgeHub or GitHub.
const NOWHERE: &str = "http://127.0.0.1:1";

/// A scaffolded project in its own temporary directory, ready to be edited.
struct Scaffolded {
    home: TempDir,
    work: TempDir,
}

impl Scaffolded {
    fn new(extra: &[&str]) -> Self {
        let scaffolded = Self {
            home: TempDir::new().unwrap(),
            work: TempDir::new().unwrap(),
        };
        let mut arguments = vec![
            "new",
            "--slug",
            SLUG,
            "--name",
            "HW Test",
            "--description",
            "Checks the badge",
            "--author",
            "Pauline",
            "--project-type",
            "app",
            "--category",
            "Utility",
            "--badge",
            "fri3d_2026",
            "--license",
            "MIT",
        ];
        arguments.extend_from_slice(extra);
        let made = scaffolded.run_in(scaffolded.work.path(), &arguments);
        assert!(made.status.success(), "{}", complaint(&made));
        scaffolded
    }

    /// `bh new` scaffolds in place, so the working directory is the root.
    fn root(&self) -> PathBuf {
        self.work.path().to_path_buf()
    }

    fn run(&self, arguments: &[&str]) -> Output {
        self.run_in(&self.root(), arguments)
    }

    fn run_in(&self, directory: &Path, arguments: &[&str]) -> Output {
        Command::cargo_bin("bh")
            .unwrap()
            .args(arguments)
            .current_dir(directory)
            .env("XDG_CONFIG_HOME", self.home.path())
            .env("BADGEHUB_API_URL", NOWHERE)
            .env("GITHUB_API_URL", NOWHERE)
            .env_remove("GITHUB_TOKEN")
            .env_remove("GH_TOKEN")
            .output()
            .unwrap()
    }

    fn metadata(&self) -> Value {
        read_json(&self.root().join(SLUG).join("metadata.json"))
    }

    fn manifest(&self) -> Value {
        read_json(&self.root().join(SLUG).join("MANIFEST.JSON"))
    }

    fn workflow(&self) -> PathBuf {
        self.root().join(".github/workflows/release.yml")
    }
}

fn read_json(path: &Path) -> Value {
    serde_json::from_str(&read_to_string(path).unwrap()).unwrap()
}

fn complaint(outcome: &Output) -> String {
    String::from_utf8(outcome.stderr.clone()).unwrap()
}

fn spoken(outcome: &Output) -> String {
    String::from_utf8(outcome.stdout.clone()).unwrap()
}

#[test]
fn a_new_name_reaches_both_files_under_the_name_each_uses() {
    let project = Scaffolded::new(&[]);

    let outcome = project.run(&["set", "--name", "Hardware Test"]);

    assert!(outcome.status.success(), "{}", complaint(&outcome));
    assert_eq!("Hardware Test", project.metadata()["name"]);
    assert_eq!("Hardware Test", project.manifest()["name"]);
}

#[test]
fn the_author_is_publisher_on_the_launcher_side() {
    let project = Scaffolded::new(&[]);

    project.run(&["set", "--author", "Pauline Vos"]);

    assert_eq!("Pauline Vos", project.metadata()["author"]);
    assert_eq!("Pauline Vos", project.manifest()["publisher"]);
}

#[test]
fn the_description_is_short_description_on_the_launcher_side() {
    let project = Scaffolded::new(&[]);

    project.run(&["set", "--description", "Tests every peripheral"]);

    assert_eq!("Tests every peripheral", project.metadata()["description"]);
    assert_eq!(
        "Tests every peripheral",
        project.manifest()["short_description"]
    );
}

#[test]
fn what_was_not_asked_for_is_left_alone() {
    let project = Scaffolded::new(&[]);

    project.run(&["set", "--name", "Hardware Test"]);

    assert_eq!("Pauline", project.metadata()["author"]);
    assert_eq!("Checks the badge", project.metadata()["description"]);
}

#[test]
fn a_field_this_tool_knows_nothing_about_survives_an_edit() {
    let project = Scaffolded::new(&[]);
    let path = project.root().join(SLUG).join("metadata.json");
    let mut present = read_json(&path);
    present["published_at"] = Value::String("yesterday".to_owned());
    write(&path, format!("{present:#}\n")).unwrap();

    project.run(&["set", "--name", "Hardware Test"]);

    assert_eq!("yesterday", project.metadata()["published_at"]);
}

#[test]
fn it_can_be_run_from_inside_the_app_directory_too() {
    let project = Scaffolded::new(&[]);

    let outcome = project.run_in(
        &project.root().join(SLUG),
        &["set", "--name", "Hardware Test"],
    );

    assert!(outcome.status.success(), "{}", complaint(&outcome));
    assert_eq!("Hardware Test", project.metadata()["name"]);
}

#[test]
fn outside_a_project_it_says_there_is_nothing_here() {
    let project = Scaffolded::new(&[]);
    let empty = TempDir::new().unwrap();

    let outcome = project.run_in(empty.path(), &["set", "--name", "Hardware Test"]);

    assert!(!outcome.status.success());
    assert!(complaint(&outcome).contains("no BadgeHub project"));
}

#[test]
fn a_blank_value_is_refused_before_either_file_is_touched() {
    let project = Scaffolded::new(&[]);

    let outcome = project.run(&["set", "--name", "   "]);

    assert!(!outcome.status.success());
    assert_eq!("HW Test", project.metadata()["name"]);
    assert_eq!("HW Test", project.manifest()["name"]);
}

#[test]
fn with_no_flags_and_no_terminal_it_names_the_flags_rather_than_hanging() {
    let project = Scaffolded::new(&[]);

    let outcome = project.run(&["set"]);

    assert!(!outcome.status.success());
    let said = complaint(&outcome);
    assert!(said.contains("--name"), "{said}");
    assert!(!said.contains("--license"), "{said}");
}

#[test]
fn scaffolding_can_write_the_release_workflow_on_request() {
    let project = Scaffolded::new(&["--release-action"]);

    let written = read_to_string(project.workflow()).unwrap();

    assert!(
        written.contains(&format!("app-directory: {SLUG}")),
        "{written}"
    );
    assert!(written.contains("badgehub-release-action@v1"), "{written}");
}

#[test]
fn scaffolding_writes_no_workflow_when_told_not_to() {
    let project = Scaffolded::new(&["--no-release-action"]);

    assert!(!project.root().join(".github").exists());
}

#[test]
fn scaffolding_without_a_terminal_writes_no_workflow_by_default() {
    let project = Scaffolded::new(&[]);

    assert!(!project.root().join(".github").exists());
}

#[test]
fn asking_for_and_refusing_the_workflow_at_once_is_refused() {
    let project = Scaffolded::new(&[]);

    let outcome = project.run(&["new", "--release-action", "--no-release-action"]);

    assert!(!outcome.status.success());
    assert!(complaint(&outcome).contains("cannot be used with"));
}

#[test]
fn the_workflow_can_be_added_to_a_project_that_has_none() {
    let project = Scaffolded::new(&[]);

    let outcome = project.run(&["release-action"]);

    assert!(outcome.status.success(), "{}", complaint(&outcome));
    assert!(project.workflow().is_file());
}

#[test]
fn adding_the_workflow_says_what_is_still_needed_to_publish() {
    let project = Scaffolded::new(&[]);

    let outcome = project.run(&["release-action"]);

    let said = spoken(&outcome);
    assert!(said.contains("gh secret set BADGEHUB_API_TOKEN"), "{said}");
    assert!(said.contains("badgehub.eu"), "{said}");
}

#[test]
fn an_existing_workflow_is_not_replaced_without_being_asked() {
    let project = Scaffolded::new(&["--release-action"]);

    let outcome = project.run(&["release-action"]);

    assert!(!outcome.status.success());
    assert!(complaint(&outcome).contains("--force"));
}

#[test]
fn forcing_replaces_the_workflow() {
    let project = Scaffolded::new(&["--release-action"]);
    write(project.workflow(), "name: something else\n").unwrap();

    let outcome = project.run(&["release-action", "--force"]);

    assert!(outcome.status.success(), "{}", complaint(&outcome));
    assert!(
        read_to_string(project.workflow())
            .unwrap()
            .contains("app-directory")
    );
}

#[test]
fn bundling_writes_an_mpk_named_for_the_manifest() {
    let project = Scaffolded::new(&[]);

    let outcome = project.run(&["bundle"]);

    assert!(outcome.status.success(), "{}", complaint(&outcome));
    assert!(project.root().join(format!("{SLUG}_0.1.0.mpk")).is_file());
}

#[test]
fn the_bundle_holds_one_top_level_directory_named_for_the_slug() {
    let project = Scaffolded::new(&[]);

    project.run(&["bundle"]);

    let names = names_in(&project.root().join(format!("{SLUG}_0.1.0.mpk")));
    assert_eq!(format!("{SLUG}/"), names[0]);
    assert!(
        names
            .iter()
            .all(|name| name.starts_with(&format!("{SLUG}/"))),
        "{names:?}"
    );
}

#[test]
fn the_bundle_carries_what_the_badge_needs() {
    let project = Scaffolded::new(&[]);

    project.run(&["bundle"]);

    let names = names_in(&project.root().join(format!("{SLUG}_0.1.0.mpk")));
    for wanted in ["MANIFEST.JSON", "metadata.json", "__init__.py"] {
        assert!(names.contains(&format!("{SLUG}/{wanted}")), "{names:?}");
    }
}

#[test]
fn bundling_twice_gives_the_same_bytes() {
    let project = Scaffolded::new(&[]);
    let archive = project.root().join(format!("{SLUG}_0.1.0.mpk"));

    project.run(&["bundle"]);
    let once = read(&archive).unwrap();
    project.run(&["bundle"]);

    assert_eq!(once, read(&archive).unwrap());
}

#[test]
fn a_new_version_gets_a_new_file_name() {
    let project = Scaffolded::new(&[]);
    let manifest = project.root().join(SLUG).join("MANIFEST.JSON");
    let bumped = read_to_string(&manifest).unwrap().replace("0.1.0", "2.0.0");
    write(&manifest, bumped).unwrap();

    project.run(&["bundle"]);

    assert!(project.root().join(format!("{SLUG}_2.0.0.mpk")).is_file());
}

#[test]
fn an_output_directory_can_be_named() {
    let project = Scaffolded::new(&[]);
    let elsewhere = TempDir::new().unwrap();

    let outcome = project.run(&[
        "bundle",
        "--output-directory",
        elsewhere.path().to_str().unwrap(),
    ]);

    assert!(outcome.status.success(), "{}", complaint(&outcome));
    assert!(elsewhere.path().join(format!("{SLUG}_0.1.0.mpk")).is_file());
}

#[test]
fn a_scaffolded_project_already_ignores_the_mpk_and_is_not_told_again() {
    let project = Scaffolded::new(&[]);

    let outcome = project.run(&["bundle"]);

    assert!(
        !spoken(&outcome).contains(".gitignore"),
        "{}",
        spoken(&outcome)
    );
}

#[test]
fn a_project_whose_gitignore_forgot_the_mpk_has_it_added() {
    let project = Scaffolded::new(&[]);
    write(project.root().join(".gitignore"), "__pycache__/\n").unwrap();

    let outcome = project.run(&["bundle"]);

    assert!(
        spoken(&outcome).contains("Added *.mpk"),
        "{}",
        spoken(&outcome)
    );
    let ignored = read_to_string(project.root().join(".gitignore")).unwrap();
    assert_eq!("__pycache__/\n*.mpk\n", ignored);
}

#[test]
fn bundling_outside_a_project_says_there_is_nothing_here() {
    let project = Scaffolded::new(&[]);
    let empty = TempDir::new().unwrap();

    let outcome = project.run_in(empty.path(), &["bundle"]);

    assert!(!outcome.status.success());
    assert!(complaint(&outcome).contains("no BadgeHub project"));
}

fn names_in(archive: &std::path::Path) -> Vec<String> {
    let reader = std::fs::File::open(archive).unwrap();
    let mut zip = zip::ZipArchive::new(reader).unwrap();
    (0..zip.len())
        .map(|index| zip.by_index(index).unwrap().name().to_owned())
        .collect()
}
