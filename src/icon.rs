use std::collections::BTreeMap;
use std::fs::File;
use std::io::BufWriter;
use std::path::Path;

use anyhow::{Context, Result};
use serde::Serialize;

use crate::slug::Slug;

/// One edge of a square icon, in pixels. `AppMetadataJSON.ts` admits 8x8,
/// 16x16, 32x32 and 64x64, but 64x64 is the only one anything renders: the
/// frontend's `AppCard` reads `icon_map["64x64"]` with no fallback, and so does
/// MicroPythonOS's app store when it maps a BadgeHub project. 32x32 comes along
/// because the backend's blur-hash preference chain falls back through it;
/// 16x16 and 8x8 are below the size at which a generated pattern is anything
/// but mud. The file names match what badgehub-app's own icon endpoint writes,
/// so a later upload lands on top of these rather than beside them.
#[derive(Clone, Copy)]
pub struct IconSize(u32);

impl IconSize {
    pub const SCAFFOLDED: [IconSize; 2] = [IconSize(32), IconSize(64)];

    fn label(self) -> String {
        format!("{edge}x{edge}", edge = self.0)
    }

    pub fn file_name(self) -> String {
        format!("icon-{}.png", self.label())
    }

    fn cell_edge(self) -> u32 {
        self.0 / CELLS_ACROSS
    }
}

/// The `icon_map` of the store listing: sizes to paths, relative to the app
/// directory the metadata itself sits in.
#[derive(Serialize)]
pub struct IconMap(BTreeMap<String, String>);

impl IconMap {
    pub fn of_placeholders() -> Self {
        Self(
            IconSize::SCAFFOLDED
                .into_iter()
                .map(|size| (size.label(), size.file_name()))
                .collect(),
        )
    }
}

/// A checkerboard scrambled by the slug. Deliberately garish and obviously
/// machine-made: an author should feel prodded to replace it, never tempted to
/// ship it.
pub struct PlaceholderIcon {
    seed: IconSeed,
}

const CELLS_ACROSS: u32 = 8;

impl PlaceholderIcon {
    pub fn for_slug(slug: &Slug) -> Self {
        Self {
            seed: IconSeed::from_slug(slug),
        }
    }

    pub fn write_into(&self, directory: &Path) -> Result<()> {
        for size in IconSize::SCAFFOLDED {
            self.write_one(directory, size)?;
        }
        Ok(())
    }

    fn write_one(&self, directory: &Path, size: IconSize) -> Result<()> {
        let path = directory.join(size.file_name());
        let file = File::create(&path).with_context(|| format!("writing {}", path.display()))?;
        let mut encoder = png::Encoder::new(BufWriter::new(file), size.0, size.0);
        encoder.set_color(png::ColorType::Rgb);
        encoder.set_depth(png::BitDepth::Eight);
        encoder
            .write_header()
            .and_then(|mut writer| writer.write_image_data(&self.pixels(size)))
            .with_context(|| format!("encoding {}", path.display()))
    }

    fn pixels(&self, size: IconSize) -> Vec<u8> {
        (0..size.0)
            .flat_map(|row| (0..size.0).map(move |column| (row, column)))
            .flat_map(|(row, column)| {
                self.colour_at(row / size.cell_edge(), column / size.cell_edge())
            })
            .collect()
    }

    fn colour_at(&self, cell_row: u32, cell_column: u32) -> [u8; 3] {
        let checkered = (cell_row + cell_column) % 2 == 1;
        let scrambled = self.seed.bit(cell_row * CELLS_ACROSS + cell_column);
        self.seed.colour(checkered != scrambled)
    }
}

/// A hash of the slug, so the same project always draws the same icon. FNV-1a
/// by hand rather than `DefaultHasher`, whose output std explicitly refuses to
/// keep stable across releases — an icon that changed under a compiler upgrade
/// would be a diff nobody asked for.
struct IconSeed(u64);

const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

/// Loud, flat and unbalanced on purpose; no pair of these reads as a designed
/// mark.
const PALETTE: [[u8; 3]; 8] = [
    [255, 0, 128],
    [0, 200, 255],
    [255, 210, 0],
    [130, 0, 255],
    [0, 220, 120],
    [255, 90, 0],
    [0, 90, 255],
    [220, 0, 40],
];

impl IconSeed {
    fn from_slug(slug: &Slug) -> Self {
        let hashed = slug
            .to_string()
            .bytes()
            .fold(FNV_OFFSET, |accumulated, byte| {
                (accumulated ^ u64::from(byte)).wrapping_mul(FNV_PRIME)
            });
        Self(hashed)
    }

    fn bit(&self, position: u32) -> bool {
        self.0 >> (position % 64) & 1 == 1
    }

    fn colour(&self, foreground: bool) -> [u8; 3] {
        let first = (self.0 % PALETTE.len() as u64) as usize;
        // A fixed odd stride keeps the two colours distinct whatever the seed.
        let second = (first + 3) % PALETTE.len();
        PALETTE[if foreground { first } else { second }]
    }
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::{IconMap, IconSize, PlaceholderIcon};
    use crate::slug::Slug;

    fn drawn(slug: &str) -> Vec<u8> {
        PlaceholderIcon::for_slug(&Slug::parse(slug).unwrap()).pixels(IconSize(32))
    }

    #[test]
    fn the_same_slug_always_draws_the_same_icon() {
        assert_eq!(drawn("org.fri3d.hwtest"), drawn("org.fri3d.hwtest"));
    }

    #[test]
    fn a_different_slug_draws_a_different_icon() {
        assert_ne!(drawn("org.fri3d.hwtest"), drawn("nl.paulinevos.agenda"));
    }

    /// A seed that painted every cell one colour would look designed rather
    /// than generated, which is the one thing this must not do.
    #[test]
    fn an_icon_is_never_a_single_flat_colour() {
        let painted = drawn("org.fri3d.hwtest");
        let first = &painted[..3];

        assert!(painted.chunks(3).any(|pixel| pixel != first));
    }

    #[test]
    fn every_scaffolded_size_lands_beside_the_metadata() {
        let app = TempDir::new().unwrap();

        PlaceholderIcon::for_slug(&Slug::parse("org.fri3d.hwtest").unwrap())
            .write_into(app.path())
            .unwrap();

        assert!(app.path().join("icon-32x32.png").exists());
        assert!(app.path().join("icon-64x64.png").exists());
    }

    #[test]
    fn what_is_written_decodes_as_a_png_of_the_size_it_claims() {
        let app = TempDir::new().unwrap();

        PlaceholderIcon::for_slug(&Slug::parse("org.fri3d.hwtest").unwrap())
            .write_into(app.path())
            .unwrap();

        for size in IconSize::SCAFFOLDED {
            let file = std::fs::File::open(app.path().join(size.file_name())).unwrap();
            let reader = png::Decoder::new(std::io::BufReader::new(file))
                .read_info()
                .unwrap();
            assert_eq!(size.0, reader.info().width);
            assert_eq!(size.0, reader.info().height);
        }
    }

    #[test]
    fn the_map_names_every_size_it_draws() {
        let named = serde_json::to_value(IconMap::of_placeholders()).unwrap();

        assert_eq!("icon-32x32.png", named["32x32"]);
        assert_eq!("icon-64x64.png", named["64x64"]);
    }
}
