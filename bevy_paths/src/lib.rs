//! `bevy_paths` is a clean, type-safe, and cross-platform path management plugin for Bevy.
//!
//! It allows you to define paths as Rust structs (Typed Paths) and automatically resolves them
//! relative to a unified project root.

#![deny(missing_docs)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::return_self_not_must_use)]

mod error;
pub use error::PathRegistrationError;

// Re-export the derive macro
pub use bevy_paths_derive::Path;

use {
    bevy_app::{App, Plugin},
    bevy_ecs::prelude::Resource,
    bevy_log::info,
    bevy_reflect::{PartialReflect, Reflect},
    std::{
        env, fs,
        path::{Component, Path, PathBuf},
        sync::Arc,
    },
    unicode_normalization::UnicodeNormalization,
};

/// Common imports for using `bevy_paths`.
pub mod prelude {
    pub use crate::{Path, PathRegistry, PathRegistryPlugin, TypedPath};
}

/// The trait for defining a managed path.
///
/// This is typically implemented via `#[derive(Path)]`.
pub trait TypedPath: Reflect + 'static + Send + Sync {
    /// Returns the template string (e.g. "levels/{id}.map") associated with this type.
    fn template() -> &'static str;
}

/// The central registry for all managed application paths.
#[derive(Resource, Clone)]
pub struct PathRegistry {
    studio: Arc<str>,
    project_id: Arc<str>,
    #[allow(dead_code)]
    app_id: Arc<str>,
    base_path: Arc<Path>,
}

impl PathRegistry {
    fn new(studio: &str, project_id: &str, app_id: &str, base_path: PathBuf) -> Self {
        Self {
            studio: Arc::from(studio),
            project_id: Arc::from(project_id),
            app_id: Arc::from(app_id),
            base_path: Arc::from(base_path),
        }
    }

    /// Returns the absolute root directory for this project.
    pub fn project_root(&self) -> PathBuf {
        self.base_path
            .join(self.studio.as_ref())
            .join(self.project_id.as_ref())
    }

    /// Resolves a typed path struct to an absolute `PathBuf`.
    ///
    /// # Example
    /// ```rust,ignore
    /// let path = registry.resolve(&Level { id: 1 });
    /// ```
    pub fn resolve<T: TypedPath>(&self, data: &T) -> PathBuf {
        let template = T::template();
        let relative_path = resolve_template_reflection(template, data);
        self.project_root().join(relative_path)
    }
}

/// Helper to substitute {placeholders} using reflection.
fn resolve_template_reflection(template: &str, data: &dyn Reflect) -> String {
    let mut result = template.to_string();

    if let Ok(reflect_struct) = data.reflect_ref().as_struct() {
        for i in 0..reflect_struct.field_len() {
            let field_name = reflect_struct.name_at(i).unwrap_or_default();
            let field_value = reflect_struct.field_at(i).unwrap();

            // Basic type to string conversion
            let value_str = convert_reflect_to_string(field_value);

            let placeholder = format!("{{{field_name}}}");
            result = result.replace(&placeholder, &value_str);
        }
    }

    result
}

fn convert_reflect_to_string(value: &dyn PartialReflect) -> String {
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

/// Bevy Plugin to initialize the `PathRegistry`.
pub struct PathRegistryPlugin {
    studio: String,
    project_id: String,
    app_id: String,
    #[cfg(debug_assertions)]
    base_path_override: Option<PathBuf>,
}

impl PathRegistryPlugin {
    /// Creates a new plugin instance.
    pub fn new(studio: &str, project_id: &str, app_id: &str) -> Self {
        Self {
            studio: studio.to_string(),
            project_id: project_id.to_string(),
            app_id: app_id.to_string(),
            #[cfg(debug_assertions)]
            base_path_override: None,
        }
    }

    /// **[Debug Only]** Overrides the base path for development convenience.
    #[cfg(debug_assertions)]
    pub fn with_base_path(mut self, path: impl AsRef<Path>) -> Self {
        self.base_path_override = Some(path.as_ref().to_path_buf());
        self
    }

    /// Registers a type to validate its template at startup.
    /// This is optional but recommended.
    ///
    /// # Panics
    /// Panics if the template contains invalid characters or reserved filenames.
    pub fn register<T: TypedPath>(self) -> Self {
        let template = T::template();
        if let Err(e) = validate_structural_path(template) {
            panic!(
                "Invalid path template for type {}: {:?}",
                std::any::type_name::<T>(),
                e
            );
        }
        self
    }
}

impl Plugin for PathRegistryPlugin {
    fn build(&self, app: &mut App) {
        let override_path = {
            #[cfg(debug_assertions)]
            {
                self.base_path_override.as_deref()
            }
            #[cfg(not(debug_assertions))]
            {
                None
            }
        };

        let base_path = determine_base_path(override_path)
            .expect("Fatal: Failed to determine and validate the base path.");

        info!("Using final validated base path: '{:?}'", base_path);

        let registry = PathRegistry::new(&self.studio, &self.project_id, &self.app_id, base_path);
        app.insert_resource(registry);
    }
}

// --- Utils ---

fn validate_structural_path(relative_path: &str) -> Result<PathBuf, PathRegistrationError> {
    let s = relative_path.trim();
    if s.is_empty() {
        return Err(PathRegistrationError::EmptyPath);
    }
    if s.starts_with('~') {
        return Err(PathRegistrationError::TildeNotAllowed);
    }
    let p = PathBuf::from(s);
    if p.is_absolute() {
        return Err(PathRegistrationError::AbsolutePathNotAllowed);
    }
    for comp in p.components() {
        match comp {
            Component::CurDir | Component::ParentDir => {
                return Err(PathRegistrationError::RelativeNavigationNotAllowed);
            }
            _ => {}
        }
    }
    // Strict component validation
    for comp in p.components() {
        if let Component::Normal(os) = comp {
            // If the component contains braces {}, it's a template placeholder.
            // We should relax validation for that specific component if it's dynamic.
            // However, to keep it safe, let's validate non-placeholder components.
            let s_comp = os.to_string_lossy();
            if !s_comp.contains('{') {
                let s_norm = normalize_component(&s_comp);
                validate_component(&s_norm)?;
            }
        }
    }
    Ok(p)
}

fn determine_base_path(override_path: Option<&Path>) -> Result<PathBuf, PathRegistrationError> {
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
            PathRegistrationError::BasePathCanonicalizationFailed(
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
            .map_err(|e| PathRegistrationError::CreateDirFailed(base_path.clone(), e))?;
    }

    let canonical_path = base_path
        .canonicalize()
        .map_err(|e| PathRegistrationError::BasePathCanonicalizationFailed(base_path, e))?;

    if canonical_path.parent().is_none() {
        return Err(PathRegistrationError::BasePathIsRoot(canonical_path));
    }

    Ok(canonical_path)
}

fn normalize_component(s: &str) -> String {
    s.nfc().collect()
}

fn validate_component(name: &str) -> Result<(), PathRegistrationError> {
    let invalid = ['<', '>', '"', ':', '|', '?', '*'];
    if name.chars().any(|c| invalid.contains(&c)) {
        return Err(PathRegistrationError::InvalidComponent(name.to_string()));
    }

    let up = name.to_uppercase();
    let reserved = [
        "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
        "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
    ];
    for r in &reserved {
        if up == *r {
            return Err(PathRegistrationError::InvalidComponent(name.to_string()));
        }
    }

    if name.ends_with(' ') || name.ends_with('.') {
        return Err(PathRegistrationError::InvalidComponent(name.to_string()));
    }

    Ok(())
}

#[cfg(test)]
mod tests;
