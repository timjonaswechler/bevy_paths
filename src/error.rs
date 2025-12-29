// error.rs

use std::io;
use std::path::PathBuf;
use thiserror::Error; // Füge `thiserror = "1.0"` zu deiner Cargo.toml hinzu

#[derive(Debug, Error)]
pub enum PathRegistrationError {
    #[error("Registered path cannot be empty.")]
    EmptyPath,

    #[error("Registered path cannot start with a tilde '~'.")]
    TildeNotAllowed,

    #[error("Registered path must be relative, but an absolute path was provided.")]
    AbsolutePathNotAllowed,

    #[error("Registered path cannot contain relative navigation like '.' or '..'.")]
    RelativeNavigationNotAllowed,

    #[error("Path component '{0}' contains invalid characters or is a reserved name on Windows.")]
    InvalidComponent(String),

    // --- NEUE / AKTUALISIERTE FEHLER FÜR base_path ---
    #[error("The provided base path '{0}' must be a directory, but it is a file.")]
    BasePathNotADirectory(PathBuf),

    #[error("Failed to resolve the canonical path for '{0}'. IO Error: {1}")]
    BasePathCanonicalizationFailed(PathBuf, io::Error),

    #[error("The base path resolved to the file system root '{0}', which is disallowed.")]
    BasePathIsRoot(PathBuf),

    #[error("Failed to create the base path directory '{0}'. IO Error: {1}")]
    CreateDirFailed(PathBuf, io::Error),
}
