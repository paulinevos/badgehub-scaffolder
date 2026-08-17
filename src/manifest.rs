use serde::Serialize;

use crate::answers::NewProject;
use crate::json_document::JsonDocument;
use crate::set::Changes;

/// MicroPythonOS's `MANIFEST.JSON` — a different file, owned by a different
/// system than `metadata.json`. BadgeHub never reads it; the on-badge launcher
/// does. The overlapping field names are a coincidence of both describing an
/// app, so the two shapes stay separate.
#[derive(Serialize)]
pub struct Manifest {
    name: String,
    publisher: String,
    short_description: String,
    fullname: String,
    version: String,
    activities: Vec<Activity>,
}

#[derive(Serialize)]
struct Activity {
    entrypoint: String,
    classname: String,
    intent_filters: Vec<IntentFilter>,
}

#[derive(Serialize)]
struct IntentFilter {
    action: String,
    category: String,
}

impl Manifest {
    pub fn describing(project: &NewProject) -> Self {
        Self {
            name: project.name.to_string(),
            publisher: project.author.to_string(),
            short_description: project.description.to_string(),
            fullname: project.slug.to_string(),
            version: project.version.to_string(),
            activities: vec![Activity {
                entrypoint: NewProject::entrypoint().to_owned(),
                classname: project.slug.suggested_class_name(),
                intent_filters: vec![IntentFilter {
                    action: "main".to_owned(),
                    category: "launcher".to_owned(),
                }],
            }],
        }
    }

    /// The same three answers metadata.json holds, under the names
    /// MicroPythonOS gives them. There is no licence here: the launcher has no
    /// field for one.
    pub fn amend(document: &mut JsonDocument, changes: &Changes) {
        for (key, given) in [
            ("name", &changes.name),
            ("publisher", &changes.author),
            ("short_description", &changes.description),
        ] {
            let Some(value) = given else { continue };
            document.set(key, value);
        }
    }
}
