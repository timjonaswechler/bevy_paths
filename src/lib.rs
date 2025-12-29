mod error;
pub use error::PathRegistrationError;

use {
    bevy_app::{App, Plugin},
    bevy_ecs::resource::Resource,
    bevy_log::info,
    std::{
        any::TypeId,
        collections::HashMap,
        env, fs,
        path::{Component, Path, PathBuf},
        sync::Arc,
    },
    unicode_normalization::UnicodeNormalization,
};

/// The central registry for all managed application paths.
///
/// It provides structured, type-safe access to project directories.
/// All paths are stored relative to a central `base_path` (project root).
///
/// # Usage
/// 1. Define a marker struct: `#[derive(PathMarker)] struct SaveFiles;`
/// 2. Register it in the plugin: `.register::<SaveFiles>("saves")`
/// 3. Access it in systems: `registry.get::<SaveFiles>()` -> returns `.../project/saves`
#[derive(Resource, Clone)]
pub struct PathRegistry {
    studio: Arc<str>,
    project_id: Arc<str>,
    app_id: Arc<str>,
    base_path: Arc<Path>,
    paths: Arc<HashMap<TypeId, PathBuf>>,
    templates: Arc<HashMap<TypeId, String>>,
}

impl PathRegistry {
    fn new(studio: &str, project_id: &str, app_id: &str, base_path: PathBuf) -> Self {
        Self {
            studio: Arc::from(studio),
            project_id: Arc::from(project_id),
            app_id: Arc::from(app_id),
            base_path: Arc::from(base_path),
            paths: Arc::new(HashMap::new()),
            templates: Arc::new(HashMap::new()),
        }
    }

    fn with_paths(
        mut self,
        paths: HashMap<TypeId, PathBuf>,
        templates: HashMap<TypeId, String>,
    ) -> Self {
        self.paths = Arc::new(paths);
        self.templates = Arc::new(templates);
        self
    }

    // --- Meta Info ---

    pub fn studio(&self) -> &str {
        &self.studio
    }

    pub fn project_id(&self) -> &str {
        &self.project_id
    }

    pub fn app_id(&self) -> &str {
        &self.app_id
    }

    /// Returns the absolute root directory for this project: `<base_path>/<studio>/<project_id>`
    /// All other registered paths are relative to this root.
    pub fn project_root(&self) -> PathBuf {
        self.base_path
            .join(self.studio.as_ref())
            .join(self.project_id.as_ref())
    }

    // --- Path Access ---

    /// Returns the registered **relative** path for a given marker type `T`.
    ///
    /// Example: `Some("saves")`
    pub fn get_relative<T: 'static>(&self) -> Option<&PathBuf> {
        self.paths.get(&TypeId::of::<T>())
    }

    /// Returns the **absolute** path for a given marker type `T`.
    ///
    /// It combines the `project_root` with the registered relative path.
    /// Returns `None` if no path has been registered for `T`.
    ///
    /// Example: `Some("/path/to/project/saves")`
    pub fn get<T: 'static>(&self) -> Option<PathBuf> {
        self.get_relative::<T>()
            .map(|rel_path| self.project_root().join(rel_path))
    }

    /// Resolves a dynamic path template for marker type `T`.
    ///
    /// Replaces occurrences of `{key}` in the registered template with `value`.
    /// The result is returned as an absolute path.
    ///
    /// # Example
    /// Registered: `"saves/{slot}/data.json"`
    /// Call: `resolve::<T>("slot", "1")`
    /// Result: `.../project/saves/1/data.json`
    ///
    /// Returns `None` if `T` is not registered as a template.
    pub fn resolve<T: 'static>(&self, key: &str, value: &str) -> Option<PathBuf> {
        let template = self.templates.get(&TypeId::of::<T>())?;
        let placeholder = format!("{{{}}}", key);

        // Simple string replacement
        let resolved_rel = template.replace(&placeholder, value);

        // Validation: Ensure the resolved string is a valid relative path
        // (We don't do full validation here for perf, but we assume inputs are sane)
        Some(self.project_root().join(resolved_rel))
    }
}

/// Marker trait for types used to identify registered paths.
///
/// Implement this trait for your empty marker structs.
/// Example: `struct SaveFiles; impl PathMarker for SaveFiles {}`
pub trait PathMarker: 'static + Send + Sync {}

/// Bevy Plugin to initialize the `PathRegistry`.
pub struct PathRegistryPlugin {
    studio: String,
    project_id: String,
    app_id: String,
    #[cfg(debug_assertions)]
    base_path_override: Option<PathBuf>,
    registrations: Vec<(TypeId, PathBuf)>,
    template_registrations: Vec<(TypeId, String)>,
}

impl PathRegistryPlugin {
    #[must_use]
    pub fn new(studio: &str, project_id: &str, app_id: &str) -> Self {
        Self {
            studio: studio.to_string(),
            project_id: project_id.to_string(),
            app_id: app_id.to_string(),
            #[cfg(debug_assertions)]
            base_path_override: None,
            registrations: Vec::new(),
            template_registrations: Vec::new(),
        }
    }

    /// **[Debug Only]** Overrides the base path for development convenience.
    ///
    /// The path is relative to the executable directory.
    /// This method is only available and active in debug builds.
    /// In release builds, this call is either compiled out or ignored.
    #[cfg(debug_assertions)]
    #[must_use]
    pub fn with_base_path(mut self, path: impl AsRef<Path>) -> Self {
        self.base_path_override = Some(path.as_ref().to_path_buf());
        self
    }

    /// Registers a relative path for a specific marker type `T`.
    ///
    /// **Rules:**
    /// - Must not be empty.
    /// - Must not start with `~`.
    /// - Must be a relative path.
    /// - Must not contain `.` or `..` components.
    /// - Components are normalized to NFC.
    /// - Validated for Windows compatibility (reserved names, invalid chars) to ensure portability.
    #[must_use]
    pub fn register<T: PathMarker>(
        mut self,
        relative_path: &str,
    ) -> Result<Self, PathRegistrationError> {
        let normalized = self.validate_and_normalize(relative_path)?;
        self.registrations.push((TypeId::of::<T>(), normalized));
        Ok(self)
    }

    /// Registers a path template for a specific marker type `T`.
    ///
    /// Templates can contain placeholders like `{id}`.
    ///
    /// **Note:** Validation is less strict for templates (placeholders are allowed),
    /// but basic structural checks (no absolute paths, no `..`) still apply.
    #[must_use]
    pub fn register_template<T: PathMarker>(
        mut self,
        template: &str,
    ) -> Result<Self, PathRegistrationError> {
        // We perform basic validation on the template string to ensure it's not malicious
        // (e.g. no absolute paths, no backtracking).
        // We DO NOT validate components strictly because `{id}` contains chars that might be invalid on Windows (like `{`).
        let s = template.trim();
        if s.is_empty() {
            return Err(PathRegistrationError::EmptyPath);
        }
        if PathBuf::from(s).is_absolute() {
            return Err(PathRegistrationError::AbsolutePathNotAllowed);
        }
        if s.contains("..") {
            return Err(PathRegistrationError::RelativeNavigationNotAllowed);
        }

        self.template_registrations
            .push((TypeId::of::<T>(), s.to_string()));
        Ok(self)
    }

    fn validate_and_normalize(
        &self,
        relative_path: &str,
    ) -> Result<PathBuf, PathRegistrationError> {
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

        let mut normalized = PathBuf::new();
        for comp in p.components() {
            if let Component::Normal(os) = comp {
                let s_comp = os.to_string_lossy();
                let s_norm = normalize_component(&s_comp);
                validate_component(&s_norm)?;
                normalized.push(s_norm);
            }
        }
        Ok(normalized)
    }
}

impl Plugin for PathRegistryPlugin {
    fn build(&self, app: &mut App) {
        // Handle override path: available in debug, None in release.
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

        let paths: HashMap<TypeId, PathBuf> = self.registrations.iter().cloned().collect();
        let templates: HashMap<TypeId, String> =
            self.template_registrations.iter().cloned().collect();

        let final_registry = registry.with_paths(paths, templates);

        app.insert_resource(final_registry);
    }
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

/// Validates a path component for cross-platform compatibility (primarily Windows constraints).
fn validate_component(name: &str) -> Result<(), PathRegistrationError> {
    // 1. Invalid characters for Windows: < > " : | ? *
    let invalid = ['<', '>', '"', ':', '|', '?', '*'];
    if name.chars().any(|c| invalid.contains(&c)) {
        return Err(PathRegistrationError::InvalidComponent(name.to_string()));
    }

    // 2. Reserved names on Windows (case-insensitive)
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

    // 3. Trailing spaces or dots are problematic on Windows
    if name.ends_with(' ') || name.ends_with('.') {
        return Err(PathRegistrationError::InvalidComponent(name.to_string()));
    }

    Ok(())
}

#[cfg(test)]
mod tests;
