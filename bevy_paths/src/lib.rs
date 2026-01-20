#![warn(missing_docs)]
//! A clean, type-safe, and cross-platform path management plugin for [Bevy](https://bevyengine.org/).
//!
//! ## Why `bevy_paths`?
//!
//! Managing file paths can quickly become a pain.
//! Hardcoded paths like `"assets/levels/level1.map"` are prone to typos, lack context, and can break across different platforms.
//!
//! `bevy_paths` solves this by introducing a **centralized path registry** and ensuring **type safety**.
//!
//! ## Features
//!
//! - **Ergonomic API:** Define paths as Rust structs. No string literals in your systems.
//! - **Type-Safe Templates:** Dynamic paths use struct fields (e.g., `id: u8`) to automatically populate templates.
//! - **Cross-Platform Safety:** Automatically handles OS-specific separators and implements validation for common naming constraints.
//! - **Project-Aware:** Keeps your data organized under a unified project root (`<base>/<studio>/<project>`).
//!
//! ## Usage
//!
//! 1. Add the **Plugin** to your App.
//! 2. Define your paths using the `#[file(...)]` macro.
//! 3. Resolve the paths.
//!
//! ```rust
//! use bevy::prelude::*;
//! use bevy_paths::prelude::*;
//! use std::path::PathBuf;
//!
//! // 1. Define a dynamic path
//! #[derive(Path, Reflect, Debug)]
//! #[file("save/{save_name}/region_{x}_{y}.map")]
//! struct RegionMap {
//!     save_name: String,
//!     x: u32,
//!     y: u32
//! }
//!
//! fn main() {
//!     App::new()
//!         .add_plugins(DefaultPlugins);
//! }
//!
//! fn load_system() {
//!     let map = RegionMap { save_name: "MySaveGame".into(), x: 10, y: 20 };
//!
//!     // 2. Resolve to: ".../save/MySaveGame/region_10_20.map"
//!     let path = map.resolve().expect("Failed to resolve path");
//! }
//! ```

use {bevy_reflect::Reflect, std::path::PathBuf};

/// In prelude are all necessary exports.
///
/// - [`Path`]
/// - [`PathValidationError`]
/// - [`TypedPath`]
pub mod prelude {
    pub use crate::{PathValidationError, TypedPath};
    pub use bevy_paths_derive::Path;
}

pub(crate) use bevy_paths_validation::validate_structural_path;
pub use {bevy_paths_derive::Path, bevy_paths_validation::PathValidationError};

mod private {
    use super::*;
    use bevy_reflect::{PartialReflect, Reflect};
    use std::{
        env, fs,
        path::{Path, PathBuf},
    };

    pub struct PathResolver;

    impl PathResolver {
        pub fn resolve(
            data: &dyn Reflect,
            template: &str,
            placeholders: &[&str],
        ) -> Result<PathBuf, PathValidationError> {
            let relative_path = Self::resolve_template_reflection(template, data, placeholders);
            let validated_path = validate_structural_path(&relative_path)?;
            let exe_dir = Self::determine_base_path(None)?;
            Ok(exe_dir.join(validated_path))
        }

        pub fn resolve_template_reflection(
            template: &str,
            data: &dyn Reflect,
            placeholders: &[&str],
        ) -> String {
            let mut result = template.to_string();
            if let Ok(reflect_struct) = data.reflect_ref().as_struct() {
                for field_name in placeholders {
                    if let Some(field_value) = reflect_struct.field(field_name) {
                        let value_str = Self::convert_reflect_to_string(field_value);
                        let placeholder = format!("{{{field_name}}}");
                        result = result.replace(&placeholder, &value_str);
                    }
                }
            }
            result
        }

        pub fn convert_reflect_to_string(value: &dyn PartialReflect) -> String {
            if let Some(v) = value.try_downcast_ref::<String>() {
                return v.clone();
            }
            if let Some(v) = value.try_downcast_ref::<u8>() {
                return v.to_string();
            }
            if let Some(v) = value.try_downcast_ref::<u16>() {
                return v.to_string();
            }
            if let Some(v) = value.try_downcast_ref::<u32>() {
                return v.to_string();
            }
            if let Some(v) = value.try_downcast_ref::<u64>() {
                return v.to_string();
            }
            if let Some(v) = value.try_downcast_ref::<usize>() {
                return v.to_string();
            }
            if let Some(v) = value.try_downcast_ref::<i8>() {
                return v.to_string();
            }
            if let Some(v) = value.try_downcast_ref::<i16>() {
                return v.to_string();
            }
            if let Some(v) = value.try_downcast_ref::<i32>() {
                return v.to_string();
            }
            if let Some(v) = value.try_downcast_ref::<i64>() {
                return v.to_string();
            }
            if let Some(v) = value.try_downcast_ref::<isize>() {
                return v.to_string();
            }
            if let Some(v) = value.try_downcast_ref::<f32>() {
                return v.to_string();
            }
            if let Some(v) = value.try_downcast_ref::<f64>() {
                return v.to_string();
            }
            if let Some(v) = value.try_downcast_ref::<bool>() {
                return v.to_string();
            }

            // Fallback
            format!("{value:?}")
        }

        pub fn determine_base_path(
            override_path: Option<&Path>,
        ) -> Result<PathBuf, PathValidationError> {
            let exe_dir = env::current_exe()
                .and_then(|p| {
                    p.parent().map(PathBuf::from).ok_or_else(|| {
                        std::io::Error::new(
                            std::io::ErrorKind::NotFound,
                            "Executable has no parent directory",
                        )
                    })
                })
                .map_err(|e| {
                    PathValidationError::BasePathCanonicalizationFailed(
                        PathBuf::from("<executable_path>"),
                        e,
                    )
                })?;

            let base_path = match override_path {
                Some(path) if path.is_absolute() => path.to_path_buf(),
                Some(path) => exe_dir.join(path),
                None => exe_dir,
            };

            if !base_path.exists() {
                fs::create_dir_all(&base_path)
                    .map_err(|e| PathValidationError::CreateDirFailed(base_path.clone(), e))?;
            }

            let canonical_path = base_path
                .canonicalize()
                .map_err(|e| PathValidationError::BasePathCanonicalizationFailed(base_path, e))?;

            if canonical_path.parent().is_none() {
                return Err(PathValidationError::BasePathIsRoot(canonical_path));
            }

            Ok(canonical_path)
        }
    }
}

/// The trait for defining a managed path.
///
/// This is typically implemented via `#[derive(Path)]`.
pub trait TypedPath: Reflect + 'static + Send + Sync {
    /// The template string (e.g. "levels/{id}.map") associated with this type.
    const TEMPLATE: &'static str;
    /// The list of placeholders in the template (e.g. `id` for "levels/{id}.map").
    const PLACEHOLDERS: &'static [&'static str];

    /// At usage of this function, the placeholders are replaced with the values of the fields.
    /// The function also validates the path structure.
    ///
    /// - If the path is invalid, a [PathValidationError] is returned.
    /// - If the path is valid, the resolved path is returned by a `PathBuf` type.
    fn resolve(&self) -> Result<PathBuf, PathValidationError> {
        private::PathResolver::resolve(self.as_reflect(), Self::TEMPLATE, Self::PLACEHOLDERS)
    }
}

#[cfg(test)]
mod tests;
