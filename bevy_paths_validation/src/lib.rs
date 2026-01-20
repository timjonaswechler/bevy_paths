#![warn(missing_docs)]

use std::io;
use std::path::{Component, PathBuf};
use unicode_normalization::UnicodeNormalization;

/// The `bevy_paths_validation` crate provides **cross-platform path validation** for the `bevy_paths` ecosystem.
///
/// It ensures that path templates are **safe, relative, and portable** across different operating systems.
///
/// # Architecture
///
/// This crate:
/// - Validates path templates for **structural correctness** (e.g., no `..`, no absolute paths).
/// - Normalizes Unicode components for **consistency**.
/// - Checks for **invalid characters** and **reserved names** (e.g., `CON`, `PRN` on Windows).
///
/// # Examples
///
/// ## Validating a Path Template
///
/// ```rust
/// use bevy_paths_validation::validate_structural_path;
///
/// let path = "assets/textures/{name}.png";
/// assert!(validate_structural_path(path).is_ok());
/// ```
///
/// ## Handling Validation Errors
///
/// ```rust
/// use bevy_paths_validation::{validate_structural_path, PathValidationError};
///
/// let invalid_path = "/absolute/path";
/// match validate_structural_path(invalid_path) {
///     Err(PathValidationError::AbsolutePathNotAllowed) => {
///         println!("Error: Path must be relative!");
///     }
///     _ => unreachable!(),
/// }
/// ```
///
/// # Performance
///
/// - **Unicode normalization** (`NFC`) is applied to path components for consistency.
/// - **Regex-free validation**: Uses `std::path::Component` for traversal (fast and safe).
///
/// # Edge Cases
///
/// - **Reserved names**: Paths like `CON`, `PRN`, or `LPT1` are rejected on Windows.
/// - **Unicode equivalence**: `é` and `é` are treated as the same component.
/// - **Trailing spaces/dots**: Components like `file .txt` or `file.` are rejected.

#[derive(Debug, thiserror::Error)]
pub enum PathValidationError {
    /// The path string was empty.
    ///
    /// # Recovery
    /// Provide a non-empty path template.
    #[error("Registered path cannot be empty.")]
    EmptyPath,

    /// The path started with a tilde `~`.
    ///
    /// # Recovery
    /// Use a relative path instead (e.g., `assets/file.txt` instead of `~/file.txt`).
    #[error("Registered path cannot start with a tilde '~'.")]
    TildeNotAllowed,

    /// The path provided was absolute, but a relative path was expected.
    ///
    /// # Recovery
    /// Remove the leading `/` or `C:\`.
    #[error("Registered path must be relative, but an absolute path was provided.")]
    AbsolutePathNotAllowed,

    /// The path contains `.` or `..` components.
    ///
    /// # Recovery
    /// Remove `.` or `..` from the path.
    #[error("Registered path cannot contain relative navigation like '.' or '..'.")]
    RelativeNavigationNotAllowed,

    /// A path component contains invalid characters or is a reserved name.
    ///
    /// # Recovery
    /// Remove invalid characters (`<`, `>`, `:`, `|`, `?`, `*`) or rename reserved components.
    #[error("Path component '{0}' contains invalid characters or is a reserved name on Windows.")]
    InvalidComponent(String),

    /// The base path exists but is not a directory.
    ///
    /// # Recovery
    /// Ensure the base path points to a directory, not a file.
    #[error("The provided base path '{0}' must be a directory, but it is a file.")]
    BasePathNotADirectory(PathBuf),

    /// Failed to canonicalize the base path.
    ///
    /// # Recovery
    /// Check file permissions or ensure the path exists.
    #[error("Failed to resolve the canonical path for '{0}'. IO Error: {1}")]
    BasePathCanonicalizationFailed(PathBuf, io::Error),

    /// The base path resolved to the filesystem root (not allowed).
    ///
    /// # Recovery
    /// Use a subdirectory instead of the root.
    #[error("The base path resolved to the file system root '{0}', which is disallowed.")]
    BasePathIsRoot(PathBuf),

    /// Failed to create the base path directory.
    ///
    /// # Recovery
    /// Check file permissions or disk space.
    #[error("Failed to create the base path directory '{0}'. IO Error: {1}")]
    CreateDirFailed(PathBuf, io::Error),
}

/// Validates a **relative path template** for structural correctness.
///
/// This function ensures that:
/// - The path is not empty.
/// - The path does not start with `~` or `/`.
/// - The path does not contain `.` or `..`.
/// - All components are valid (no invalid characters or reserved names).
///
/// # Arguments
///
/// * `relative_path` - A relative path template (e.g., `"assets/{name}.png"`).
///
/// # Returns
///
/// - `Ok(PathBuf)` if the path is valid.
/// - `Err(PathValidationError)` if the path is invalid.
///
/// # Examples
///
/// ```rust
/// use bevy_paths_validation::validate_structural_path;
///
/// let path = "assets/textures/{name}.png";
/// assert!(validate_structural_path(path).is_ok());
/// ```
///
/// ```rust
/// use bevy_paths_validation::{validate_structural_path, PathValidationError};
///
/// let invalid_path = "/absolute/path";
/// assert!(matches!(
///     validate_structural_path(invalid_path),
///     Err(PathValidationError::AbsolutePathNotAllowed)
/// ));
/// ```
///
/// # Performance
///
/// - Uses `std::path::Component` for traversal (fast and safe).
/// - Applies Unicode normalization (`NFC`) to components.
pub fn validate_structural_path(relative_path: &str) -> Result<PathBuf, PathValidationError> {
    let s = relative_path.trim();
    if s.is_empty() {
        return Err(PathValidationError::EmptyPath);
    }
    if s.starts_with('~') {
        return Err(PathValidationError::TildeNotAllowed);
    }
    let p = PathBuf::from(s);
    if p.is_absolute() {
        return Err(PathValidationError::AbsolutePathNotAllowed);
    }
    for comp in p.components() {
        match comp {
            Component::CurDir | Component::ParentDir => {
                return Err(PathValidationError::RelativeNavigationNotAllowed);
            }
            _ => {}
        }
    }
    // Strict component validation
    for comp in p.components() {
        if let Component::Normal(os) = comp {
            let s_comp = os.to_string_lossy();
            if !s_comp.contains('{') {
                let s_norm = normalize_component(&s_comp);
                validate_component(&s_norm)?;
            }
        }
    }
    Ok(p)
}

/// Normalizes a path component using Unicode NFC normalization.
///
/// This ensures that equivalent Unicode characters (e.g., `é` and `é`) are treated as the same component.
///
/// # Arguments
///
/// * `s` - A path component (e.g., `"textures"`).
///
/// # Returns
///
/// The normalized string.
///
/// # Examples
///
/// ```rust
/// use bevy_paths_validation::normalize_component;
///
/// let component = "é"; // Combining character
/// let normalized = normalize_component(component);
/// assert_eq!(normalized, "é"); // Precomposed character
/// ```
pub fn normalize_component(s: &str) -> String {
    s.nfc().collect()
}

/// Validates a single path component for invalid characters or reserved names.
///
/// This function checks for:
/// - Invalid characters (`<`, `>`, `"`, `:`, `|`, `?`, `*`).
/// - Reserved names (e.g., `CON`, `PRN`, `LPT1` on Windows).
/// - Trailing spaces or dots.
///
/// # Arguments
///
/// * `name` - A path component (e.g., `"textures"`).
///
/// # Returns
///
/// - `Ok(())` if the component is valid.
/// - `Err(PathValidationError::InvalidComponent)` if the component is invalid.
///
/// # Examples
///
/// ```rust
/// use bevy_paths_validation::validate_component;
///
/// assert!(validate_component("textures").is_ok());
/// assert!(validate_component("CON").is_err()); // Reserved name
/// ```
pub fn validate_component(name: &str) -> Result<(), PathValidationError> {
    let invalid = ['<', '>', '"', ':', '|', '?', '*'];
    if name.chars().any(|c| invalid.contains(&c)) {
        return Err(PathValidationError::InvalidComponent(name.to_string()));
    }

    let up = name.to_uppercase();
    let reserved = [
        "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
        "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
    ];
    for r in &reserved {
        if up == *r {
            return Err(PathValidationError::InvalidComponent(name.to_string()));
        }
    }

    if name.ends_with(' ') || name.ends_with('.') {
        return Err(PathValidationError::InvalidComponent(name.to_string()));
    }

    Ok(())
}
