//! The GitHub client, against a local mock standing in for api.github.com.
//! Response bodies are real recordings — see `tests/fixtures/README.md`.

use badgehub::github::{GithubApi, NewRepository, Visibility};
use badgehub::settings::GithubToken;
use mockito::{Matcher, Mock, Server, ServerGuard};
use serde_json::json;

const CREATED: &str = include_str!("fixtures/created_repository.json");
const VALIDATION_FAILED: &str = include_str!("fixtures/validation_failed.json");

fn hwtest() -> NewRepository {
    NewRepository::named("hwtest", "Checks the badge", Visibility::Public)
}

fn token() -> GithubToken {
    GithubToken::parse("ghp_pretend").unwrap()
}

fn answering(server: &mut ServerGuard, status: usize, body: &str) -> Mock {
    server
        .mock("POST", "/user/repos")
        .with_status(status)
        .with_header("content-type", "application/json")
        .with_body(body)
        .create()
}

#[test]
fn a_created_repository_yields_its_clone_url() {
    let mut server = Server::new();
    let mocked = answering(&mut server, 201, CREATED);

    let url = GithubApi::at(&server.url())
        .create(&hwtest(), &token())
        .unwrap();

    mocked.assert();
    assert_eq!(
        "https://github.com/octokit-fixture-org/rename-repository-newname.git",
        url.to_string()
    );
}

#[test]
fn the_request_carries_the_name_description_and_visibility() {
    let mut server = Server::new();
    let mocked = server
        .mock("POST", "/user/repos")
        .match_header("authorization", "Bearer ghp_pretend")
        .match_header("accept", "application/vnd.github+json")
        .match_body(Matcher::Json(json!({
            "name": "hwtest",
            "description": "Checks the badge",
            "private": false,
        })))
        .with_status(201)
        .with_body(CREATED)
        .create();

    GithubApi::at(&server.url())
        .create(&hwtest(), &token())
        .unwrap();

    mocked.assert();
}

#[test]
fn a_private_repository_is_asked_for_as_private() {
    let mut server = Server::new();
    let mocked = server
        .mock("POST", "/user/repos")
        .match_body(Matcher::PartialJson(json!({ "private": true })))
        .with_status(201)
        .with_body(CREATED)
        .create();

    let private = NewRepository::named("hwtest", "Checks the badge", Visibility::Private);
    GithubApi::at(&server.url())
        .create(&private, &token())
        .unwrap();

    mocked.assert();
}

#[test]
fn a_rejected_token_says_so_rather_than_carrying_on() {
    let mut server = Server::new();
    answering(&mut server, 401, r#"{"message": "Bad credentials"}"#);

    let refused = GithubApi::at(&server.url()).create(&hwtest(), &token());

    let complaint = refused.unwrap_err().to_string();
    assert!(complaint.contains("rejected the token"), "{complaint}");
    assert!(complaint.contains("bh config"), "{complaint}");
}

#[test]
fn a_token_without_the_right_permission_names_the_permission() {
    let mut server = Server::new();
    answering(
        &mut server,
        403,
        r#"{"message": "Resource not accessible"}"#,
    );

    let refused = GithubApi::at(&server.url()).create(&hwtest(), &token());

    let complaint = refused.unwrap_err().to_string();
    assert!(complaint.contains("Administration"), "{complaint}");
}

#[test]
fn an_existing_repository_is_reported_as_such() {
    let mut server = Server::new();
    answering(&mut server, 422, VALIDATION_FAILED);

    let refused = GithubApi::at(&server.url()).create(&hwtest(), &token());

    let complaint = refused.unwrap_err().to_string();
    assert!(
        complaint.contains("already has a repository"),
        "{complaint}"
    );
}

#[test]
fn an_unexpected_status_still_fails_loudly() {
    let mut server = Server::new();
    answering(&mut server, 500, r#"{"message": "we broke it"}"#);

    let refused = GithubApi::at(&server.url()).create(&hwtest(), &token());

    let complaint = format!("{:#}", refused.unwrap_err());
    assert!(complaint.contains("asking GitHub"), "{complaint}");
}

#[test]
fn an_answer_without_a_clone_url_is_not_treated_as_success() {
    let mut server = Server::new();
    answering(&mut server, 201, r#"{"id": 1, "name": "hwtest"}"#);

    let refused = GithubApi::at(&server.url()).create(&hwtest(), &token());

    let complaint = refused.unwrap_err().to_string();
    assert!(complaint.contains("no clone_url"), "{complaint}");
}

#[test]
fn a_body_that_is_not_json_is_not_treated_as_success() {
    let mut server = Server::new();
    answering(&mut server, 201, "<html>upstream is having a day</html>");

    let refused = GithubApi::at(&server.url()).create(&hwtest(), &token());

    assert!(refused.is_err());
}

#[test]
fn a_trailing_slash_on_the_base_url_does_not_double_up() {
    let mut server = Server::new();
    let mocked = answering(&mut server, 201, CREATED);

    GithubApi::at(&format!("{}/", server.url()))
        .create(&hwtest(), &token())
        .unwrap();

    mocked.assert();
}
