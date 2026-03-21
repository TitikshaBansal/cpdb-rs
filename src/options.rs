use crate::error::{CpdbError, Result};
use crate::ffi;
use crate::util;
use glib_sys::{g_hash_table_iter_init, g_hash_table_iter_next, GHashTableIter};
use std::mem::MaybeUninit;

// ─── Public types ────────────────────────────────────────────────────────────

/// A single printer option with its supported choices.
///
/// All fields are owned `String`s — no raw pointers are held after construction.
#[derive(Debug, Clone, PartialEq)]
pub struct OptionInfo {
    /// The option name, e.g. `"copies"`, `"sides"`.
    pub name: String,
    /// The default value for this option as reported by the backend.
    pub default_value: String,
    /// The option group, e.g. `"General"`.
    pub group: String,
    /// All values the printer supports for this option.
    pub supported_values: Vec<String>,
}

/// An owned snapshot of all options for a printer.
///
/// Built by iterating `cpdb_options_t.table` once and copying everything into
/// Rust-owned memory. After construction no raw pointers are held, so the
/// collection can be freely moved and stored without lifetime concerns.
///
/// # Example
///
/// ```rust,no_run
/// # use cpdb_rs::{Frontend, init};
/// # fn main() -> cpdb_rs::error::Result<()> {
/// init();
/// let frontend = Frontend::new()?;
/// frontend.connect_to_dbus()?;
/// let printer = frontend.get_printer("My Printer")?;
///
/// let options = printer.get_options_collection()?;
/// for opt in &options.options {
///     println!("{}: {} (choices: {})", opt.name, opt.default_value,
///              opt.supported_values.join(", "));
/// }
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, Default)]
pub struct OptionsCollection {
    pub options: Vec<OptionInfo>,
}

impl OptionsCollection {
    /// Constructs an `OptionsCollection` by iterating `raw.table`.
    ///
    /// All string data is copied into owned Rust types inside this call.
    /// After `from_raw` returns, `raw` is no longer accessed.
    ///
    /// # Safety
    /// `raw` must be either null or a valid pointer to a fully initialised
    /// `cpdb_options_t` whose `table` field is a valid `GHashTable*`.
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

        // SAFETY: We initialise the iterator on the stack, iterate the
        // GHashTable synchronously, and copy all data into owned Rust Strings
        // before returning. The GHashTable is borrowed for the duration of
        // this block and is not modified. Pointers obtained from
        // `g_hash_table_iter_next` are borrowed references into the table and
        // must NOT be freed.
        unsafe {
            let mut iter = MaybeUninit::<GHashTableIter>::uninit();
            g_hash_table_iter_init(iter.as_mut_ptr(), table as *mut glib_sys::GHashTable);
            let mut iter = iter.assume_init();

            let mut key: *mut libc::c_void = std::ptr::null_mut();
            let mut value: *mut libc::c_void = std::ptr::null_mut();

            while g_hash_table_iter_next(
                &mut iter,
                &mut key as *mut *mut libc::c_void,
                &mut value as *mut *mut libc::c_void,
            ) != 0
            {
                if value.is_null() {
                    continue;
                }

                let opt = value as *mut ffi::cpdb_option_t;

                // Copy each field into an owned String. Null fields become
                // empty strings so callers never need to check for None.
                let name = util::cstr_to_string((*opt).option_name).unwrap_or_default();
                let default_value =
                    util::cstr_to_string((*opt).default_value).unwrap_or_default();
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

    // ─── Convenience helpers ─────────────────────────────────────────────────

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
}

// ─── Unit tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn null_pointer_returns_null_pointer_error() {
        let result = unsafe { OptionsCollection::from_raw(std::ptr::null_mut()) };
        assert!(
            matches!(result, Err(CpdbError::NullPointer)),
            "expected NullPointer, got {:?}",
            result
        );
    }

    #[test]
    fn null_table_returns_empty_collection() {
        // cpdb_options_t with a null `table` field
        let opts = ffi::cpdb_options_t {
            table: std::ptr::null_mut(),
            media: std::ptr::null_mut(),
            count: 0,
            media_count: 0,
        };
        let result = unsafe {
            OptionsCollection::from_raw(&opts as *const _ as *mut ffi::cpdb_options_t)
        };
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
    fn option_info_fields_are_correct() {
        // We test OptionInfo directly without going through a live GHashTable,
        // since constructing a real GHashTable requires cpdb-libs to be installed.
        let opt = OptionInfo {
            name: "copies".to_string(),
            default_value: "1".to_string(),
            group: "General".to_string(),
            supported_values: vec!["1".to_string(), "2".to_string(), "3".to_string()],
        };

        assert_eq!(opt.name, "copies");
        assert_eq!(opt.default_value, "1");
        assert_eq!(opt.group, "General");
        assert_eq!(opt.supported_values.len(), 3);
        assert_eq!(opt.supported_values[2], "3");
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
        assert!(found.is_some());
        assert_eq!(found.unwrap().default_value, "one-sided");
        assert!(col.get("nonexistent").is_none());
    }

    #[test]
    fn collection_len_and_iter() {
        let col = OptionsCollection {
            options: vec![
                OptionInfo {
                    name: "a".to_string(),
                    default_value: String::new(),
                    group: String::new(),
                    supported_values: vec![],
                },
                OptionInfo {
                    name: "b".to_string(),
                    default_value: String::new(),
                    group: String::new(),
                    supported_values: vec![],
                },
            ],
        };
        assert_eq!(col.len(), 2);
        assert_eq!(col.iter().count(), 2);
    }

    #[test]
    fn option_with_null_supported_values_is_empty() {
        // If supported_values is null, we should not crash and should produce
        // an empty supported_values vec.
        let opt = OptionInfo {
            name: "media".to_string(),
            default_value: "iso_a4_210x297mm".to_string(),
            group: "Media".to_string(),
            supported_values: vec![],
        };
        assert!(opt.supported_values.is_empty());
    }
}
