use std::fs::{read_to_string, write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde_json::{Map, Value};

/// A JSON file edited in place. Everything it does not recognise is carried
/// through untouched: BadgeHub adds fields over time, and people hand-edit
/// these files, and neither should be lost because this tool changed a name.
#[derive(Debug)]
pub struct JsonDocument {
    path: PathBuf,
    fields: Map<String, Value>,
}

impl JsonDocument {
    pub fn read(path: &Path) -> Result<Self> {
        let contents =
            read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
        let parsed: Value = serde_json::from_str(&contents)
            .with_context(|| format!("reading {}", path.display()))?;
        let Value::Object(fields) = parsed else {
            bail!("{} is not a JSON object", path.display());
        };
        Ok(Self {
            path: path.to_owned(),
            fields,
        })
    }

    pub fn set(&mut self, key: &str, value: &str) {
        self.fields
            .insert(key.to_owned(), Value::String(value.to_owned()));
    }

    pub fn text_at(&self, key: &str) -> Option<String> {
        self.fields.get(key)?.as_str().map(str::to_owned)
    }

    /// Same shape `Scaffold::put_json` writes, so a file this has touched is
    /// indistinguishable from a freshly scaffolded one.
    pub fn write_out(&self) -> Result<()> {
        let rendered = serde_json::to_string_pretty(&self.fields)?;
        write(&self.path, format!("{rendered}\n"))
            .with_context(|| format!("writing {}", self.path.display()))
    }
}

#[cfg(test)]
mod tests {
    use std::fs::{read_to_string, write};

    use tempfile::TempDir;

    use super::JsonDocument;

    fn holding(contents: &str) -> (TempDir, std::path::PathBuf) {
        let directory = TempDir::new().unwrap();
        let path = directory.path().join("metadata.json");
        write(&path, contents).unwrap();
        (directory, path)
    }

    #[test]
    fn a_known_key_is_replaced() {
        let (_kept, path) = holding(r#"{"name": "Old"}"#);

        let mut document = JsonDocument::read(&path).unwrap();
        document.set("name", "New");
        document.write_out().unwrap();

        assert_eq!(Some("New".to_owned()), document.text_at("name"));
        assert!(read_to_string(&path).unwrap().contains("\"New\""));
    }

    #[test]
    fn a_key_nobody_here_knows_about_survives() {
        let (_kept, path) = holding(r#"{"name": "Old", "published_at": "yesterday"}"#);

        let mut document = JsonDocument::read(&path).unwrap();
        document.set("name", "New");
        document.write_out().unwrap();

        let written = read_to_string(&path).unwrap();
        assert!(written.contains("published_at"), "{written}");
        assert!(written.contains("yesterday"), "{written}");
    }

    #[test]
    fn a_missing_key_reads_as_nothing_rather_than_failing() {
        let (_kept, path) = holding(r#"{"name": "Old"}"#);

        let document = JsonDocument::read(&path).unwrap();

        assert_eq!(None, document.text_at("author"));
    }

    #[test]
    fn what_is_written_is_pretty_printed_and_ends_in_a_newline() {
        let (_kept, path) = holding(r#"{"name":"Old","author":"Someone"}"#);

        JsonDocument::read(&path).unwrap().write_out().unwrap();

        let written = read_to_string(&path).unwrap();
        assert!(written.starts_with("{\n  \"name\""), "{written}");
        assert!(written.ends_with("}\n"), "{written}");
    }

    /// Without serde_json's preserve_order, every edit would sort the file and
    /// show up as a diff nobody asked for.
    #[test]
    fn the_keys_stay_where_the_user_left_them() {
        let (_kept, path) = holding(r#"{"zebra": "last", "author": "first"}"#);

        let mut document = JsonDocument::read(&path).unwrap();
        document.set("author", "changed");
        document.write_out().unwrap();

        let written = read_to_string(&path).unwrap();
        assert!(written.find("zebra") < written.find("author"), "{written}");
    }

    #[test]
    fn a_file_that_is_not_json_is_refused() {
        let (_kept, path) = holding("<html>not json</html>");

        assert!(JsonDocument::read(&path).is_err());
    }

    #[test]
    fn a_json_list_is_refused() {
        let (_kept, path) = holding("[1, 2, 3]");

        let refused = JsonDocument::read(&path).unwrap_err().to_string();

        assert!(refused.contains("not a JSON object"), "{refused}");
    }
}
