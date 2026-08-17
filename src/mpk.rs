use std::fs::{File, read, read_dir};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use zip::write::{SimpleFileOptions, ZipWriter};
use zip::{CompressionMethod, DateTime};

use crate::json_document::JsonDocument;

/// Never read by MicroPythonOS — the archive it installs is described entirely
/// by the entries inside it — but a fixed stamp is what keeps two builds of the
/// same source byte-identical, which is the whole point of building locally.
const FIXED_STAMP: (u16, u8, u8, u8, u8, u8) = (2025, 1, 1, 0, 0, 0);
/// Bytecode caches are a side effect of running the app, not part of it.
const NOT_PART_OF_THE_APP: [&str; 2] = ["__pycache__", ".DS_Store"];

/// A MicroPythonOS package: a stored ZIP holding exactly one top-level
/// directory named for the manifest's fullname. Per
/// <https://docs.micropythonos.com/apps/bundling-apps/>, an archive shaped any
/// other way is rejected at install time on the badge.
#[derive(Debug)]
pub struct Mpk {
    app_directory: PathBuf,
    fullname: String,
    version: String,
}

impl Mpk {
    pub fn of(app_directory: &Path, manifest: &JsonDocument) -> Result<Self> {
        let fullname = manifest
            .text_at("fullname")
            .context("MANIFEST.JSON has no fullname")?;
        let version = manifest
            .text_at("version")
            .context("MANIFEST.JSON has no version")?;
        let named = app_directory
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default();
        if named != fullname {
            bail!("the app directory must be named {fullname}, not {named}");
        }
        Ok(Self {
            app_directory: app_directory.to_owned(),
            fullname,
            version,
        })
    }

    pub fn file_name(&self) -> String {
        format!("{}_{}.mpk", self.fullname, self.version)
    }

    pub fn write_into(&self, directory: &Path) -> Result<PathBuf> {
        let path = directory.join(self.file_name());
        let file = File::create(&path).with_context(|| format!("writing {}", path.display()))?;
        self.write_to(BufWriter::new(file))
            .with_context(|| format!("writing {}", path.display()))?;
        Ok(path)
    }

    fn write_to(&self, sink: impl Write + std::io::Seek) -> Result<()> {
        let mut archive = ZipWriter::new(sink);
        archive.add_directory(format!("{}/", self.fullname), self.options())?;
        for entry in self.entries()? {
            entry.add_to(&mut archive, self.options())?;
        }
        archive.finish()?;
        Ok(())
    }

    /// Stored rather than deflated, and stamped identically, so the bytes
    /// depend on the app and nothing else.
    fn options(&self) -> SimpleFileOptions {
        let (year, month, day, hour, minute, second) = FIXED_STAMP;
        let stamp = DateTime::from_date_and_time(year, month, day, hour, minute, second)
            .expect("a fixed, valid timestamp");
        SimpleFileOptions::default()
            .compression_method(CompressionMethod::Stored)
            .last_modified_time(stamp)
    }

    /// Sorted by their path as bytes, so the order is the same wherever this
    /// runs. Reading a directory returns whatever order the filesystem feels
    /// like, which is how an archive ends up differing between two machines
    /// holding identical files.
    fn entries(&self) -> Result<Vec<Entry>> {
        let mut found = Vec::new();
        self.gather(&self.app_directory, &self.fullname, &mut found)?;
        found.sort_by(|one, other| one.path_in_archive.cmp(&other.path_in_archive));
        Ok(found)
    }

    fn gather(&self, directory: &Path, prefix: &str, found: &mut Vec<Entry>) -> Result<()> {
        let listing =
            read_dir(directory).with_context(|| format!("reading {}", directory.display()))?;
        for entry in listing {
            let entry = entry?;
            let name = entry.file_name().to_string_lossy().into_owned();
            self.gather_one(&entry.path(), &format!("{prefix}/{name}"), &name, found)?;
        }
        Ok(())
    }

    fn gather_one(
        &self,
        path: &Path,
        path_in_archive: &str,
        name: &str,
        found: &mut Vec<Entry>,
    ) -> Result<()> {
        if NOT_PART_OF_THE_APP.contains(&name) || name.ends_with(".mpk") {
            return Ok(());
        }
        if path.is_dir() {
            found.push(Entry::directory(path_in_archive));
            return self.gather(path, path_in_archive, found);
        }
        found.push(Entry::file(path, path_in_archive));
        Ok(())
    }
}

struct Entry {
    on_disk: Option<PathBuf>,
    path_in_archive: String,
}

impl Entry {
    fn directory(path_in_archive: &str) -> Self {
        Self {
            on_disk: None,
            path_in_archive: format!("{path_in_archive}/"),
        }
    }

    fn file(on_disk: &Path, path_in_archive: &str) -> Self {
        Self {
            on_disk: Some(on_disk.to_owned()),
            path_in_archive: path_in_archive.to_owned(),
        }
    }

    fn add_to(
        &self,
        archive: &mut ZipWriter<impl Write + std::io::Seek>,
        options: SimpleFileOptions,
    ) -> Result<()> {
        let Some(on_disk) = &self.on_disk else {
            archive.add_directory(&self.path_in_archive, options)?;
            return Ok(());
        };
        archive.start_file(&self.path_in_archive, options)?;
        let contents = read(on_disk).with_context(|| format!("reading {}", on_disk.display()))?;
        archive.write_all(&contents)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::fs::{create_dir_all, read, write};
    use std::path::{Path, PathBuf};

    use tempfile::TempDir;

    use super::Mpk;
    use crate::json_document::JsonDocument;

    const MANIFEST: &str = r#"{"fullname": "nl.paulinevos.demo", "version": "1.2.3"}"#;

    fn app_directory_in(kept: &TempDir, named: &str) -> PathBuf {
        let app = kept.path().join(named);
        create_dir_all(&app).unwrap();
        write(app.join("MANIFEST.JSON"), MANIFEST).unwrap();
        write(app.join("__init__.py"), "class Demo:\n").unwrap();
        write(app.join("metadata.json"), "{}\n").unwrap();
        app
    }

    fn mpk_of(app: &Path) -> Mpk {
        let manifest = JsonDocument::read(&app.join("MANIFEST.JSON")).unwrap();
        Mpk::of(app, &manifest).unwrap()
    }

    fn names_in(archive: &Path) -> Vec<String> {
        let reader = std::fs::File::open(archive).unwrap();
        let mut zip = zip::ZipArchive::new(reader).unwrap();
        (0..zip.len())
            .map(|index| zip.by_index(index).unwrap().name().to_owned())
            .collect()
    }

    #[test]
    fn it_is_named_for_the_fullname_and_version() {
        let kept = TempDir::new().unwrap();
        let app = app_directory_in(&kept, "nl.paulinevos.demo");

        assert_eq!("nl.paulinevos.demo_1.2.3.mpk", mpk_of(&app).file_name());
    }

    /// MicroPythonOS rejects an archive whose one top-level directory is not
    /// the fullname, so the mismatch is worth catching before the badge does.
    #[test]
    fn an_app_directory_named_something_else_is_refused() {
        let kept = TempDir::new().unwrap();
        let app = app_directory_in(&kept, "wrong-name");
        let manifest = JsonDocument::read(&app.join("MANIFEST.JSON")).unwrap();

        let refused = Mpk::of(&app, &manifest).unwrap_err().to_string();

        assert!(
            refused.contains("must be named nl.paulinevos.demo"),
            "{refused}"
        );
    }

    #[test]
    fn the_first_entry_is_the_directory_named_for_the_fullname() {
        let kept = TempDir::new().unwrap();
        let app = app_directory_in(&kept, "nl.paulinevos.demo");

        let written = mpk_of(&app).write_into(kept.path()).unwrap();

        assert_eq!("nl.paulinevos.demo/", names_in(&written)[0]);
    }

    #[test]
    fn entries_are_sorted_by_path_rather_than_however_the_filesystem_answered() {
        let kept = TempDir::new().unwrap();
        let app = app_directory_in(&kept, "nl.paulinevos.demo");

        let written = mpk_of(&app).write_into(kept.path()).unwrap();

        let names = names_in(&written);
        let mut sorted = names.clone();
        sorted.sort();
        assert_eq!(sorted, names);
    }

    #[test]
    fn two_builds_of_the_same_source_are_byte_identical() {
        let kept = TempDir::new().unwrap();
        let app = app_directory_in(&kept, "nl.paulinevos.demo");
        let elsewhere = TempDir::new().unwrap();

        let one = mpk_of(&app).write_into(kept.path()).unwrap();
        let other = mpk_of(&app).write_into(elsewhere.path()).unwrap();

        assert_eq!(read(one).unwrap(), read(other).unwrap());
    }

    #[test]
    fn nested_directories_come_along() {
        let kept = TempDir::new().unwrap();
        let app = app_directory_in(&kept, "nl.paulinevos.demo");
        create_dir_all(app.join("lib/deep")).unwrap();
        write(app.join("lib/deep/thing.py"), "x = 1\n").unwrap();

        let written = mpk_of(&app).write_into(kept.path()).unwrap();

        let names = names_in(&written);
        assert!(
            names.contains(&"nl.paulinevos.demo/lib/".to_owned()),
            "{names:?}"
        );
        assert!(
            names.contains(&"nl.paulinevos.demo/lib/deep/thing.py".to_owned()),
            "{names:?}"
        );
    }

    #[test]
    fn bytecode_caches_are_left_out() {
        let kept = TempDir::new().unwrap();
        let app = app_directory_in(&kept, "nl.paulinevos.demo");
        create_dir_all(app.join("__pycache__")).unwrap();
        write(app.join("__pycache__/demo.pyc"), "bytecode").unwrap();

        let written = mpk_of(&app).write_into(kept.path()).unwrap();

        assert!(
            !names_in(&written)
                .iter()
                .any(|name| name.contains("pycache"))
        );
    }

    /// Bundling into the app directory would otherwise fold the last build into
    /// the next one, and the archive would grow every time.
    #[test]
    fn an_mpk_already_there_is_not_bundled_into_the_next_one() {
        let kept = TempDir::new().unwrap();
        let app = app_directory_in(&kept, "nl.paulinevos.demo");
        write(app.join("stale_0.0.1.mpk"), "an earlier build").unwrap();

        let written = mpk_of(&app).write_into(kept.path()).unwrap();

        assert!(!names_in(&written).iter().any(|name| name.ends_with(".mpk")));
    }

    #[test]
    fn what_goes_in_comes_back_out_unchanged() {
        let kept = TempDir::new().unwrap();
        let app = app_directory_in(&kept, "nl.paulinevos.demo");

        let written = mpk_of(&app).write_into(kept.path()).unwrap();

        let reader = std::fs::File::open(&written).unwrap();
        let mut zip = zip::ZipArchive::new(reader).unwrap();
        let mut entry = zip.by_name("nl.paulinevos.demo/__init__.py").unwrap();
        let mut contents = String::new();
        std::io::Read::read_to_string(&mut entry, &mut contents).unwrap();
        assert_eq!("class Demo:\n", contents);
    }
}
