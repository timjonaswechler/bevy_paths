use std::io;
use std::path::PathBuf;
use thiserror::Error;

/// Errors that can occur during path registration and validation.
#[derive(Debug, Error)]
pub enum PathRegistrationError {
    /// The path string was empty.
    #[error("Registered path cannot be empty.")]
    EmptyPath,

    /// The path started with a tilde `~`.
    #[error("Registered path cannot start with a tilde '~'.")]
    TildeNotAllowed,

    /// The path provided was absolute, but a relative path was expected.
    #[error("Registered path must be relative, but an absolute path was provided.")]
    AbsolutePathNotAllowed,

    /// The path contains `.` or `..` components.
    #[error("Registered path cannot contain relative navigation like '.' or '..'.")]
    RelativeNavigationNotAllowed,

    /// A path component contains invalid characters or is a reserved name.
    #[error("Path component '{0}' contains invalid characters or is a reserved name on Windows.")]
    InvalidComponent(String),

    /// The base path exists but is not a directory.
    #[error("The provided base path '{0}' must be a directory, but it is a file.")]
    BasePathNotADirectory(PathBuf),

    /// Failed to canonicalize the base path.
    #[error("Failed to resolve the canonical path for '{0}'. IO Error: {1}")]
    BasePathCanonicalizationFailed(PathBuf, io::Error),

    /// The base path resolved to the filesystem root (not allowed).
    #[error("The base path resolved to the file system root '{0}', which is disallowed.")]
    BasePathIsRoot(PathBuf),

    /// Failed to create the base path directory.
    #[error("Failed to create the base path directory '{0}'. IO Error: {1}")]
    CreateDirFailed(PathBuf, io::Error),
}
