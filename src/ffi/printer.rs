//! Safe wrapper around `cpdb_printer_obj_t`.
//!
//! # Ownership and lifetimes
//!
//! Printers come in two flavours:
//!
//! - **Borrowed** — returned by [`crate::Frontend::find_printer`],
//!   [`crate::Frontend::get_printer`], [`crate::Frontend::get_printers`],
//!   and the default-printer accessors. The underlying C object is owned by
//!   the frontend's hash table; the Rust binding carries the frontend's
//!   lifetime so the borrow checker enforces that the printer cannot outlive
//!   its frontend.
//! - **Owned** — returned by [`Printer::load_from_file`]. The C object was
//!   allocated independently; Rust frees it via `cpdbDeletePrinterObj` on
//!   drop. Owned printers have a `'static` lifetime.
//!
//! `Printer` deliberately does not implement [`Send`] or [`Sync`]. Most
//! cpdb-libs methods on a printer object mutate shared state (the printer's
//! settings table is shared with the frontend), and the C library does not
//! lock internally. If you need to dispatch printer operations from
//! multiple threads, wrap a single printer in a [`std::sync::Mutex`].

use super::bindings as ffi;
use super::frontend::Frontend;
use super::util;
use super::callbacks::{self, AcquireCompletion};
use crate::error::{CpdbError, Result};
use crate::options::OptionsCollection;
use libc::c_char;
use std::collections::HashMap;
use std::ffi::{CStr, CString};
use std::marker::PhantomData;
use std::mem::MaybeUninit;
use std::os::fd::{FromRawFd, OwnedFd};
use std::ptr::NonNull;

/// Page margins in hundredths of a millimetre.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Margin {
    /// Top margin.
    pub top: i32,
    /// Bottom margin.
    pub bottom: i32,
    /// Left margin.
    pub left: i32,
    /// Right margin.
    pub right: i32,
}

/// One or more [`Margin`] entries returned by the backend for a media type.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Margins {
    /// Every margin set the backend reports for the queried media.
    pub entries: Vec<Margin>,
}

/// Media dimensions in hundredths of a millimetre.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MediaSize {
    /// Width.
    pub width: i32,
    /// Length.
    pub length: i32,
}

/// Handle returned by [`Printer::print_fd`].
///
/// The caller writes the document data to [`PrintFdHandle::fd`] and then
/// drops the handle to close it.
#[derive(Debug)]
pub struct PrintFdHandle {
    /// A writable file descriptor that consumes the print job data.
    pub fd: OwnedFd,
    /// The backend-assigned job ID, or an empty string when not provided.
    pub job_id: String,
    /// Optional auxiliary socket path the backend may return.
    pub socket_path: Option<String>,
}

/// Handle returned by [`Printer::print_socket`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrintSocketHandle {
    /// Path to a Unix-domain socket the caller writes the job data to.
    pub socket_path: String,
    /// The backend-assigned job ID, or an empty string when not provided.
    pub job_id: String,
}

/// An owned snapshot of a printer's translation table.
///
/// Built by walking `cpdb_printer_obj.translations` once and copying every
/// `(key, value)` pair into Rust-owned `String`s. After construction the
/// map holds no raw pointers and can be freely stored or sent across
/// threads.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TranslationMap {
    /// The locale this map was captured for, when available.
    pub locale: Option<String>,
    /// Translation entries: source string → localised string.
    pub entries: HashMap<String, String>,
}

impl TranslationMap {
    /// `true` when no entries are present.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Number of entries in the map.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Looks up the translation for `key`.
    pub fn get(&self, key: &str) -> Option<&str> {
        self.entries.get(key).map(String::as_str)
    }
}

/// A safe handle to a cpdb printer object.
///
/// See [the module docs](self) for the ownership and lifetime model.
#[derive(Debug)]
pub struct Printer<'frontend> {
    raw: NonNull<ffi::cpdb_printer_obj_t>,
    owned: bool,
    // Borrowed printers borrow from a `Frontend`; using a non-`Send`/`Sync`
    // marker also keeps owned printers off other threads, which matches
    // cpdb-libs' lack of internal locking.
    _marker: PhantomData<&'frontend Frontend>,
}

impl<'frontend> Printer<'frontend> {
    // ─── Constructors ────────────────────────────────────────────────────────

    /// Wraps a printer object that is owned by a frontend's hash table.
    ///
    /// The returned printer borrows from the frontend and will NOT be freed
    /// by Rust on drop.
    pub(crate) fn from_raw_borrowed(raw: *mut ffi::cpdb_printer_obj_t) -> Result<Self> {
        let raw = NonNull::new(raw).ok_or(CpdbError::NullPointer)?;
        Ok(Self {
            raw,
            owned: false,
            _marker: PhantomData,
        })
    }

    /// Wraps a printer object that the binding will free on drop.
    ///
    /// Used for printers loaded from a pickle file.
    pub(crate) fn from_raw_owned(raw: *mut ffi::cpdb_printer_obj_t) -> Result<Self> {
        let raw = NonNull::new(raw).ok_or(CpdbError::NullPointer)?;
        Ok(Self {
            raw,
            owned: true,
            _marker: PhantomData,
        })
    }

    /// Returns the raw pointer for use within this crate.
    #[doc(hidden)]
    pub fn as_raw(&self) -> *mut ffi::cpdb_printer_obj_t {
        self.raw.as_ptr()
    }

    /// `true` when this binding will free the underlying C object on drop.
    #[doc(hidden)]
    pub(crate) fn is_owned(&self) -> bool {
        self.owned
    }

    // ─── Field accessors ─────────────────────────────────────────────────────

    /// The backend-assigned printer ID (stable across sessions).
    pub fn id(&self) -> Result<String> {
        self.read_str_field(|p| unsafe { (*p).id })
    }

    /// The human-readable printer name.
    pub fn name(&self) -> Result<String> {
        self.read_str_field(|p| unsafe { (*p).name })
    }

    /// The physical location string supplied by the backend.
    pub fn location(&self) -> Result<String> {
        self.read_str_field(|p| unsafe { (*p).location })
    }

    /// A free-form description (`info` in the C struct).
    pub fn description(&self) -> Result<String> {
        self.read_str_field(|p| unsafe { (*p).info })
    }

    /// The printer's make and model.
    pub fn make_and_model(&self) -> Result<String> {
        self.read_str_field(|p| unsafe { (*p).make_and_model })
    }

    /// The backend name this printer belongs to (e.g. `"CUPS"`).
    pub fn backend_name(&self) -> Result<String> {
        self.read_str_field(|p| unsafe { (*p).backend_name })
    }

    /// The cached state string (`idle`, `processing`, ...).
    ///
    /// For an authoritative answer use [`Printer::get_updated_state`].
    pub fn cached_state(&self) -> Result<String> {
        self.read_str_field(|p| unsafe { (*p).state })
    }

    /// Reads an optional NUL-terminated string field from the printer struct.
    fn read_str_field<F>(&self, accessor: F) -> Result<String>
    where
        F: FnOnce(*mut ffi::cpdb_printer_obj_t) -> *const c_char,
    {
        let ptr = accessor(self.raw.as_ptr());
        // SAFETY: `ptr` is borrowed from the printer object; the read above
        // is just a field deref. `cstr_to_string` is null-tolerant.
        match unsafe { util::cstr_to_string(ptr) } {
            Ok(s) => Ok(s),
            Err(CpdbError::NullPointer) => Ok(String::new()),
            Err(e) => Err(e),
        }
    }

    // ─── State ───────────────────────────────────────────────────────────────

    /// Queries the current state from the backend.
    ///
    /// The returned string is owned by Rust; the cpdb-libs allocation is
    /// freed inside this call.
    pub fn get_updated_state(&self) -> Result<String> {
        // SAFETY: cpdbGetState returns a freshly `g_strdup`'d string we own.
        unsafe {
            let raw = ffi::cpdbGetState(self.raw.as_ptr());
            util::cstr_to_string_and_g_free(raw)
        }
    }

    /// `true` when the printer is accepting jobs.
    pub fn is_accepting_jobs(&self) -> Result<bool> {
        // SAFETY: pointer is non-null.
        Ok(unsafe { ffi::cpdbIsAcceptingJobs(self.raw.as_ptr()) } != 0)
    }

    // ─── Defaults ────────────────────────────────────────────────────────────

    /// Marks this printer as the user's default. Returns `true` on success.
    pub fn set_user_default(&self) -> Result<bool> {
        // SAFETY: pointer is non-null.
        Ok(unsafe { ffi::cpdbSetUserDefaultPrinter(self.raw.as_ptr()) } != 0)
    }

    /// Marks this printer as the system-wide default. Returns `true` on success.
    pub fn set_system_default(&self) -> Result<bool> {
        // SAFETY: pointer is non-null.
        Ok(unsafe { ffi::cpdbSetSystemDefaultPrinter(self.raw.as_ptr()) } != 0)
    }

    // ─── Job submission ──────────────────────────────────────────────────────

    /// Submits a file as a print job with no extra options.
    ///
    /// Returns the backend-assigned job ID string.
    pub fn print_file(&self, file_path: &str) -> Result<String> {
        let c_path = CString::new(file_path)?;
        // SAFETY: cpdbPrintFile returns a `g_strdup`'d job ID we own.
        unsafe {
            let id = ffi::cpdbPrintFile(self.raw.as_ptr(), c_path.as_ptr());
            util::cstr_to_string_and_g_free(id)
                .map_err(|_| CpdbError::JobFailed("cpdbPrintFile returned null".into()))
        }
    }

    /// Streams a print job over a file descriptor.
    ///
    /// cpdb-libs hands back a writable file descriptor; the caller writes
    /// the job's data to it and drops the returned handle to close it.
    /// The backend job ID and an optional auxiliary socket path are also
    /// returned.
    pub fn print_fd(&self, title: &str) -> Result<PrintFdHandle> {
        let c_title = CString::new(title)?;
        let mut jobid_ptr: *mut c_char = std::ptr::null_mut();
        let mut socket_ptr: *mut c_char = std::ptr::null_mut();
        // SAFETY: pointers are non-null; output params receive
        // `g_strdup`'d strings we own.
        let fd = unsafe {
            ffi::cpdbPrintFD(
                self.raw.as_ptr(),
                &mut jobid_ptr,
                c_title.as_ptr(),
                &mut socket_ptr,
            )
        };
        if fd < 0 {
            // Defensive cleanup in case cpdb-libs allocated before failing.
            unsafe {
                if !jobid_ptr.is_null() {
                    glib_sys::g_free(jobid_ptr as glib_sys::gpointer);
                }
                if !socket_ptr.is_null() {
                    glib_sys::g_free(socket_ptr as glib_sys::gpointer);
                }
            }
            return Err(CpdbError::JobFailed(
                "cpdbPrintFD returned an invalid fd".into(),
            ));
        }
        let job_id = if jobid_ptr.is_null() {
            String::new()
        } else {
            unsafe { util::cstr_to_string_and_g_free(jobid_ptr) }.unwrap_or_default()
        };
        let socket_path = if socket_ptr.is_null() {
            None
        } else {
            unsafe { util::cstr_to_string_and_g_free(socket_ptr) }.ok()
        };
        // SAFETY: cpdb-libs returned a valid fd we now own.
        let fd = unsafe { OwnedFd::from_raw_fd(fd) };
        Ok(PrintFdHandle {
            fd,
            job_id,
            socket_path,
        })
    }

    /// Streams a print job over a Unix-domain socket.
    ///
    /// cpdb-libs returns a socket path the caller connects to and writes
    /// the job data through, plus the backend-assigned job ID.
    pub fn print_socket(&self, title: &str) -> Result<PrintSocketHandle> {
        let c_title = CString::new(title)?;
        let mut jobid_ptr: *mut c_char = std::ptr::null_mut();
        // SAFETY: cpdb returns a `g_strdup`'d socket path we own.
        let socket_ptr =
            unsafe { ffi::cpdbPrintSocket(self.raw.as_ptr(), &mut jobid_ptr, c_title.as_ptr()) };
        if socket_ptr.is_null() {
            if !jobid_ptr.is_null() {
                unsafe { glib_sys::g_free(jobid_ptr as glib_sys::gpointer) };
            }
            return Err(CpdbError::JobFailed(
                "cpdbPrintSocket returned a null socket path".into(),
            ));
        }
        let socket_path = unsafe { util::cstr_to_string_and_g_free(socket_ptr) }?;
        let job_id = if jobid_ptr.is_null() {
            String::new()
        } else {
            unsafe { util::cstr_to_string_and_g_free(jobid_ptr) }.unwrap_or_default()
        };
        Ok(PrintSocketHandle {
            socket_path,
            job_id,
        })
    }

    /// Submits a job with a per-call set of options and an explicit title.
    ///
    /// Options are applied to the printer's settings table via
    /// `cpdbAddSettingToPrinter` before submission, so they take effect for
    /// this job and persist for subsequent jobs on the same printer until
    /// cleared via [`Printer::clear_setting`].
    ///
    /// Returns the backend-assigned job ID string.
    pub fn submit_job(
        &self,
        file_path: &str,
        options: &[(&str, &str)],
        title: &str,
    ) -> Result<String> {
        let c_path = CString::new(file_path)?;
        let c_title = CString::new(title)?;
        for (key, value) in options {
            let k = CString::new(*key)?;
            let v = CString::new(*value)?;
            // SAFETY: pointers are non-null; CStrings live until end of loop body.
            unsafe { ffi::cpdbAddSettingToPrinter(self.raw.as_ptr(), k.as_ptr(), v.as_ptr()) };
        }
        // SAFETY: cpdbPrintFileWithJobTitle returns a `g_strdup`'d job ID we own.
        unsafe {
            let id = ffi::cpdbPrintFileWithJobTitle(
                self.raw.as_ptr(),
                c_path.as_ptr(),
                c_title.as_ptr(),
            );
            util::cstr_to_string_and_g_free(id)
                .map_err(|_| CpdbError::JobFailed("cpdbPrintFileWithJobTitle returned null".into()))
        }
    }

    // ─── Options ─────────────────────────────────────────────────────────────

    /// Returns the default value for a named option, if the option exists.
    pub fn get_option(&self, option_name: &str) -> Result<Option<String>> {
        let c_name = CString::new(option_name)?;
        // SAFETY: `cpdbGetOption` returns a borrowed pointer into the printer's
        // option table; we must NOT free it. Reading `default_value` is a
        // simple field deref.
        unsafe {
            let opt = ffi::cpdbGetOption(self.raw.as_ptr(), c_name.as_ptr());
            if opt.is_null() {
                return Ok(None);
            }
            let dv = (*opt).default_value;
            if dv.is_null() {
                Ok(None)
            } else {
                util::cstr_to_string(dv).map(Some)
            }
        }
    }

    /// Returns the default value for a named option, freeing the GLib string.
    pub fn get_default(&self, option_name: &str) -> Result<String> {
        let c_name = CString::new(option_name)?;
        // SAFETY: `cpdbGetDefault` returns a `g_strdup`'d string we own.
        unsafe {
            let v = ffi::cpdbGetDefault(self.raw.as_ptr(), c_name.as_ptr());
            util::cstr_to_string_and_g_free(v)
        }
    }

    /// Returns the *current* (setting-or-default) value for a named option.
    pub fn get_current(&self, option_name: &str) -> Result<String> {
        let c_name = CString::new(option_name)?;
        // SAFETY: `cpdbGetCurrent` returns a `g_strdup`'d string we own.
        unsafe {
            let v = ffi::cpdbGetCurrent(self.raw.as_ptr(), c_name.as_ptr());
            util::cstr_to_string_and_g_free(v)
        }
    }

    /// Asynchronously asks the backend to populate the printer's full
    /// option table. Returns immediately; no completion notification.
    ///
    /// Use [`Printer::acquire_details_with`] if you need to be told when
    /// the operation finishes.
    pub fn acquire_details(&self) {
        // SAFETY: passing a null callback is documented as valid.
        unsafe {
            ffi::cpdbAcquireDetails(self.raw.as_ptr(), None, std::ptr::null_mut());
        }
    }

    /// Asynchronously populates the option table, firing `completion` when
    /// the backend responds.
    ///
    /// `completion` is invoked from cpdb-libs' D-Bus listener thread with
    /// `success = true` when the backend reported success. Panics inside
    /// the closure are caught and absorbed.
    pub fn acquire_details_with<F>(&self, completion: F)
    where
        F: FnOnce(&Printer<'_>, bool) + Send + 'static,
    {
        let boxed: Box<AcquireCompletion> = Box::new(completion);
        let user_data = callbacks::into_completion_user_data(boxed);
        // SAFETY: `user_data` was produced for this exact callback firing;
        // the trampoline reclaims and frees it.
        unsafe {
            ffi::cpdbAcquireDetails(
                self.raw.as_ptr(),
                Some(callbacks::acquire_trampoline),
                user_data,
            );
        }
    }

    /// Returns an owned snapshot of every option on this printer.
    ///
    /// Call [`Printer::acquire_details`] before this so the option table is
    /// populated by the backend.
    pub fn get_options_collection(&self) -> Result<OptionsCollection> {
        // SAFETY: `cpdbGetAllOptions` returns a borrowed pointer to the
        // printer's `options` field; the collection copies all data before
        // returning, so we never retain the pointer.
        unsafe {
            let opts = ffi::cpdbGetAllOptions(self.raw.as_ptr());
            let opts = NonNull::new(opts).ok_or_else(|| {
                CpdbError::BackendError(
                    "cpdbGetAllOptions returned null — call acquire_details() first".into(),
                )
            })?;
            Ok(OptionsCollection::from_raw(opts))
        }
    }

    // ─── Per-printer settings ────────────────────────────────────────────────

    /// Reads a per-printer setting, or returns `None` when unset.
    pub fn get_setting(&self, name: &str) -> Result<Option<String>> {
        let c_name = CString::new(name)?;
        // SAFETY: `cpdbGetSetting` returns a borrowed pointer into the
        // printer's settings table; we must NOT free it.
        unsafe {
            let v = ffi::cpdbGetSetting(self.raw.as_ptr(), c_name.as_ptr());
            if v.is_null() {
                Ok(None)
            } else {
                util::cstr_to_string(v).map(Some)
            }
        }
    }

    /// Inserts or overwrites a per-printer setting.
    pub fn add_setting(&self, name: &str, value: &str) -> Result<()> {
        let c_name = CString::new(name)?;
        let c_val = CString::new(value)?;
        // SAFETY: pointers are non-null; the CStrings outlive the call.
        unsafe { ffi::cpdbAddSettingToPrinter(self.raw.as_ptr(), c_name.as_ptr(), c_val.as_ptr()) };
        Ok(())
    }

    /// Removes a per-printer setting. Returns `Ok(true)` when it existed.
    pub fn clear_setting(&self, name: &str) -> Result<bool> {
        let c_name = CString::new(name)?;
        // SAFETY: pointers are non-null; the CString outlives the call.
        let existed =
            unsafe { ffi::cpdbClearSettingFromPrinter(self.raw.as_ptr(), c_name.as_ptr()) };
        Ok(existed != 0)
    }

    // ─── Media ───────────────────────────────────────────────────────────────

    /// Returns the descriptive name of a media type, if known.
    pub fn get_media(&self, media_name: &str) -> Result<Option<String>> {
        let c_name = CString::new(media_name)?;
        // SAFETY: `cpdbGetMedia` returns a borrowed pointer into the
        // printer's media table.
        unsafe {
            let m = ffi::cpdbGetMedia(self.raw.as_ptr(), c_name.as_ptr());
            if m.is_null() {
                return Ok(None);
            }
            let name_ptr = (*m).name;
            if name_ptr.is_null() {
                Ok(None)
            } else {
                util::cstr_to_string(name_ptr).map(Some)
            }
        }
    }

    /// Returns the media size for a named media type.
    ///
    /// Both dimensions are in hundredths of a millimetre.
    pub fn get_media_size(&self, media_name: &str) -> Result<MediaSize> {
        let c_name = CString::new(media_name)?;
        let (mut width, mut length): (i32, i32) = (0, 0);
        // SAFETY: passing valid pointers to two stack-allocated `i32`s.
        let rc = unsafe {
            ffi::cpdbGetMediaSize(self.raw.as_ptr(), c_name.as_ptr(), &mut width, &mut length)
        };
        if rc == 0 {
            Ok(MediaSize { width, length })
        } else {
            Err(CpdbError::NotFound(format!("media size '{media_name}'")))
        }
    }

    /// Returns every margin entry the backend reports for a named media type.
    pub fn get_media_margins(&self, media_name: &str) -> Result<Margins> {
        let c_name = CString::new(media_name)?;
        let mut raw_margins: *mut ffi::cpdb_margin_t = std::ptr::null_mut();
        // SAFETY: passing valid pointers; cpdb-libs writes to `raw_margins`.
        let count = unsafe {
            ffi::cpdbGetMediaMargins(self.raw.as_ptr(), c_name.as_ptr(), &mut raw_margins)
        };
        if count <= 0 || raw_margins.is_null() {
            return Err(CpdbError::NotFound(format!("media margins '{media_name}'")));
        }
        let mut out = Vec::with_capacity(count as usize);
        // SAFETY: cpdb-libs guarantees `count` valid entries at `raw_margins`.
        for i in 0..(count as isize) {
            let m = unsafe { &*raw_margins.offset(i) };
            out.push(Margin {
                top: m.top,
                bottom: m.bottom,
                left: m.left,
                right: m.right,
            });
        }
        Ok(Margins { entries: out })
    }

    // ─── Translations ────────────────────────────────────────────────────────

    /// Asynchronously fetches translations for the given locale.
    ///
    /// Use [`Printer::acquire_translations_with`] if you need to be told
    /// when the operation finishes.
    pub fn acquire_translations(&self, locale: &str) -> Result<()> {
        let c_locale = CString::new(locale)?;
        // SAFETY: passing a null callback is documented as valid.
        unsafe {
            ffi::cpdbAcquireTranslations(
                self.raw.as_ptr(),
                c_locale.as_ptr(),
                None,
                std::ptr::null_mut(),
            );
            ffi::cpdbAcquireTranslations(self.raw, c_locale.as_ptr(), None, std::ptr::null_mut());
        }
        Ok(())
    }

    /// Asynchronously fetches translations, firing `completion` on success/failure.
    ///
    /// `completion` is invoked from cpdb-libs' D-Bus listener thread.
    /// Panics inside the closure are caught and absorbed.
    pub fn acquire_translations_with<F>(&self, locale: &str, completion: F) -> Result<()>
    where
        F: FnOnce(&Printer<'_>, bool) + Send + 'static,
    {
        let c_locale = CString::new(locale)?;
        let boxed: Box<AcquireCompletion> = Box::new(completion);
        let user_data = callbacks::into_completion_user_data(boxed);
        // SAFETY: `user_data` was produced for this exact callback firing.
        unsafe {
            ffi::cpdbAcquireTranslations(
                self.raw.as_ptr(),
                c_locale.as_ptr(),
                Some(callbacks::acquire_trampoline),
                user_data,
            );
        }
        Ok(())
    }

    /// Returns the human-readable label for an option in the given locale.
    pub fn get_option_translation(&self, option: &str, locale: &str) -> Result<Option<String>> {
        let c_opt = CString::new(option)?;
        let c_locale = CString::new(locale)?;
        // SAFETY: cpdb returns a `g_strdup`'d translation string we own.
        unsafe {
            let t =
                ffi::cpdbGetOptionTranslation(self.raw.as_ptr(), c_opt.as_ptr(), c_locale.as_ptr());
            translation_to_option(t)
        }
    }

    /// Returns the human-readable label for a specific option choice.
    pub fn get_choice_translation(
        &self,
        option: &str,
        choice: &str,
        locale: &str,
    ) -> Result<Option<String>> {
        let c_opt = CString::new(option)?;
        let c_choice = CString::new(choice)?;
        let c_locale = CString::new(locale)?;
        // SAFETY: cpdb returns a `g_strdup`'d translation string we own.
        unsafe {
            let t = ffi::cpdbGetChoiceTranslation(
                self.raw.as_ptr(),
                c_opt.as_ptr(),
                c_choice.as_ptr(),
                c_locale.as_ptr(),
            );
            translation_to_option(t)
        }
    }

    /// Like [`get_option_translation`], but only consults the in-memory
    /// translation table — never falls through to the backend over D-Bus.
    ///
    /// Returns `None` if the option label has not been loaded yet. Call
    /// [`Printer::get_all_translations`] (or
    /// [`Printer::acquire_translations_with`]) first to populate the table.
    ///
    /// [`get_option_translation`]: Self::get_option_translation
    pub fn get_option_translation_from_table(
        &self,
        option: &str,
        locale: &str,
    ) -> Result<Option<String>> {
        let c_opt = CString::new(option)?;
        let c_locale = CString::new(locale)?;
        // SAFETY: cpdb returns a `g_strdup`'d translation string we own.
        unsafe {
            let t = ffi::cpdbGetOptionTranslationFromTable(
                self.raw.as_ptr(),
                c_opt.as_ptr(),
                c_locale.as_ptr(),
            );
            translation_to_option(t)
        }
    }

    /// Like [`get_choice_translation`], but only consults the in-memory
    /// translation table — never falls through to the backend over D-Bus.
    ///
    /// [`get_choice_translation`]: Self::get_choice_translation
    pub fn get_choice_translation_from_table(
        &self,
        option: &str,
        choice: &str,
        locale: &str,
    ) -> Result<Option<String>> {
        let c_opt = CString::new(option)?;
        let c_choice = CString::new(choice)?;
        let c_locale = CString::new(locale)?;
        // SAFETY: cpdb returns a `g_strdup`'d translation string we own.
        unsafe {
            let t = ffi::cpdbGetChoiceTranslationFromTable(
                self.raw.as_ptr(),
                c_opt.as_ptr(),
                c_choice.as_ptr(),
                c_locale.as_ptr(),
            );
            translation_to_option(t)
        }
    }

    /// Returns the human-readable label for an option group.
    pub fn get_group_translation(&self, group: &str, locale: &str) -> Result<Option<String>> {
        let c_group = CString::new(group)?;
        let c_locale = CString::new(locale)?;
        // SAFETY: cpdb returns a `g_strdup`'d translation string we own.
        unsafe {
            let t = ffi::cpdbGetGroupTranslation(
                self.raw.as_ptr(),
                c_group.as_ptr(),
                c_locale.as_ptr(),
            );
            translation_to_option(t)
        }
    }

    /// Synchronously populates every translation for the given locale.
    pub fn get_all_translations(&self, locale: &str) -> Result<()> {
        let c_locale = CString::new(locale)?;
        // SAFETY: pointers are non-null.
        unsafe { ffi::cpdbGetAllTranslations(self.raw.as_ptr(), c_locale.as_ptr()) };
        Ok(())
    }

    /// Returns an owned snapshot of the printer's cached translation table.
    ///
    /// Call [`Printer::get_all_translations`] (or
    /// [`Printer::acquire_translations_with`]) first to populate the table.
    /// Returns an empty map when no translations have been loaded.
    pub fn translations(&self) -> TranslationMap {
        // SAFETY: dereferencing the printer struct's `locale` and
        // `translations` fields is sound; we only read borrowed pointers.
        let raw = self.raw.as_ptr();
        let locale_ptr = unsafe { (*raw).locale };
        let table = unsafe { (*raw).translations } as *mut glib_sys::GHashTable;

        let locale = if locale_ptr.is_null() {
            None
        } else {
            unsafe { util::cstr_to_string(locale_ptr) }.ok()
        };

        if table.is_null() {
            return TranslationMap {
                locale,
                entries: HashMap::new(),
            };
        }

        let mut entries: HashMap<String, String> = HashMap::new();
        // SAFETY: iterator is initialised on the stack and iterated
        // synchronously; we copy all data into owned Strings before
        // returning. The table is not mutated during this loop.
        unsafe {
            let mut iter = MaybeUninit::<glib_sys::GHashTableIter>::uninit();
            glib_sys::g_hash_table_iter_init(iter.as_mut_ptr(), table);
            let mut iter = iter.assume_init();

            let mut key: glib_sys::gpointer = std::ptr::null_mut();
            let mut value: glib_sys::gpointer = std::ptr::null_mut();
            while glib_sys::g_hash_table_iter_next(&mut iter, &mut key, &mut value) != 0 {
                if key.is_null() || value.is_null() {
                    continue;
                }
                let k = CStr::from_ptr(key as *const c_char)
                    .to_string_lossy()
                    .into_owned();
                let v = CStr::from_ptr(value as *const c_char)
                    .to_string_lossy()
                    .into_owned();
                entries.insert(k, v);
            }
        }
        TranslationMap { locale, entries }
    }

    // ─── Debug helpers ───────────────────────────────────────────────────────

    /// Dumps a human-readable description of this printer to stderr via
    /// `cpdbDebugPrinter`. Useful for diagnostics; production code should
    /// inspect the typed accessors instead.
    pub fn debug_dump(&self) {
        // SAFETY: pointer is non-null.
        unsafe { ffi::cpdbDebugPrinter(self.raw.as_ptr()) };
    }

    /// Dumps the printer's basic option set to stdout via
    /// `cpdbPrintBasicOptions`. Diagnostics-only.
    pub fn dump_basic_options(&self) {
        // SAFETY: pointer is non-null.
        unsafe { ffi::cpdbPrintBasicOptions(self.raw.as_ptr()) };
    }

    // ─── Persistence ─────────────────────────────────────────────────────────

    /// Serialises this printer to a file (`cpdbPicklePrinterToFile`).
    pub fn pickle_to_file(&self, path: &str, frontend: &Frontend) -> Result<()> {
        let c_path = CString::new(path)?;
        // SAFETY: both pointers are non-null and live for the duration of the call.
        unsafe {
            ffi::cpdbPicklePrinterToFile(self.raw.as_ptr(), c_path.as_ptr(), frontend.as_raw());
        }
        Ok(())
    }

    /// Loads a printer that was previously serialised via [`pickle_to_file`].
    ///
    /// The returned printer is *owned* — it is freed when dropped.
    ///
    /// [`pickle_to_file`]: Self::pickle_to_file
    pub fn load_from_file(path: &str) -> Result<Printer<'static>> {
        let c_path = CString::new(path)?;
        // SAFETY: `cpdbResurrectPrinterFromFile` returns an owned printer.
        let raw = unsafe { ffi::cpdbResurrectPrinterFromFile(c_path.as_ptr()) };
        if raw.is_null() {
            return Err(CpdbError::NotFound(format!("pickled printer at {path}")));
        }
        Printer::<'static>::from_raw_owned(raw)
    }
}

/// Converts a cpdb-libs-allocated translation string into `Option<String>`,
/// freeing the underlying buffer.
///
/// # Safety
/// `ptr` must be null or a GLib-allocated NUL-terminated string we own.
unsafe fn translation_to_option(ptr: *mut c_char) -> Result<Option<String>> {
    if ptr.is_null() {
        Ok(None)
    } else {
        unsafe { util::cstr_to_string_and_g_free(ptr) }.map(Some)
    }
}

impl Drop for Printer<'_> {
    fn drop(&mut self) {
        if self.owned {
            // SAFETY: we own the pointer and have not aliased it.
            unsafe { ffi::cpdbDeletePrinterObj(self.raw.as_ptr()) };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_raw_borrowed_rejects_null() {
        let r = Printer::from_raw_borrowed(std::ptr::null_mut());
        assert!(matches!(r, Err(CpdbError::NullPointer)));
    }

    #[test]
    fn from_raw_owned_rejects_null() {
        let r = Printer::from_raw_owned(std::ptr::null_mut());
        assert!(matches!(r, Err(CpdbError::NullPointer)));
    }

    // `load_from_file` calls `cpdbResurrectPrinterFromFile` — real FFI.
    // Miri cannot interpret it, so skip there.
    #[test]
    #[cfg_attr(miri, ignore)]
    fn load_from_nonexistent_file_returns_error() {
        let r = Printer::load_from_file("/tmp/cpdb-rs-nonexistent-pickle-file");
        assert!(r.is_err());
    }
}
