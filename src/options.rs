//! Printer options (capabilities) returned by `GetAllOptions`.
//!
//! [`OptionsCollection`] provides an owned, framework-agnostic snapshot
//! of a printer's supported settings (copies, duplex, color mode, etc.).

#[cfg(feature = "ffi")]
use crate::error::{CpdbError, Result};
#[cfg(feature = "ffi")]
use crate::ffi::bindings as ffi;
#[cfg(feature = "ffi")]
use crate::ffi::util;
#[cfg(feature = "ffi")]
use glib_sys::{GHashTableIter, g_hash_table_iter_init, g_hash_table_iter_next};
#[cfg(feature = "ffi")]
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
/// Built from D-Bus `GetAllOptions` responses (zbus backend) or from
/// C `cpdb_options_t` pointers (FFI backend). After construction, no raw
/// pointers are held - the collection is freely movable and cloneable.
///
/// # Example
///
/// ```rust
/// use cpdb_rs::options::{OptionsCollection, OptionInfo};
///
/// let col = OptionsCollection {
///     options: vec![OptionInfo {
///         name: "copies".to_string(),
///         default_value: "1".to_string(),
///         group: "General".to_string(),
///         supported_values: vec!["1".to_string(), "2".to_string()],
///     }],
/// };
/// assert_eq!(col.get("copies").unwrap().default_value, "1");
/// ```
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
    ///
    /// `raw` must be either null or a valid pointer to a fully initialised
    /// `cpdb_options_t` whose `table` field is a valid `GHashTable*`.
    #[cfg(feature = "ffi")]
    pub unsafe fn from_raw(raw: *mut ffi::cpdb_options_t) -> Result<Self> {
        if raw.is_null() {
            return Err(CpdbError::NullPointer);
        }

        // SAFETY: raw is non-null and caller guarantees validity.
        let table = unsafe { (*raw).table };

        if table.is_null() {
            return Ok(Self::default());
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

                // Copy each field into an owned String. Null fields become
                // empty strings so callers never need to check for None.
                let name = util::cstr_to_string((*opt).option_name).unwrap_or_default();
                let default_value = util::cstr_to_string((*opt).default_value).unwrap_or_default();
                let group = util::cstr_to_string((*opt).group_name).unwrap_or_default();

                // supported_values is a char** array of length num_supported.
                let mut supported_values: Vec<String> =
                    Vec::with_capacity((*opt).num_supported as usize);

                if !(*opt).supported_values.is_null() && (*opt).num_supported > 0 {
                    for i in 0..((*opt).num_supported as usize) {
                        let s_ptr = *(*opt).supported_values.add(i);
                        if !s_ptr.is_null() {
                            if let Ok(s) = util::cstr_to_string(s_ptr) {
                                supported_values.push(s);
                            }
                        }
                    }
                }

                options.push(OptionInfo {
                    name,
                    default_value,
                    group,
                    supported_values,
                });
            }
        }

        Ok(Self { options })
    }

    /// Builds an `OptionsCollection` from the D-Bus response tuples returned
    /// by [`crate::proxy::PrintBackendProxy::get_all_options()`].
    #[cfg(feature = "zbus-backend")]
    pub fn from_dbus(raw: Vec<crate::proxy::RawOption>) -> Self {
        let options = raw
            .into_iter()
            .map(|r| OptionInfo {
                name: r.option_name,
                group: r.group_name,
                default_value: r.default_value,
                supported_values: r.supported_values.into_iter().map(|(s,)| s).collect(),
            })
            .collect();
        Self { options }
    }

    /// Returns the number of options in this collection.
    #[inline]
    pub fn len(&self) -> usize {
        self.options.len()
    }

    /// Returns `true` if this collection has no options.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.options.is_empty()
    }

    /// Finds an option by name (linear search).
    pub fn get(&self, name: &str) -> Option<&OptionInfo> {
        self.options.iter().find(|o| o.name == name)
    }

    /// Returns an iterator over all options.
    pub fn iter(&self) -> impl Iterator<Item = &OptionInfo> {
        self.options.iter()
    }

    /// Builds an `OptionsCollection` from the D-Bus response tuples returned
    /// by [`PrintBackendProxy::get_all_options()`].
    ///
    /// This is the zbus equivalent of [`from_raw()`](#method.from_raw) — same
    /// output struct, different input source.
    #[cfg(feature = "zbus-backend")]
    pub fn from_dbus(raw: Vec<crate::proxy::RawOption>) -> Self {
        let options = raw
            .into_iter()
            .map(|r| OptionInfo {
                name: r.option_name,
                group: r.group_name,
                default_value: r.default_value,
                supported_values: r.supported_values.into_iter().map(|(s,)| s).collect(),
            })
            .collect();
        Self { options }
    }
}

// ─── Unit tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "ffi")]
    #[test]
    fn null_pointer_returns_null_pointer_error() {
        let result = unsafe { OptionsCollection::from_raw(std::ptr::null_mut()) };
        assert!(
            matches!(result, Err(CpdbError::NullPointer)),
            "expected NullPointer, got {:?}",
            result
        );
    }

    #[cfg(feature = "ffi")]
    #[test]
    fn null_table_returns_empty_collection() {
        let opts = ffi::cpdb_options_t {
            table: std::ptr::null_mut(),
            media: std::ptr::null_mut(),
            count: 0,
            media_count: 0,
        };
        let result =
            unsafe { OptionsCollection::from_raw(&opts as *const _ as *mut ffi::cpdb_options_t) };
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
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
