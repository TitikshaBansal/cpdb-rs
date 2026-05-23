//! Paper sizes and margins returned by `GetAllOptions`.

#[cfg(feature = "zbus-backend")]
use crate::proxy::RawMedia;

#[derive(Debug, Clone)]
pub struct MarginInfo {
    pub left: i32,
    pub right: i32,
    pub top: i32,
    pub bottom: i32,
}

#[derive(Debug, Clone)]
pub struct MediaInfo {
    pub name: String,
    pub width: i32,
    pub length: i32,
    pub margins: Vec<MarginInfo>,
}

#[derive(Debug, Clone, Default)]
pub struct MediaCollection {
    pub media: Vec<MediaInfo>,
}

impl MediaCollection {
    /// Build from D-Bus response (the `Vec<RawMedia>` from GetAllOptions)
    #[cfg(feature = "zbus-backend")]
    pub fn from_dbus(raw: Vec<RawMedia>) -> Self {
        let media = raw
            .into_iter()
            .map(|r| MediaInfo {
                name: r.name,
                width: r.width,
                length: r.length,
                margins: r
                    .margins
                    .into_iter()
                    .map(|m| MarginInfo {
                        left: m.left,
                        right: m.right,
                        top: m.top,
                        bottom: m.bottom,
                    })
                    .collect(),
            })
            .collect();
        Self { media }
    }

    /// Returns the number of media entries.
    pub fn len(&self) -> usize {
        self.media.len()
    }

    /// Returns `true` if the collection contains no media entries.
    pub fn is_empty(&self) -> bool {
        self.media.is_empty()
    }

    /// Finds a media entry by name (e.g. `"iso_a4_210x297mm"`).
    pub fn get(&self, name: &str) -> Option<&MediaInfo> {
        self.media.iter().find(|m| m.name == name)
    }

    /// Returns an iterator over all media entries.
    pub fn iter(&self) -> impl Iterator<Item = &MediaInfo> {
        self.media.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_media() -> MediaCollection {
        MediaCollection {
            media: vec![
                MediaInfo {
                    name: "iso_a4_210x297mm".to_string(),
                    width: 21000,
                    length: 29700,
                    margins: vec![MarginInfo {
                        left: 500,
                        right: 500,
                        top: 500,
                        bottom: 500,
                    }],
                },
                MediaInfo {
                    name: "na_letter_8.5x11in".to_string(),
                    width: 21590,
                    length: 27940,
                    margins: vec![],
                },
            ],
        }
    }

    #[test]
    fn empty_collection() {
        let col = MediaCollection::default();
        assert!(col.is_empty());
        assert_eq!(col.len(), 0);
        assert!(col.get("iso_a4_210x297mm").is_none());
    }

    #[test]
    fn len_and_is_empty() {
        let col = sample_media();
        assert!(!col.is_empty());
        assert_eq!(col.len(), 2);
    }

    #[test]
    fn get_finds_by_name() {
        let col = sample_media();
        let a4 = col.get("iso_a4_210x297mm");
        assert!(a4.is_some());
        let a4 = a4.unwrap();
        assert_eq!(a4.width, 21000);
        assert_eq!(a4.length, 29700);
        assert_eq!(a4.margins.len(), 1);
        assert_eq!(a4.margins[0].left, 500);
    }

    #[test]
    fn get_returns_none_for_missing() {
        let col = sample_media();
        assert!(col.get("nonexistent_paper").is_none());
    }

    #[test]
    fn iter_yields_all_entries() {
        let col = sample_media();
        let names: Vec<&str> = col.iter().map(|m| m.name.as_str()).collect();
        assert!(names.contains(&"iso_a4_210x297mm"));
        assert!(names.contains(&"na_letter_8.5x11in"));
    }

    #[test]
    fn media_without_margins() {
        let col = sample_media();
        let letter = col.get("na_letter_8.5x11in").unwrap();
        assert!(letter.margins.is_empty());
    }

    #[cfg(feature = "zbus-backend")]
    #[test]
    fn from_dbus_empty_vec() {
        let col = MediaCollection::from_dbus(vec![]);
        assert!(col.is_empty());
    }

    #[cfg(feature = "zbus-backend")]
    #[test]
    fn from_dbus_converts_correctly() {
        use crate::proxy::{RawMargin, RawMedia};

        let raw = vec![RawMedia {
            name: "iso_a4_210x297mm".to_string(),
            width: 21000,
            length: 29700,
            num_margins: 1,
            margins: vec![RawMargin {
                left: 500,
                right: 500,
                top: 300,
                bottom: 300,
            }],
        }];

        let col = MediaCollection::from_dbus(raw);
        assert_eq!(col.len(), 1);
        let a4 = &col.media[0];
        assert_eq!(a4.name, "iso_a4_210x297mm");
        assert_eq!(a4.width, 21000);
        assert_eq!(a4.margins.len(), 1);
        assert_eq!(a4.margins[0].top, 300);
    }
}
