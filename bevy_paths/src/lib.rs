mod error;
use error::PathRegistrationError;

use bevy::prelude::*;
use std::any::TypeId;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;
use unicode_normalization::UnicodeNormalization; // for NFC normalization

/// Zentrale Registry für alle registrierten Pfade
#[derive(Resource, Clone)]
pub struct PathRegistry {
    studio: Arc<str>,
    project_id: Arc<str>,
    app_id: Arc<str>,
    base_path: Arc<Path>,
    paths: Arc<HashMap<TypeId, PathBuf>>, // gespeicherte relative Pfade
}

impl PathRegistry {
    fn new(studio: &str, project_id: &str, app_id: &str, base_path: PathBuf) -> Self {
        Self {
            studio: Arc::from(studio),
            project_id: Arc::from(project_id),
            app_id: Arc::from(app_id),
            base_path: Arc::from(base_path),
            paths: Arc::new(HashMap::new()),
        }
    }

    fn with_paths(mut self, paths: HashMap<TypeId, PathBuf>) -> Self {
        self.paths = Arc::new(paths);
        self
    }

    pub fn studio(&self) -> &str {
        &self.studio
    }

    pub fn project_id(&self) -> &str {
        &self.project_id
    }

    pub fn app_id(&self) -> &str {
        &self.app_id
    }

    /// `<base_path>/<studio>/<project_id>`
    pub fn project_root(&self) -> PathBuf {
        self.base_path
            .join(self.studio.as_ref())
            .join(self.project_id.as_ref())
    }

    /// Registriert einen Pfad für einen bestimmten Typ intern.
    /// Nutzt Arc::make_mut, kein Panic wenn Arc geteilt ist.
    fn register<T: 'static>(&mut self, relative_path: PathBuf) {
        let map = Arc::make_mut(&mut self.paths);
        map.insert(TypeId::of::<T>(), relative_path);
    }

    /// Retrieves relative path
    pub fn get_relative<T: 'static>(&self) -> Option<&PathBuf> {
        self.paths.get(&TypeId::of::<T>())
    }

    /// Absolute Pfad: project_root + relative
    pub fn get<T: 'static>(&self) -> Option<PathBuf> {
        self.get_relative::<T>()
            .map(|rel_path| self.project_root().join(rel_path))
    }

    pub fn get_or_panic<T: 'static>(&self, type_name: &str) -> PathBuf {
        self.get::<T>()
            .unwrap_or_else(|| panic!("Path für {} nicht registriert", type_name))
    }
}

/// Marker
pub trait PathMarker: 'static {}
pub use bevy_paths_derive::PathMarker;

/// Plugin
pub struct PathRegistryPlugin {
    studio: String,
    project_id: String,
    app_id: String,
    // Speichert NUR den optionalen Pfad für den Debug-Fall
    base_path_override: Option<PathBuf>,
    registrations: Vec<(TypeId, PathBuf)>,
}

impl PathRegistryPlugin {
    #[must_use]
    pub fn new(studio: &str, project_id: &str, app_id: &str) -> Self {
        Self {
            studio: studio.to_string(),
            project_id: project_id.to_string(),
            app_id: app_id.to_string(),
            // Standard ist KEIN Override
            base_path_override: None,
            registrations: Vec::new(),
        }
    }

    /// **[Nur für Debug-Builds]** Setzt einen alternativen `base_path`,
    /// relativ zum Verzeichnis der ausführbaren Datei.
    ///
    /// Dieser Aufruf wird in einem **Release-Build (`--release`) komplett ignoriert**,
    /// wo immer das Verzeichnis der .exe verwendet wird.
    ///
    /// # Beispiel
    /// Um im Debug-Modus Daten im Projektverzeichnis statt in `target/debug` zu speichern,
    /// kann `../` verwendet werden.
    ///
    /// ```rust
    /// .with_base_path("../assets_dev")
    /// ```
    #[must_use]
    pub fn with_base_path(mut self, path: impl AsRef<Path>) -> Self {
        self.base_path_override = Some(path.as_ref().to_path_buf());
        self
    }

    /// Registriert einen Pfad relativ zum project_root (statisch-typed marker).
    /// Regeln:
    /// - leerer String nicht erlaubt
    /// - '~' verboten
    /// - absolute Pfade verboten
    /// - '.' oder '..' verboten
    /// - Komponenten werden NFC-normalisiert und Windows-kompatibilitätsgeprüft
    #[must_use]
    pub fn register<T: PathMarker>(
        mut self,
        relative_path: &str,
    ) -> Result<Self, PathRegistrationError> {
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

        // disallow '.' and '..' anywhere
        for comp in p.components() {
            match comp {
                Component::CurDir | Component::ParentDir => {
                    return Err(PathRegistrationError::RelativeNavigationNotAllowed);
                }
                _ => {}
            }
        }

        // rebuild normalized path from normal components
        let mut normalized = PathBuf::new();
        for comp in p.components() {
            if let Component::Normal(os) = comp {
                let s_comp = os.to_string_lossy();
                // normalize unicode to NFC
                let s_norm = normalize_component(&s_comp);
                // validate windows component (best-effort)
                validate_windows_component(&s_norm)?;
                normalized.push(s_norm);
            }
        }

        self.registrations.push((TypeId::of::<T>(), normalized));
        Ok(self)
    }
}

impl Plugin for PathRegistryPlugin {
    fn build(&self, app: &mut App) {
        // 1. Bestimme das Basisverzeichnis (entweder durch Override oder Standard).
        let base_path = determine_base_path(self.base_path_override.as_deref())
            .expect("Fatal: Failed to determine and validate the base path.");

        info!("Using final validated base path: '{:?}'", base_path);

        // 2. Erstelle und konfiguriere die PathRegistry.
        let registry = PathRegistry::new(&self.studio, &self.project_id, &self.app_id, base_path);

        let paths: HashMap<TypeId, PathBuf> = self.registrations.iter().cloned().collect();
        let final_registry = registry.with_paths(paths);

        // 3. Füge die Registry als Ressource zur App hinzu.
        app.insert_resource(final_registry);
    }
}

impl Default for PathRegistryPlugin {
    fn default() -> Self {
        Self::new("default_studio", "default_project", "default_app")
    }
}

/// Ermittelt, validiert und kanonisiert das zu verwendende Basisverzeichnis.
///
/// Die Funktion führt folgende Schritte aus:
/// 1.  Ermittelt das Verzeichnis der ausführbaren Datei als Standard.
/// 2.  Löst den optionalen `override_path` relativ zu diesem Verzeichnis auf,
///     falls er nicht bereits absolut ist.
/// 3.  Stellt sicher, dass das Verzeichnis existiert (und erstellt es bei Bedarf).
/// 4.  Kanonisiert den Pfad, um eine absolute und bereinigte Pfadangabe zu erhalten.
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
        Some(path) if path.is_absolute() => {
            info!("Using absolute path override: '{:?}'", path);
            path.to_path_buf()
        }
        Some(path) => {
            info!(
                "Resolving relative path override '{:?}' against exe dir '{:?}'",
                path, exe_dir
            );
            exe_dir.join(path)
        }
        None => {
            info!(
                "Using default base path (executable directory): '{:?}'",
                exe_dir
            );
            exe_dir
        }
    };

    // Stelle sicher, dass das Verzeichnis existiert.
    if !base_path.exists() {
        fs::create_dir_all(&base_path)
            .map_err(|e| PathRegistrationError::CreateDirFailed(base_path.clone(), e))?;
    }

    // Kanonisiere den Pfad für eine saubere, absolute Repräsentation.
    let canonical_path = base_path
        .canonicalize()
        .map_err(|e| PathRegistrationError::BasePathCanonicalizationFailed(base_path, e))?;

    // Sicherheitsprüfung: Verhindere, dass das Wurzelverzeichnis als Basis dient.
    if canonical_path.parent().is_none() {
        return Err(PathRegistrationError::BasePathIsRoot(canonical_path));
    }

    Ok(canonical_path)
}

/// Normalisiert eine Pfadkomponente mittels Unicode NFC.
///
/// Dies sorgt für plattformübergreifend konsistente Pfadnamen,
/// insbesondere auf Systemen wie macOS, die eine andere Normalisierungsform verwenden.
fn normalize_component(s: &str) -> String {
    s.nfc().collect()
}

/// Windows component validation (best-effort). Returns Err on invalid name/char/reserved name.
/// On non-windows builds it is a no-op.
#[cfg(windows)]
fn validate_windows_component(name: &str) -> Result<(), PathRegistrationError> {
    // invalid characters: < > " : | ? *
    let invalid = ['<', '>', '"', ':', '|', '?', '*'];
    if name.chars().any(|c| invalid.contains(&c)) {
        return Err(PathRegistrationError::InvalidComponent(name.to_string()));
    }

    // reserved names (case-insensitive) — partial list
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

#[cfg(not(windows))]
fn validate_windows_component(_name: &str) -> Result<(), PathRegistrationError> {
    Ok(())
}
