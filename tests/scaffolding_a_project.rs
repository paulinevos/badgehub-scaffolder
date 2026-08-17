//! The whole tool, run as `bh` would be. Every network call goes to a local
//! mock, and every run gets its own config and working directory, so nothing
//! here touches the real BadgeHub, GitHub, or the machine's own config.

use std::fs::{create_dir_all, read_to_string, write};
use std::path::{Path, PathBuf};
use std::process::Output;

use assert_cmd::Command;
use mockito::{Server, ServerGuard};
use serde_json::Value;
use tempfile::TempDir;

const CREATED: &str = include_str!("fixtures/created_repository.json");
const CLONE_URL: &str = "https://github.com/octokit-fixture-org/rename-repository-newname.git";
/// Nothing listens here, so the catalogue falls back to its baked-in lists
/// without waiting on a timeout.
const NO_BADGEHUB: &str = "http://127.0.0.1:1";

const SLUG: &str = "org.fri3d.hwtest";

/// One run of `bh`: its own home, its own working directory, and no way to
/// reach anything real.
struct Run {
    home: TempDir,
    work: TempDir,
    badgehub: String,
    github: String,
}

impl Run {
    fn new() -> Self {
        Self {
            home: TempDir::new().unwrap(),
            work: TempDir::new().unwrap(),
            badgehub: NO_BADGEHUB.to_owned(),
            github: NO_BADGEHUB.to_owned(),
        }
    }

    fn with_config(self, contents: &str) -> Self {
        let directory = self.home.path().join("badgehub");
        create_dir_all(&directory).unwrap();
        write(directory.join("config.json"), contents).unwrap();
        self
    }

    fn against_github(mut self, server: &ServerGuard) -> Self {
        self.github = server.url();
        self
    }

    fn against_badgehub(mut self, server: &ServerGuard) -> Self {
        self.badgehub = server.url();
        self
    }

    fn run(&self, arguments: &[&str]) -> Output {
        Command::cargo_bin("bh")
            .unwrap()
            .args(arguments)
            .current_dir(self.work.path())
            .env("XDG_CONFIG_HOME", self.home.path())
            .env("BADGEHUB_API_URL", &self.badgehub)
            .env("GITHUB_API_URL", &self.github)
            .env_remove("GITHUB_TOKEN")
            .env_remove("GH_TOKEN")
            .output()
            .unwrap()
    }

    fn scaffold(&self, extra: &[&str]) -> Output {
        self.scaffold_into(&[], extra)
    }

    /// The directory is positional, so it has to lead the flags.
    fn scaffold_into(&self, directory: &[&str], extra: &[&str]) -> Output {
        let mut arguments = vec!["new"];
        arguments.extend_from_slice(directory);
        arguments.extend_from_slice(&[
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
        ]);
        arguments.extend_from_slice(extra);
        self.run(&arguments)
    }

    /// The project root is now the working directory itself: `bh new`
    /// scaffolds in place rather than making a directory to hold it.
    fn root(&self) -> &Path {
        self.work.path()
    }

    fn app(&self) -> PathBuf {
        self.root().join(SLUG)
    }

    fn metadata(&self) -> Value {
        let path = self.app().join("metadata.json");
        serde_json::from_str(&read_to_string(path).unwrap()).unwrap()
    }

    fn remotes(&self) -> String {
        let shown = std::process::Command::new("git")
            .arg("-C")
            .arg(self.root())
            .args(["remote", "-v"])
            .output()
            .unwrap();
        String::from_utf8(shown.stdout).unwrap()
    }
}

fn complaint(outcome: &Output) -> String {
    String::from_utf8(outcome.stderr.clone()).unwrap()
}

fn github_creating(server: &mut ServerGuard) -> mockito::Mock {
    server
        .mock("POST", "/user/repos")
        .with_status(201)
        .with_header("content-type", "application/json")
        .with_body(CREATED)
        .create()
}

#[test]
fn a_run_with_every_flag_needs_no_terminal() {
    let run = Run::new();

    let outcome = run.scaffold(&[]);

    assert!(outcome.status.success(), "{}", complaint(&outcome));
    assert!(run.app().join("__init__.py").exists());
    assert!(run.root().join("README.md").exists());
    assert!(run.root().join(".git").exists());
}

/// The wordmark is for a person at a terminal. Here stdin is a pipe, so every
/// stream has to come out as it did before there was a banner at all.
#[test]
fn the_wordmark_never_reaches_a_pipe() {
    let run = Run::new();

    let outcome = run.scaffold(&[]);

    let reported = String::from_utf8(outcome.stdout.clone()).unwrap();
    assert!(!reported.contains("__"), "{reported}");
    assert!(
        !complaint(&outcome).contains("__"),
        "{}",
        complaint(&outcome)
    );
    assert_eq!(1, reported.lines().count(), "{reported}");
}

#[test]
fn what_was_answered_reaches_the_store_listing() {
    let run = Run::new();

    run.scaffold(&[]);

    let metadata = run.metadata();
    assert_eq!("HW Test", metadata["name"]);
    assert_eq!("Pauline", metadata["author"]);
    assert_eq!("0.1.0", metadata["version"]);
    assert_eq!("app", metadata["project_type"]);
    assert_eq!(Value::Null, metadata["git_url"]);
}

#[test]
fn saved_defaults_answer_the_questions_the_flags_left_out() {
    let run = Run::new().with_config(r#"{"author": "Pauline Vos", "license": "MIT"}"#);

    let outcome = Command::cargo_bin("bh")
        .unwrap()
        .args([
            "new",
            "--slug",
            SLUG,
            "--name",
            "HW Test",
            "--description",
            "Checks the badge",
            "--project-type",
            "app",
            "--category",
            "Utility",
            "--badge",
            "fri3d_2026",
        ])
        .current_dir(run.root())
        .env("XDG_CONFIG_HOME", run.home.path())
        .env("BADGEHUB_API_URL", NO_BADGEHUB)
        .output()
        .unwrap();

    assert!(outcome.status.success(), "{}", complaint(&outcome));
    let metadata = run.metadata();
    assert_eq!("Pauline Vos", metadata["author"]);
    assert_eq!("MIT", metadata["license_type"]);
    assert!(run.root().join("LICENSE").exists());
}

#[test]
fn a_flag_beats_a_saved_default() {
    let run = Run::new().with_config(r#"{"author": "Pauline Vos"}"#);

    run.scaffold(&[]);

    assert_eq!("Pauline", run.metadata()["author"]);
}

#[test]
fn an_existing_repository_url_becomes_the_origin_remote() {
    let run = Run::new();

    let outcome = run.scaffold(&["--git-url", "https://github.com/fri3d/hwtest"]);

    assert!(outcome.status.success(), "{}", complaint(&outcome));
    assert_eq!("https://github.com/fri3d/hwtest", run.metadata()["git_url"]);
    assert!(
        run.remotes()
            .contains("origin\thttps://github.com/fri3d/hwtest")
    );
}

#[test]
fn no_repository_means_no_remote_and_no_git_url() {
    let run = Run::new();

    run.scaffold(&[]);

    assert_eq!("", run.remotes().trim());
}

#[test]
fn a_malformed_repository_url_is_refused_before_anything_is_written() {
    let run = Run::new();

    let outcome = run.scaffold(&["--git-url", "not a url"]);

    assert!(!outcome.status.success());
    assert!(complaint(&outcome).contains("is not a repository URL"));
    assert!(!run.app().exists());
}

#[test]
fn creating_a_repository_wires_up_the_remote_it_answers_with() {
    let mut server = Server::new();
    let created = github_creating(&mut server);
    let run = Run::new()
        .against_github(&server)
        .with_config(r#"{"github_token": "ghp_pretend"}"#);

    let outcome = run.scaffold(&["--create-repo"]);

    assert!(outcome.status.success(), "{}", complaint(&outcome));
    created.assert();
    assert_eq!(CLONE_URL, run.metadata()["git_url"]);
    assert!(run.remotes().contains(CLONE_URL));
}

#[test]
fn the_repository_is_named_after_the_slug_unless_told_otherwise() {
    let mut server = Server::new();
    let created = server
        .mock("POST", "/user/repos")
        .match_body(mockito::Matcher::PartialJson(
            serde_json::json!({ "name": SLUG }),
        ))
        .with_status(201)
        .with_body(CREATED)
        .create();
    let run = Run::new()
        .against_github(&server)
        .with_config(r#"{"github_token": "ghp_pretend"}"#);

    run.scaffold(&["--create-repo"]);

    created.assert();
}

#[test]
fn a_chosen_repository_name_and_visibility_are_what_gets_asked_for() {
    let mut server = Server::new();
    let created = server
        .mock("POST", "/user/repos")
        .match_body(mockito::Matcher::PartialJson(serde_json::json!({
            "name": "hwtest",
            "private": true,
        })))
        .with_status(201)
        .with_body(CREATED)
        .create();
    let run = Run::new()
        .against_github(&server)
        .with_config(r#"{"github_token": "ghp_pretend"}"#);

    let outcome = run.scaffold(&[
        "--create-repo",
        "--repo-name",
        "hwtest",
        "--visibility",
        "private",
    ]);

    assert!(outcome.status.success(), "{}", complaint(&outcome));
    created.assert();
}

#[test]
fn an_environment_token_is_enough_on_its_own() {
    let mut server = Server::new();
    let created = github_creating(&mut server);
    let run = Run::new().against_github(&server);

    let outcome = Command::cargo_bin("bh")
        .unwrap()
        .args([
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
            "--create-repo",
        ])
        .current_dir(run.root())
        .env("XDG_CONFIG_HOME", run.home.path())
        .env("BADGEHUB_API_URL", NO_BADGEHUB)
        .env("GITHUB_API_URL", &run.github)
        .env("GITHUB_TOKEN", "ghp_from_the_shell")
        .output()
        .unwrap();

    assert!(outcome.status.success(), "{}", complaint(&outcome));
    created.assert();
}

#[test]
fn creating_without_a_token_says_where_to_get_one_rather_than_prompting() {
    let run = Run::new();

    let outcome = run.scaffold(&["--create-repo"]);

    assert!(!outcome.status.success());
    assert!(
        complaint(&outcome).contains("GITHUB_TOKEN"),
        "{}",
        complaint(&outcome)
    );
    assert!(!run.root().join(SLUG).exists());
}

#[test]
fn a_refused_creation_leaves_no_half_made_project_behind() {
    let mut server = Server::new();
    server
        .mock("POST", "/user/repos")
        .with_status(422)
        .with_body(include_str!("fixtures/validation_failed.json"))
        .create();
    let run = Run::new()
        .against_github(&server)
        .with_config(r#"{"github_token": "ghp_pretend"}"#);

    let outcome = run.scaffold(&["--create-repo"]);

    assert!(!outcome.status.success());
    assert!(complaint(&outcome).contains("already has a repository"));
    assert!(!run.app().exists());
}

#[test]
fn creating_and_naming_an_existing_repository_are_not_both_allowed() {
    let run = Run::new();

    let outcome = run.scaffold(&["--create-repo", "--git-url", "https://github.com/a/b"]);

    assert!(!outcome.status.success());
    assert!(complaint(&outcome).contains("cannot be used with"));
}

#[test]
fn naming_a_repository_without_creating_one_is_refused() {
    let run = Run::new();

    let outcome = run.scaffold(&["--repo-name", "hwtest"]);

    assert!(!outcome.status.success());
    assert!(complaint(&outcome).contains("--create-repo"));
}

#[test]
fn the_live_catalogue_wins_over_the_baked_in_lists() {
    let mut server = Server::new();
    server
        .mock("GET", "/categories")
        .with_body(r#"["Mystery"]"#)
        .create();
    server
        .mock("GET", "/badges")
        .with_body(r#"["fri3d_2026"]"#)
        .create();
    let run = Run::new().against_badgehub(&server);

    let outcome = run.run(&[
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
        "Mystery",
        "--badge",
        "fri3d_2026",
    ]);

    assert!(outcome.status.success(), "{}", complaint(&outcome));
    assert_eq!("Mystery", run.metadata()["categories"][0]);
}

#[test]
fn a_category_no_badgehub_knows_is_refused() {
    let run = Run::new();

    let outcome = run.run(&[
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
        "Interpretive Dance",
        "--badge",
        "fri3d_2026",
    ]);

    assert!(!outcome.status.success());
    assert!(complaint(&outcome).contains("unknown categories"));
}

#[test]
fn an_invalid_slug_is_refused() {
    let run = Run::new();

    let outcome = run.run(&[
        "new",
        "--slug",
        "Not A Slug",
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
    ]);

    assert!(!outcome.status.success());
    assert!(complaint(&outcome).contains("not a valid BadgeHub slug"));
}

#[test]
fn scaffolding_twice_in_one_place_is_refused() {
    let run = Run::new();

    run.scaffold(&[]);
    let second = run.scaffold(&[]);

    assert!(!second.status.success());
    assert!(
        complaint(&second).contains("not empty"),
        "{}",
        complaint(&second)
    );
}

#[test]
fn a_missing_answer_names_the_flag_that_would_have_supplied_it() {
    let run = Run::new();

    let outcome = run.run(&["new", "--slug", SLUG]);

    assert!(!outcome.status.success());
    assert!(complaint(&outcome).contains("--name"));
}

#[test]
fn saving_defaults_leaves_a_config_only_its_owner_can_read() {
    use std::os::unix::fs::PermissionsExt;

    let run = Run::new();

    let outcome = run.run(&["config"]);

    assert!(outcome.status.success(), "{}", complaint(&outcome));
    let saved = run.home.path().join("badgehub").join("config.json");
    let mode = saved.metadata().unwrap().permissions().mode();
    assert_eq!(0o600, mode & 0o777);
}

#[test]
fn it_scaffolds_into_the_directory_you_are_standing_in() {
    let run = Run::new();

    run.scaffold(&[]);

    // Not a directory named for the slug holding another one: the working
    // directory is the project root, and the slug names only the app directory.
    assert!(run.app().join("metadata.json").is_file());
    assert!(!run.app().join(SLUG).exists());
}

#[test]
fn a_directory_holding_anything_at_all_is_refused() {
    let run = Run::new();
    write(run.root().join("notes.txt"), "mine\n").unwrap();

    let outcome = run.scaffold(&[]);

    assert!(!outcome.status.success());
    assert!(
        complaint(&outcome).contains("not empty"),
        "{}",
        complaint(&outcome)
    );
    assert!(!run.app().exists());
}

/// The files GitHub starts a repository with are the likeliest accident, and
/// they are refused like anything else.
#[test]
fn a_directory_holding_only_a_readme_is_refused_too() {
    let run = Run::new();
    write(run.root().join("README.md"), "# demo\n").unwrap();

    let outcome = run.scaffold(&[]);

    assert!(!outcome.status.success());
    assert!(
        complaint(&outcome).contains("README.md"),
        "{}",
        complaint(&outcome)
    );
}

#[test]
fn a_named_directory_is_made_and_scaffolded_into() {
    let run = Run::new();

    let outcome = run.scaffold_into(&["hwtest"], &[]);

    assert!(outcome.status.success(), "{}", complaint(&outcome));
    assert!(
        run.root()
            .join("hwtest")
            .join(SLUG)
            .join("metadata.json")
            .is_file()
    );
    assert!(run.root().join("hwtest").join("README.md").is_file());
}

#[test]
fn a_named_directory_may_be_nested_and_is_made_all_the_way_down() {
    let run = Run::new();

    let outcome = run.scaffold_into(&["projects/badges/hwtest"], &[]);

    assert!(outcome.status.success(), "{}", complaint(&outcome));
    assert!(
        run.root()
            .join("projects/badges/hwtest")
            .join(SLUG)
            .join("metadata.json")
            .is_file()
    );
}

#[test]
fn an_absolute_directory_is_taken_as_it_is() {
    let run = Run::new();
    let elsewhere = TempDir::new().unwrap();
    let named = elsewhere.path().join("hwtest");

    let outcome = run.scaffold_into(&[named.to_str().unwrap()], &[]);

    assert!(outcome.status.success(), "{}", complaint(&outcome));
    assert!(named.join(SLUG).join("metadata.json").is_file());
    assert!(!run.root().join(SLUG).exists());
}

#[test]
fn a_named_directory_that_is_not_empty_is_refused_too() {
    let run = Run::new();
    create_dir_all(run.root().join("hwtest")).unwrap();
    write(run.root().join("hwtest").join("notes.txt"), "mine\n").unwrap();

    let outcome = run.scaffold_into(&["hwtest"], &[]);

    assert!(!outcome.status.success());
    assert!(
        complaint(&outcome).contains("not empty"),
        "{}",
        complaint(&outcome)
    );
}
