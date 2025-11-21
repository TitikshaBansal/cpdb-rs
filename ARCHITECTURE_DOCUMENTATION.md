# CPDB-RS: Comprehensive Architecture Documentation

## Table of Contents
1. [Project Overview](#project-overview)
2. [High-Level Architecture](#high-level-architecture)
3. [System Design Concepts](#system-design-concepts)
4. [Module Architecture](#module-architecture)
5. [Design Decisions](#design-decisions)
6. [Data Structures](#data-structures)
7. [Memory Management and Safety](#memory-management-and-safety)
8. [FFI Layer and Interoperability](#ffi-layer-and-interoperability)
9. [Build System and Dependencies](#build-system-and-dependencies)
10. [Error Handling Strategy](#error-handling-strategy)
11. [Testing Strategy](#testing-strategy)
12. [Cross-Platform Considerations](#cross-platform-considerations)

---

## Project Overview

**cpdb-rs** is a Rust binding library for the Common Print Dialog Backends (cpdb-libs) C library from OpenPrinting. This project provides safe, idiomatic Rust abstractions over the C API for interacting with printing systems on Linux and macOS.

### Purpose
- Enable Rust applications to interact with printing systems without unsafe C code
- Provide memory-safe abstractions over cpdb-libs
- Support printer discovery, job submission, and queue management
- Bridge Rust applications with D-Bus-based print services

### Key Technologies
- **Language**: Rust (2021 edition)
- **FFI**: bindgen for C bindings
- **Communication**: D-Bus for IPC
- **Platform Support**: Linux (primary), macOS (conditional)
- **Backend**: cpdb-libs C library

---

## High-Level Architecture

### System Components

```
┌───────────────────────────────────────────────────────────┐
│                    Rust Application Layer                 │
│  ┌────────────┐  ┌────────────┐  ┌────────────┐           │
│  │  Frontend  │  │  Printer   │  │  Settings  │           │
│  │  Wrapper   │  │  Wrapper   │  │  Wrapper   │           │
│  └────────────┘  └────────────┘  └────────────┘           │
└────────────────────┬──────────────────────────────────────┘
                     │ Safe Rust API
┌────────────────────▼──────────────────────────────────────┐
│                  FFI Layer (bindgen)                      │
│  ┌────────────────────────────────────────────────────┐   │
│  │   src/ffi.rs - Auto-generated C bindings           │   │
│  └────────────────────────────────────────────────────┘   │
└────────────────────┬──────────────────────────────────────┘
                     │ Unsafe FFI calls
┌────────────────────▼──────────────────────────────────────┐
│              cpdb-libs C Library Layer                    │
│  ┌────────────┐  ┌────────────┐  ┌────────────┐           │
│  │ cpdb-      │  │ cpdb-      │  │ cpdb-      │           │
│  │ frontend   │  │ backend    │  │ printer    │           │
│  └────────────┘  └────────────┘  └────────────┘           │
└────────────────────┬──────────────────────────────────────┘
                     │ D-Bus IPC
┌────────────────────▼──────────────────────────────────────┐
│              D-Bus System Layer                           │
│  ┌────────────────────────────────────────────────────┐   │
│  │   Print Service Daemons (CUPS, etc.)               │   │
│  └────────────────────────────────────────────────────┘   │
└───────────────────────────────────────────────────────────┘
```

### Data Flow

1. **Initialization**: Rust app calls `init()` → library initialization
2. **Frontend Creation**: `Frontend::new()` creates frontend object
3. **D-Bus Connection**: `connect_to_dbus()` establishes IPC connection
4. **Printer Discovery**: Frontend queries D-Bus for available printers
5. **Print Job**: Application selects printer → submits job → tracks status
6. **Resource Cleanup**: Drop implementations ensure proper cleanup

---

## System Design Concepts

### 1. **Wrapper Pattern**
Each major cpdb-libs object (Frontend, Printer, Settings, etc.) is wrapped in a Rust struct that:
- Holds a raw pointer to the C object
- Provides a safe Rust API
- Manages the object's lifecycle through Drop trait
- Implements Send and Sync for thread safety

**Example:**
```rust
pub struct Frontend {
    raw: *mut ffi::cpdb_frontend_obj_t,  // Raw C pointer
}
```

### 2. **Safe FFI Boundaries**
All FFI operations are isolated within:
- Explicit `unsafe` blocks
- Null pointer validation before dereferencing
- Proper error propagation to safe code
- Resource management through Drop traits

### 3. **Resource Management (RAII)**
Rust's ownership system is used for automatic resource cleanup:
- Frontend, Printer, Settings implement Drop
- Raw C pointers are freed when objects go out of scope
- No manual memory management required

### 4. **Error Propagation**
Comprehensive error handling with:
- Custom `CpdbError` enum using thiserror
- Result<T, CpdbError> return types
- Proper conversion from C error codes to Rust errors

### 5. **Lazy Initialization**
Library initialization is simple and non-blocking:
```rust
pub fn init() {
    unsafe { ffi::cpdbInit(); }
}
```

### 6. **Builder-like Pattern**
Some operations follow a builder pattern implicitly:
- Create object (`new()`)
- Configure (`add_setting()`)
- Execute (`save_to_disk()`)

---

## Module Architecture

### Core Modules

#### 1. **lib.rs** - Module Organization
```rust
pub mod ffi;          // FFI bindings
pub mod error;         // Error types
pub mod common;        // Initialization and version
pub mod frontend;      // Frontend wrapper
pub mod backend;       // Backend wrapper (stub)
pub mod printer;       // Printer wrapper
pub mod job;           // Print job wrapper (stub)
pub mod settings;      // Settings wrapper
pub mod util;          // Utility functions
```

**Design Decision**: Modules are logically separated by functionality, making the codebase maintainable and navigable.

#### 2. **ffi.rs** - Foreign Function Interface
Auto-generated by bindgen from `include/wrapper.h`.

**Header Structure:**
```c
#include <glib.h>
#include <glib-object.h>
#include <cpdb/cpdb.h>
#include <cpdb/cpdb-frontend.h>
#include <cpdb/backend.h>
```

**Bindgen Configuration:**
- Disabled comment generation to avoid broken links
- Added libc prefix for types
- Configured allowlists for specific functions and types
- Set size_t_is_usize for platform compatibility

#### 3. **frontend.rs** - Frontend Management

**Purpose**: Manages the connection to the print system via D-Bus.

**Key Design Decisions**:
- Frontend object is the entry point for printer discovery
- Connect/disconnect lifecycle is explicit
- Thread-safe through `Send` and `Sync` implementations
- Provides raw pointer accessor (`as_raw()`) for internal use

**Key Methods**:
```rust
impl Frontend {
    pub fn new() -> Result<Self>
    pub fn connect_to_dbus(&self) -> Result<()>
    pub fn disconnect_from_dbus(&self) -> Result<()>
    pub fn get_printers(&self) -> Result<Vec<Printer>>
    pub fn start_listing(printer_callback) -> Result<Self>
    pub fn stop_listing_printers(&self) -> Result<()>
}
```

**Lifecycle Management**:
```rust
impl Drop for Frontend {
    fn drop(&mut self) {
        unsafe {
            if !self.raw.is_null() {
                ffi::cpdbDeleteFrontendObj(self.raw);
                self.raw = ptr::null_mut();
            }
        }
    }
}
```

#### 4. **printer.rs** - Printer Operations

**Purpose**: Represents a printer in the system and provides printing operations.

**Key Design Decisions**:
- Field access uses a generic helper pattern to reduce code duplication
- String fields are accessed through `get_string_field()` helper
- Media information is extracted and formatted as Rust types
- Printer configuration can be serialized/deserialized

**Field Access Pattern**:
```rust
fn get_string_field<F>(&self, field_accessor: F, field_name: &'static str) -> Result<String>
where
    F: FnOnce(*mut ffi::cpdb_printer_obj_t) -> *const c_char
{
    if self.raw.is_null() {
        return Err(CpdbError::BackendError(format!("...")));
    }
    unsafe {
        let c_ptr = field_accessor(self.raw);
        match util::cstr_to_string(c_ptr) {
            Ok(s) => Ok(s),
            Err(CpdbError::NullPointer) => Ok(String::new()),
            Err(e) => Err(e),
        }
    }
}
```

**Print Operations**:
- `print_single_file()`: Print a file directly
- `submit_job()`: Submit a print job with options and title
- Returns job ID as String

**Media Information**:
- `get_media()`: Get media type information
- `get_media_size()`: Get media dimensions (width, length)
- `get_media_margins()`: Get printable margins

**Design Pattern**: The Printer struct is Clone-able, but clones share the same underlying C object (doesn't call C clone).

#### 5. **settings.rs** - Printer Settings

**Purpose**: Manages printer settings and configuration options.

**Key Design Decisions**:
- Settings can be persisted to disk
- Add/clear settings operations
- Can copy settings for reuse
- Supports GVariant serialization for D-Bus

**Settings Structure**:
```rust
pub struct Settings {
    raw: *mut ffi::cpdb_settings_t,
}
```

**Lifecycle**:
```rust
impl Drop for Settings {
    fn drop(&mut self) {
        unsafe {
            if !self.raw.is_null() {
                ffi::cpdbDeleteSettings(self.raw);
                self.raw = ptr::null_mut();
            }
        }
    }
}
```

**Operations**:
- `new()`: Create new settings
- `copy()`: Create a copy of settings
- `add_setting(key, value)`: Add a setting
- `clear_setting(key)`: Remove a setting
- `save_to_disk()`: Persist to file
- `read_from_disk()`: Load from file

#### 6. **error.rs** - Error Handling

**Purpose**: Comprehensive error type system.

**Error Types**:
```rust
pub enum CpdbError {
    NullPointer,                          // Null pointer encountered
    InvalidPrinter,                       // Invalid printer object
    JobFailed(String),                   // Print job failed
    BackendError(String),                // Backend error
    FrontendError(String),               // Frontend error
    OptionError(String),                 // Option parsing error
    CupsError(i32),                      // CUPS error
    Utf8Error(#[from] std::str::Utf8Error),
    NulError(#[from] NulError),          // Nul byte in string
    IoError(#[from] std::io::Error),     // IO error
    InvalidStatus(i32),                  // Invalid status code
    Unsupported,                         // Unsupported operation
}
```

**Key Features**:
- Uses `thiserror` for automatic `Error` and `Display` implementations
- Automatic conversions for common errors via `#[from]`
- Status code conversion helper: `from_status()`
- String conversion helper: `cstr_to_string()`

#### 7. **util.rs** - Utility Functions

**Purpose**: Helper functions for C/Rust interoperation.

**Key Functions**:

1. **cstr_to_string**: Convert C string to Rust String
```rust
pub unsafe fn cstr_to_string(ptr: *const c_char) -> Result<String> {
    if ptr.is_null() {
        return Err(CpdbError::NullPointer);
    }
    unsafe {
        Ok(CStr::from_ptr(ptr)
            .to_str()?
            .to_string())
    }
}
```

2. **cstr_to_string_and_g_free**: Convert and free GLib string
```rust
pub unsafe fn cstr_to_string_and_g_free(c_ptr: *mut c_char) -> Result<String> {
    // Convert, then free the C memory
}
```

3. **to_c_options**: Convert Rust options to C options
```rust
pub fn to_c_options(options: &[(&str, &str)]) -> Result<Vec<ffi::cpdb_option_t>>
```

**Design Pattern**: These functions encapsulate unsafe operations and provide safe interfaces.

#### 8. **common.rs** - Library Initialization

**Purpose**: Provides library initialization and version information.

**Functions**:
```rust
pub fn init()                    // Initialize the library
pub fn version() -> Result<String>  // Get version string
```

**Design**: Simple, stateless initialization function.

---

## Design Decisions

### 1. **Wrapper Pattern Over OOP**
**Decision**: Wrap C objects in Rust structs rather than using trait objects.

**Rationale**:
- Simpler memory model (single struct with raw pointer)
- Direct pointer access is needed for C interop
- No virtual dispatch overhead
- Easier to reason about ownership

**Alternative Considered**: Trait-based design with polymorphic types, rejected due to complexity and FFI constraints.

### 2. **Explicit Unsafe Blocks**
**Decision**: All FFI operations are marked with explicit `unsafe` blocks.

**Rationale**:
- Makes FFI boundaries obvious in code
- Allows for audit of unsafe operations
- Preserves Rust's safety guarantees elsewhere

**Example**:
```rust
pub fn connect_to_dbus(&self) -> Result<()> {
    if self.raw.is_null() {
        return Err(CpdbError::FrontendError("...".to_string()));
    }
    unsafe {
        ffi::cpdbConnectToDBus(self.raw);
    }
    Ok(())
}
```

### 3. **Null Pointer Validation**
**Decision**: Validate pointers at API boundary before FFI calls.

**Rationale**:
- Prevents unsafe dereferences
- Provides meaningful error messages
- Fails fast at boundary

**Pattern Used**:
```rust
if self.raw.is_null() {
    return Err(CpdbError::BackendError("Pointer is null".to_string()));
}
unsafe {
    // Now safe to use self.raw
}
```

### 4. **Drop Trait for Resource Management**
**Decision**: Implement Drop for all wrapper types to ensure cleanup.

**Rationale**:
- RAII ensures automatic cleanup
- No manual `free()` calls needed
- Prevents resource leaks even on panic

**Implementation Pattern**:
```rust
impl Drop for Frontend {
    fn drop(&mut self) {
        unsafe {
            if !self.raw.is_null() {
                ffi::cpdbDeleteFrontendObj(self.raw);
                self.raw = ptr::null_mut();
            }
        }
    }
}
```

### 5. **Send + Sync for Thread Safety**
**Decision**: Mark types as `Send` and `Sync` for multi-threaded use.

**Rationale**:
- C objects are assumed thread-safe based on cpdb-libs design
- Allows use in async/multi-threaded contexts
- Matches expected usage patterns

**Implementation**:
```rust
unsafe impl Send for Frontend {}
unsafe impl Sync for Frontend {}
```

**Note**: This is safe if the underlying C library is thread-safe (which cpdb-libs is).

### 6. **Result Type for Error Handling**
**Decision**: Use `Result<T, CpdbError>` throughout.

**Rationale**:
- Idiomatic Rust error handling
- Type-safe error propagation
- Enforces error handling in caller

**Usage Pattern**:
```rust
pub fn get_printers(&self) -> Result<Vec<Printer>> {
    // ...
}
```

### 7. **Stubbed Modules for Future Expansion**
**Decision**: `backend.rs` and `job.rs` are currently stubs.

**Rationale**:
- cpdb-libs doesn't expose these as separate objects
- API allows future extension without breaking changes
- Placeholder for potential backend API development

### 8. **Static Initialization (No Global State)**
**Decision**: Keep library state in objects, not global variables.

**Rationale**:
- No global state means thread-safe by default
- Each Frontend can be independently managed
- Avoids initialization order issues

---

## Data Structures

### 1. **Frontend**
```rust
pub struct Frontend {
    raw: *mut ffi::cpdb_frontend_obj_t,  // Opaque C pointer
}
```
- **Size**: One pointer (8 bytes on 64-bit systems)
- **Ownership**: Frontend owns the C object
- **Thread Safety**: Send + Sync marked
- **Lifecycle**: Created via `new()`, deleted in `Drop`

### 2. **Printer**
```rust
pub struct Printer {
    raw: *mut ffi::cpdb_printer_obj_t,  // Opaque C pointer
}
```
- **Size**: One pointer (8 bytes on 64-bit systems)
- **Ownership**: Printer doesn't own the C object (Frontend does)
- **Thread Safety**: Send + Sync marked
- **Lifecycle**: Created from raw pointer, must not outlive Frontend

**Access to C struct fields**:
```rust
// C struct fields accessed via safe wrapper
pub fn id(&self) -> Result<String>
pub fn name(&self) -> Result<String>
pub fn location(&self) -> Result<String>
pub fn description(&self) -> Result<String>
pub fn make_and_model(&self) -> Result<String>
pub fn backend_name(&self) -> Result<String>
pub fn current_state_field(&self) -> Result<String>
```

### 3. **Settings**
```rust
pub struct Settings {
    raw: *mut ffi::cpdb_settings_t,  // Opaque C pointer
}
```
- **Size**: One pointer
- **Ownership**: Settings owns the C object
- **Operations**: Add/clear settings, save/load
- **Lifecycle**: Created via `new()`, deleted in `Drop`

### 4. **Options**
```rust
pub struct Options {
    raw: *mut ffi::cpdb_options_t,  // Opaque C pointer
}
```
- **Size**: One pointer
- **Purpose**: Represents printer options
- **Usage**: Used internally by Printer

### 5. **Media**
```rust
pub struct Media {
    raw: *mut ffi::cpdb_media_t,  // Opaque C pointer
}
```
- **Size**: One pointer
- **Purpose**: Represents media information
- **Usage**: Used internally by Printer

### 6. **CpdbError**
```rust
pub enum CpdbError {
    NullPointer,
    InvalidPrinter,
    JobFailed(String),
    BackendError(String),
    FrontendError(String),
    OptionError(String),
    CupsError(i32),
    Utf8Error(#[from] std::str::Utf8Error),
    NulError(#[from] NulError),
    IoError(#[from] std::io::Error),
    InvalidStatus(i32),
    Unsupported,
}
```
- **Size**: Largest variant (typically String variants)
- **Purpose**: Error information
- **Features**: Automatic conversions via `#[from]`

### 7. **Result Type**
```rust
pub type Result<T> = std::result::Result<T, CpdbError>;
```
- **Purpose**: Convenience type alias
- **Usage**: Used throughout the API

---

## Memory Management and Safety

### 1. **Ownership Model**

**Rule**: Each wrapper struct owns its C object pointer.

**Implementation**:
- Frontend owns its `cpdb_frontend_obj_t`
- Settings owns its `cpdb_settings_t`
- Options owns its `cpdb_options_t`
- Media owns its `cpdb_media_t`

**Exception**: Printer does NOT own its C object (Frontend does).

### 2. **Lifetime Management**

**Frontend**:
```rust
impl Drop for Frontend {
    fn drop(&mut self) {
        unsafe {
            if !self.raw.is_null() {
                ffi::cpdbDeleteFrontendObj(self.raw);  // Free C memory
                self.raw = ptr::null_mut();           // Invalidate pointer
            }
        }
    }
}
```

**Printer**:
```rust
impl Drop for Printer {
    fn drop(&mut self) {
        // Printer doesn't own the object, just invalidate the pointer
        if !self.raw.is_null() {
            self.raw = ptr::null_mut();
        }
    }
}
```

**Settings**:
```rust
impl Drop for Settings {
    fn drop(&mut self) {
        unsafe {
            if !self.raw.is_null() {
                ffi::cpdbDeleteSettings(self.raw);  // Free C memory
                self.raw = ptr::null_mut();
            }
        }
    }
}
```

### 3. **String Management**

**Pattern**: Convert C strings to Rust Strings immediately.

**Helper Function**:
```rust
pub unsafe fn cstr_to_string(ptr: *const c_char) -> Result<String> {
    if ptr.is_null() {
        return Err(CpdbError::NullPointer);
    }
    Ok(CStr::from_ptr(ptr)
        .to_str()?
        .to_string())
}
```

**Freeing GLib Strings**:
```rust
pub unsafe fn cstr_to_string_and_g_free(c_ptr: *mut c_char) -> Result<String> {
    if c_ptr.is_null() {
        return Err(CpdbError::NullPointer);
    }
    let result = unsafe {
        CStr::from_ptr(c_ptr)
            .to_str()
            .map(|s| s.to_string())
    }.map_err(CpdbError::from);
    
    glib_sys::g_free(c_ptr as *mut c_void);  // Free GLib memory
    result
}
```

**Rule**: Always free memory allocated by GLib using `g_free()`.

### 4. **Null Pointer Safety**

**Strategy**: Check for null before every dereference.

**Pattern**:
```rust
if self.raw.is_null() {
    return Err(CpdbError::BackendError("Pointer is null".to_string()));
}
unsafe {
    // Safe to dereference
}
```

### 5. **Use-After-Free Prevention**

**Strategy**: Invalidate pointers in Drop implementations.

**Example**:
```rust
impl Drop for Frontend {
    fn drop(&mut self) {
        unsafe {
            if !self.raw.is_null() {
                ffi::cpdbDeleteFrontendObj(self.raw);
                self.raw = ptr::null_mut();  // Prevent use-after-free
            }
        }
    }
}
```

### 6. **Double-Free Prevention**

**Strategy**: Check if already freed before freeing.

**Pattern**:
```rust
impl Drop for Settings {
    fn drop(&mut self) {
        unsafe {
            if !self.raw.is_null() {  // Check if not already freed
                ffi::cpdbDeleteSettings(self.raw);
                self.raw = ptr::null_mut();  // Mark as freed
            }
        }
    }
}
```

---

## FFI Layer and Interoperability

### 1. **bindgen Configuration**

**Build Script Setup** (build.rs):
```rust
let mut builder = bindgen::Builder::default()
    .header("include/wrapper.h")
    .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
    .size_t_is_usize(true)
    .derive_default(true)
    .generate_comments(false)  // Avoid broken rustdoc links
    .ctypes_prefix("libc")
    .layout_tests(false)
    .raw_line("use libc;")
    .raw_line("#[allow(non_upper_case_globals)]")
    .raw_line("#[allow(non_camel_case_types)]")
    .raw_line("#[allow(non_snake_case)]")
    .raw_line("#[allow(dead_code)]");
```

**Key Configuration Decisions**:
- `generate_comments(false)`: Avoid broken intra-doc links from C comments
- `ctypes_prefix("libc")`: Use libc for primitive types
- Allow non-standard naming: C APIs use camelCase, not snake_case

### 2. **Include Path Management**

**Strategy**: Support multiple discovery methods.

1. **Environment Variable**:
```rust
if let Ok(path) = env::var("CPDB_LIBS_PATH") {
    return Some(path);
}
```

2. **Common Installation Paths**:
```rust
let common_paths = [
    "/usr/local/lib/cpdb-libs",
    "/usr/lib/cpdb-libs",
    "/opt/cpdb-libs",
    format!("{}/cpdb-libs", home_dir),
];
```

3. **pkg-config**:
```rust
if let Ok(lib) = pkg_config::Config::new().probe("cpdb") {
    // Use pkg-config result
}
```

### 3. **Linker Configuration**

**Libraries Linked**:
```rust
println!("cargo:rustc-link-lib=cpdb");
println!("cargo:rustc-link-lib=cpdb-frontend");
println!("cargo:rustc-link-lib=glib-2.0");
println!("cargo:rustc-link-lib=gobject-2.0");
```

**Conditional Backend Linking**:
```rust
if matches!(env::var("CPDB_LINK_BACKEND").ok().as_deref(), 
            Some("1") | Some("true") | Some("yes")) {
    println!("cargo:rustc-link-lib=cpdb-backend");
}
```

### 4. **Function Allowlisting**

**Strategy**: Only generate bindings for needed functions.

**Benefits**:
- Faster bindgen execution
- Smaller ffi.rs file
- Explicit API surface

**Example**:
```rust
let functions_to_allow = [
    "cpdbGetVersion",
    "cpdbInit",
    "cpdbGetNewFrontendObj",
    "cpdbConnectToDBus",
    // ... many more
];
```

### 5. **Type Allowlisting**

**Strategy**: Only generate bindings for needed types.

**Example**:
```rust
let types_to_allow = [
    "cpdb_frontend_obj_s",
    "cpdb_frontend_obj_t",
    "cpdb_printer_obj_s",
    "cpdb_printer_obj_t",
    // ... more types
];
```

---

## Build System and Dependencies

### 1. **Cargo.toml Structure**

**Package Configuration**:
```toml
[package]
name = "cpdb-rs"
version = "0.1.0"
edition = "2021"
build = "build.rs"
```

**Dependencies**:
```toml
[dependencies]
libc = "0.2"
thiserror = "1.0"
log = "0.4"
once_cell = "1.18"
crossbeam-channel = "0.5"
glib-sys = "0.19"
```

**Why Each Dependency**:
- `libc`: C type definitions
- `thiserror`: Error type generation
- `log`: Logging framework
- `once_cell`: Thread-safe lazy initialization
- `crossbeam-channel`: Thread-safe channels
- `glib-sys`: GLib system bindings (for `g_free`)

**Build Dependencies**:
```toml
[build-dependencies]
bindgen = "0.69"
pkg-config = "0.3"
```

**Features**:
```toml
[features]
frontend = []
backend = []
```

**Dev Dependencies**:
```toml
[dev-dependencies]
tempfile = "3.10"  // For tests
```

### 2. **Build Process**

**Steps**:

1. **Discovery Phase** (build.rs):
   - Find cpdb-libs installation
   - Locate headers
   - Determine include paths

2. **Binding Generation**:
   - Run bindgen on `include/wrapper.h`
   - Generate `ffi.rs` with C function signatures
   - Write to `$OUT_DIR/cpdb_sys.rs`

3. **Linker Configuration**:
   - Set library search paths
   - Link required libraries
   - Configure target-specific paths

4. **Rust Compilation**:
   - Compile Rust code
   - Link against C libraries
   - Generate documentation

### 3. **Cross-Platform Considerations**

**Linux**:
- Standard library paths: `/usr/lib`, `/usr/lib/x86_64-linux-gnu`
- D-Bus readily available
- CUPS integration

**macOS**:
- Homebrew paths: `/usr/local/lib`, `/opt/homebrew/lib`
- No libcrypt by default
- May need Autotools to generate headers
- Conditional backend linking

**Platform Detection**:
```rust
fn add_system_library_paths() {
    let target = env::var("TARGET").unwrap_or_default();
    
    if target.contains("linux") {
        println!("cargo:rustc-link-search=native=/usr/lib/x86_64-linux-gnu");
        // ...
    } else if target.contains("darwin") {
        println!("cargo:rustc-link-search=native=/usr/local/lib");
        // ...
    }
}
```

---

## Error Handling Strategy

### 1. **Error Type Design**

**Enum-Based Error Type**:
```rust
#[derive(Error, Debug)]
pub enum CpdbError {
    #[error("Null pointer encountered")]
    NullPointer,
    
    #[error("Invalid printer object")]
    InvalidPrinter,
    
    #[error("Print job failed: {0}")]
    JobFailed(String),
    
    #[error("Backend error: {0}")]
    BackendError(String),
    
    // ... more variants
}
```

**Features**:
- `thiserror` provides automatic `Error` and `Display` impls
- Clear error messages with context
- Specific error types for different failure modes

### 2. **Error Propagation**

**Pattern**: Use `?` operator for automatic conversion.

**Example**:
```rust
pub fn name(&self) -> Result<String> {
    self.get_string_field(|p| unsafe { (*p).name }, "name")
}
```

**Automatic Conversions** (via `#[from]`):
```rust
#[error("Nul byte in string: {0}")]
NulError(#[from] NulError),

#[error("IO error: {0}")]
IoError(#[from] std::io::Error),
```

### 3. **Error Context**

**Strategy**: Provide context in error messages.

**Example**:
```rust
if self.raw.is_null() {
    return Err(CpdbError::BackendError(
        format!("Printer object pointer is null when accessing {}", field_name)
    ));
}
```

### 4. **Error Recovery**

**Strategy**: Graceful degradation where possible.

**Example**:
```rust
match util::cstr_to_string(c_ptr) {
    Ok(s) => Ok(s),
    Err(CpdbError::NullPointer) => Ok(String::new()),  // Return empty string
    Err(e) => Err(e),
}
```

---

## Testing Strategy

### 1. **Unit Tests** (tests/unit_tests.rs)

**Purpose**: Test individual components in isolation.

**Coverage**:
- Library initialization
- Version retrieval
- Frontend creation
- Settings operations
- Options creation
- Error handling
- String conversion utilities

**Test Structure**:
```rust
#[test]
fn test_frontend_creation() {
    setup_test_environment();
    match Frontend::new() {
        Ok(frontend) => { /* Success */ }
        Err(e) => { /* Expected in test env */ }
    }
}
```

### 2. **Integration Tests** (tests/integration.rs)

**Purpose**: Test components working together.

**Limitations**:
- Requires D-Bus connection
- Requires print backends
- Marked with `#[ignore]` for CI

**Usage**:
```bash
cargo test --ignored  # Run integration tests
```

### 3. **Example Tests**

**Purpose**: Verify examples compile and run.

**Examples**:
- `examples/basic_usage.rs`: Basic functionality
- `examples/cli_printer_manager.rs`: Full CLI

**Usage**:
```bash
cargo run --example basic_usage
cargo run --example cli_printer_manager -- list
```

### 4. **CI Integration**

**GitHub Actions**:
- Ubuntu: Full test suite
- macOS: Compilation tests (limited by dependencies)
- Documentation: rustdoc checks
- Security: `cargo audit`

---

## Cross-Platform Considerations

### 1. **Linux (Primary Platform)**

**Advantages**:
- Full D-Bus integration
- CUPS readily available
- Standard library paths
- Complete feature support

**Dependencies**:
```bash
sudo apt-get install libcpdb-dev
```

### 2. **macOS**

**Challenges**:
- No libcrypt by default
- Header generation via Autotools
- Conditional backend linking

**Dependencies**:
```bash
brew install autoconf automake libtool gettext pkg-config glib
```

**Build Configuration**:
```bash
export BINDGEN_EXTRA_CLANG_ARGS="-I$CPDB_LIBS_PATH -I$CPDB_LIBS_PATH/cpdb"
export CPDB_LIBS_PATH=$HOME/cpdb-libs
```

### 3. **Windows**

**Status**: Not currently supported.

**Reasons**:
- cpdb-libs is Unix-focused
- No D-Bus on Windows
- Different print architecture

### 4. **Conditional Compilation**

**Strategy**: Feature flags for platform-specific code.

**Example**:
```rust
#[cfg(target_os = "linux")]
// Linux-specific code

#[cfg(target_os = "macos")]
// macOS-specific code
```

---

## Implementation Details

### 1. **Printer Discovery Flow**

**Step-by-Step**:

1. **Initialize**: `init()` calls `cpdbInit()`
2. **Create Frontend**: `Frontend::new()` calls `cpdbGetNewFrontendObj()`
3. **Connect**: `connect_to_dbus()` calls `cpdbConnectToDBus()`
4. **List**: `get_printers()` calls `cpdbGetAllPrinters()`
5. **Convert**: Callbacks convert C printers to Rust `Vec<Printer>`
6. **Return**: Return printer list

**Current Limitation**: `get_printers()` returns empty vec (callback not yet implemented fully).

### 2. **Print Job Submission Flow**

**Step-by-Step**:

1. **Select Printer**: User selects from `Vec<Printer>`
2. **Build Options**: Create settings/options
3. **Submit**: `print_single_file()` or `submit_job()`
   - Convert Rust strings to C strings
   - Call `cpdbPrintFile()` or `cpdbPrintFileWithJobTitle()`
   - Return job ID
4. **Track**: Job ID can be used for status tracking

### 3. **String Conversion Flow**

**Pattern**:
```rust
Rust String → CString → *const c_char → C function
```

**Example**:
```rust
let c_file_path = CString::new(file_path)?;
unsafe {
    let job_id_ptr = ffi::cpdbPrintFile(self.raw, c_file_path.as_ptr());
    util::cstr_to_string_and_g_free(job_id_ptr)
}
```

### 4. **Settings Persistence Flow**

**Save**:
1. Add settings via `add_setting()`
2. Call `save_to_disk()` → `cpdbSaveSettingsToDisk()`
3. Settings written to system location

**Load**:
1. Call `read_from_disk()` → `cpdbReadSettingsFromDisk()`
2. Settings loaded from disk
3. Return `Settings` object

### 5. **Media Information Flow**

**Get Media**:
1. Call `get_media(media_name)` → `cpdbGetMedia()`
2. Returns `*mut cpdb_media_t`
3. Extract fields: name, size, margins
4. Convert to Rust types

**Get Size**:
1. Call `get_media_size(media_name, &mut width, &mut length)`
2. Populates width and length by reference
3. Returns `(width, length)` tuple

**Get Margins**:
1. Call `get_media_margins(media_name, &mut margins_ptr)`
2. Returns pointer to `cpdb_margin_t`
3. Extract top, bottom, left, right
4. Format as string

---

## Performance Considerations

### 1. **Zero-Copy Where Possible**

**String Handling**:
- Convert C strings to Rust strings only when needed
- Keep ownership of data to avoid copies
- Use `Cow<str>` internally where beneficial

### 2. **Minimal Allocations**

**Pattern**: Pre-allocate vectors with known capacity.

**Example**:
```rust
let mut c_options = Vec::with_capacity(options.len());
```

### 3. **Efficient Cloning**

**Printer Clone**:
```rust
impl Clone for Printer {
    fn clone(&self) -> Self {
        Self { raw: self.raw }  // Just copy pointer, don't deep clone
    }
}
```

**Settings Clone**:
```rust
impl Clone for Settings {
    fn clone(&self) -> Self {
        self.copy().expect("Failed to clone settings")
    }
}
```

### 4. **Lazy Initialization**

**Strategy**: Only call C functions when needed.

**Example**:
```rust
pub fn get_printers(&self) -> Result<Vec<Printer>> {
    // Call C function only if frontend exists
    if self.raw.is_null() {
        return Err(CpdbError::FrontendError("...".to_string()));
    }
    unsafe {
        ffi::cpdbGetAllPrinters(self.raw);
        Ok(Vec::new())
    }
}
```

---

## Security Considerations

### 1. **Input Validation**

**Strategy**: Validate all user inputs before FFI calls.

**Example**:
```rust
pub fn print_single_file(&self, file_path: &str) -> Result<String> {
    // Validate file_path is not empty, etc.
    let c_file_path = CString::new(file_path)?;  // Check for null bytes
    // ...
}
```

### 2. **Null Pointer Safety**

**Strategy**: Check every pointer before dereferencing.

**Pattern**: Always use `if ptr.is_null()` guard.

### 3. **Resource Cleanup**

**Strategy**: Always implement `Drop` to prevent leaks.

**Benefit**: Automatic cleanup even on panic.

### 4. **Error Information**

**Strategy**: Don't leak sensitive information in error messages.

**Example**: Generic error messages for external interfaces.

---

## Conclusion

This documentation provides a comprehensive overview of cpdb-rs, covering:

- **Architecture**: High-level and low-level design
- **Design Decisions**: Rationale for key choices
- **Data Structures**: All major types and their implementations
- **Memory Management**: Safety guarantees and resource management
- **FFI Layer**: Interoperability details
- **Build System**: Dependencies and cross-platform support
- **Error Handling**: Strategy and implementation
- **Testing**: Coverage and CI integration
- **Implementation Details**: How operations actually work

The library provides a safe, idiomatic Rust interface to cpdb-libs while maintaining full compatibility with the underlying C library. The design emphasizes memory safety, proper resource management, and a clear API that matches Rust conventions.

