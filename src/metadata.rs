use serde::Serialize;

use crate::answers::NewProject;
use crate::json_document::JsonDocument;
use crate::repository_url::RepositoryUrl;
use crate::set::Changes;

/// The BadgeHub store listing, served at
/// `/projects/{slug}/latest/files/metadata.json`. Shape follows
/// `AppMetadataJSON.ts` in badgehub-app, where every field is optional by
/// design — so anything the user skipped is omitted rather than sent as null.
#[derive(Serialize)]
pub struct Metadata {
    project_type: String,
    name: String,
    description: String,
    author: String,
    version: String,
    categories: Vec<String>,
    badges: Vec<String>,
    application: Vec<Variant>,
    #[serde(skip_serializing_if = "Option::is_none")]
    git_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    license_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    license_file: Option<String>,
}

#[derive(Serialize)]
struct Variant {
    #[serde(rename = "type")]
    runtime: String,
    executable: String,
}

impl Metadata {
    pub fn describing(project: &NewProject) -> Self {
        Self {
            project_type: project.project_type.to_string(),
            name: project.name.to_string(),
            description: project.description.to_string(),
            author: project.author.to_string(),
            version: project.version.to_string(),
            categories: project.categories.clone(),
            badges: project.badges.clone(),
            application: vec![Variant {
                runtime: "micropython".to_owned(),
                executable: NewProject::entrypoint().to_owned(),
            }],
            git_url: project.git_url.as_ref().map(RepositoryUrl::to_string),
            license_type: project.license_type.clone(),
            license_file: project.license_type.as_ref().map(|_| "LICENSE".to_owned()),
        }
    }

    /// The field names live here, beside the shape they belong to, so an edit
    /// and a fresh scaffold can never disagree about what a field is called.
    pub fn amend(document: &mut JsonDocument, changes: &Changes) {
        for (key, given) in [
            ("name", &changes.name),
            ("author", &changes.author),
            ("description", &changes.description),
        ] {
            let Some(value) = given else { continue };
            document.set(key, value);
        }
    }
}
