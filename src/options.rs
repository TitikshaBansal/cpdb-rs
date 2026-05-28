//! Owned, value-type snapshots of a printer's options.
//!
//! These types decouple Rust code from the lifetime of a `cpdb_options_t`
//! by copying every field into owned Rust storage at construction.

use crate::ffi;
use crate::util;
use glib_sys::{GHashTableIter, g_hash_table_iter_init, g_hash_table_iter_next};
use std::mem::MaybeUninit;
use std::ptr::NonNull;

/// A single printer option with its supported choices.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OptionInfo {
    /// The option name, e.g. `"copies"` or `"sides"`.
    pub name: String,
    /// The default value as reported by the backend.
    pub default_value: String,
    /// The option group (e.g. `"General"`), or an empty string when unset.
    pub group: String,
    /// All values the printer supports for this option.
    pub supported_values: Vec<String>,
}

/// An owned snapshot of every option in a `cpdb_options_t`.
///
/// Built by iterating `cpdb_options_t.table` once and copying every field
/// into Rust-owned `String`s. After construction the collection holds no
/// raw pointers and can be freely stored, cloned, or sent across threads.
#[derive(Debug, Clone, Default)]
pub struct OptionsCollection {
    /// Every option discovered, in iteration order of the underlying
    /// hash table (which itself is implementation-defined).
    pub options: Vec<OptionInfo>,
}

impl OptionsCollection {
    /// Builds an [`OptionsCollection`] by iterating `raw.table`.
    ///
    /// All string data is copied into owned Rust types inside this call;
    /// after it returns the pointer is no longer accessed.
    ///
    /// # Safety
    /// `raw` must point at a fully initialised `cpdb_options_t` whose
    /// `table` field is null or a valid `GHashTable*` of
    /// `*mut cpdb_option_t` values.
    pub unsafe fn from_raw(raw: NonNull<ffi::cpdb_options_t>) -> Self {
        // SAFETY: caller guarantees `raw` is valid.
        let table = unsafe { (*raw.as_ptr()).table };
        if table.is_null() {
            return Self::default();
        }

        let mut options: Vec<OptionInfo> = Vec::new();

        // SAFETY: we initialise the iterator on the stack and iterate the
        // table synchronously, copying all data into owned Strings before
        // returning. Pointers obtained from `g_hash_table_iter_next` are
        // borrowed into the table and must NOT be freed.
        unsafe {
            let mut iter = MaybeUninit::<GHashTableIter>::uninit();
            g_hash_table_iter_init(iter.as_mut_ptr(), table as *mut glib_sys::GHashTable);
            let mut iter = iter.assume_init();

            let mut key: *mut libc::c_void = std::ptr::null_mut();
            let mut value: *mut libc::c_void = std::ptr::null_mut();

            while g_hash_table_iter_next(&mut iter, &mut key, &mut value) != 0 {
                if value.is_null() {
                    continue;
                }
                let opt = value as *mut ffi::cpdb_option_t;
                options.push(option_info_from_raw(opt));
            }
        }

        Self { options }
    }

    /// Number of options in the collection.
    #[inline]
    pub fn len(&self) -> usize {
        self.options.len()
    }

    /// `true` when the collection is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.options.is_empty()
    }

    /// Returns the option with the given name, if any.
    pub fn get(&self, name: &str) -> Option<&OptionInfo> {
        self.options.iter().find(|o| o.name == name)
    }

    /// Iterates over every option.
    pub fn iter(&self) -> impl Iterator<Item = &OptionInfo> {
        self.options.iter()
    }
}

/// Copies one `cpdb_option_t` into an owned [`OptionInfo`].
///
/// # Safety
/// `opt` must be a valid pointer into a live `cpdb_option_t` whose string
/// fields are NUL-terminated and whose `supported_values` array (if any)
/// has at least `num_supported` valid entries.
unsafe fn option_info_from_raw(opt: *mut ffi::cpdb_option_t) -> OptionInfo {
    let name = unsafe { util::cstr_to_string((*opt).option_name) }.unwrap_or_default();
    let default_value = unsafe { util::cstr_to_string((*opt).default_value) }.unwrap_or_default();
    let group = unsafe { util::cstr_to_string((*opt).group_name) }.unwrap_or_default();

    let mut supported_values: Vec<String> =
        Vec::with_capacity(unsafe { (*opt).num_supported } as usize);

    let arr = unsafe { (*opt).supported_values };
    let count = unsafe { (*opt).num_supported };
    if !arr.is_null() && count > 0 {
        for i in 0..(count as usize) {
            let s_ptr = unsafe { *arr.add(i) };
            if !s_ptr.is_null()
                && let Ok(s) = unsafe { util::cstr_to_string(s_ptr) }
            {
                supported_values.push(s);
            }
        }
    }

    OptionInfo {
        name,
        default_value,
        group,
        supported_values,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn null_table_returns_empty_collection() {
        let opts = ffi::cpdb_options_t {
            table: std::ptr::null_mut(),
            media: std::ptr::null_mut(),
            count: 0,
            media_count: 0,
        };
        let raw = NonNull::from(&opts).cast::<ffi::cpdb_options_t>();
        let result = unsafe { OptionsCollection::from_raw(raw) };
        assert!(result.is_empty());
    }

    #[test]
    fn empty_collection_helpers() {
        let col = OptionsCollection::default();
        assert!(col.is_empty());
        assert_eq!(col.len(), 0);
        assert!(col.get("copies").is_none());
    }

    #[test]
    fn collection_get_finds_by_name() {
        let col = OptionsCollection {
            options: vec![
                OptionInfo {
                    name: "copies".to_string(),
                    default_value: "1".to_string(),
                    group: "General".to_string(),
                    supported_values: vec!["1".to_string(), "2".to_string()],
                },
                OptionInfo {
                    name: "sides".to_string(),
                    default_value: "one-sided".to_string(),
                    group: "General".to_string(),
                    supported_values: vec![
                        "one-sided".to_string(),
                        "two-sided-long-edge".to_string(),
                    ],
                },
            ],
        };
        let found = col.get("sides");
        assert_eq!(found.unwrap().default_value, "one-sided");
        assert!(col.get("nonexistent").is_none());
    }

    #[test]
    fn collection_len_and_iter() {
        let col = OptionsCollection {
            options: vec![
                OptionInfo {
                    name: "a".into(),
                    default_value: String::new(),
                    group: String::new(),
                    supported_values: vec![],
                },
                OptionInfo {
                    name: "b".into(),
                    default_value: String::new(),
                    group: String::new(),
                    supported_values: vec![],
                },
            ],
        };
        assert_eq!(col.len(), 2);
        assert_eq!(col.iter().count(), 2);
    }
}
