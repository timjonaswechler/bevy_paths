use super::*;
use std::path::PathBuf;

// Define some dummy marker structs for testing
struct SavePath;
impl PathMarker for SavePath {}

struct AssetPath;
impl PathMarker for AssetPath {}

struct DynamicLevel;
impl PathMarker for DynamicLevel {}

#[test]
fn test_project_root_construction() {
    let registry = PathRegistry::new(
        "MyStudio",
        "MyGame",
        "Client",
        PathBuf::from("/tmp/base"),
    );

    let root = registry.project_root();
    // Use components to be OS-agnostic separator-wise
    let expected = PathBuf::from("/tmp/base").join("MyStudio").join("MyGame");
    assert_eq!(root, expected);
}

#[test]
fn test_simple_path_registration() {
    let mut registry = PathRegistry::new(
        "S", "P", "A", 
        PathBuf::from("/base")
    );
    
    // Manually injecting paths (simulating what the Plugin does)
    let mut paths = HashMap::new();
    paths.insert(TypeId::of::<SavePath>(), PathBuf::from("saves/slot_1"));
    registry = registry.with_paths(paths, HashMap::new());

    // Test get_relative
    assert_eq!(
        registry.get_relative::<SavePath>(),
        Some(&PathBuf::from("saves/slot_1"))
    );

    // Test get (absolute)
    let abs_path = registry.get::<SavePath>().expect("Should be registered");
    let expected = PathBuf::from("/base/S/P/saves/slot_1");
    assert_eq!(abs_path, expected);

    // Test missing
    assert!(registry.get::<AssetPath>().is_none());
}

#[test]
fn test_template_resolution() {
    let mut registry = PathRegistry::new(
        "S", "P", "A", 
        PathBuf::from("/base")
    );

    let mut templates = HashMap::new();
    // Register: "levels/{id}/map.dat"
    templates.insert(TypeId::of::<DynamicLevel>(), "levels/{id}/map.dat".to_string());
    
    registry = registry.with_paths(HashMap::new(), templates);

    // 1. Resolve valid
    let path = registry.resolve::<DynamicLevel>("id", "dungeon_1");
    assert!(path.is_some());
    let path = path.unwrap();
    
    // Expected: /base/S/P/levels/dungeon_1/map.dat
    let expected = PathBuf::from("/base/S/P/levels/dungeon_1/map.dat");
    assert_eq!(path, expected);

    // 2. Resolve missing (wrong Marker)
    let missing = registry.resolve::<SavePath>("id", "1");
    assert!(missing.is_none());
}

#[test]
fn test_template_multi_replacement() {
    let mut registry = PathRegistry::new("S", "P", "A", PathBuf::from("/base"));
    let mut templates = HashMap::new();
    
    // Template with multiple occurrences
    templates.insert(TypeId::of::<DynamicLevel>(), "backup/{date}/{date}_save.json".to_string());
    registry = registry.with_paths(HashMap::new(), templates);

    let path = registry.resolve::<DynamicLevel>("date", "2024-01-01").unwrap();
    let expected = PathBuf::from("/base/S/P/backup/2024-01-01/2024-01-01_save.json");
    assert_eq!(path, expected);
}

#[test]
fn test_validation_logic() {
    // We can't test Plugin::register directly easily without App, 
    // but we can test the internal validation functions if we expose them or test via a helper.
    // Since validate_and_normalize is private, we will test the logic by trying to invoke the logic manually or 
    // rely on a small integration test in lib.rs logic if we could access private methods.
    
    // Alternatively, we verify the Plugin behavior by instantiating it.
    let plugin = PathRegistryPlugin::new("S", "P", "A");

    // Valid path
    assert!(plugin.validate_and_normalize("saves/data").is_ok());

    // Invalid: Absolute
    #[cfg(unix)]
    assert!(plugin.validate_and_normalize("/absolute/path").is_err());
    #[cfg(windows)]
    assert!(plugin.validate_and_normalize("C:\\absolute\\path").is_err());

    // Invalid: Parent navigation
    assert!(plugin.validate_and_normalize("../parent").is_err());
    assert!(plugin.validate_and_normalize("saves/../hacked").is_err());

    // Invalid: Empty
    assert!(plugin.validate_and_normalize("").is_err());
    assert!(plugin.validate_and_normalize("   ").is_err());
}

#[test]
fn test_windows_validation_simulated() {
    // Even on Unix, our validate_component logic should catch these if we enabled it globally.
    // (Which we did!)
    
    let plugin = PathRegistryPlugin::new("S", "P", "A");

    // Invalid chars
    assert!(plugin.validate_and_normalize("file_with_asterisk_*.txt").is_err());
    assert!(plugin.validate_and_normalize("file_with_pipe_|.txt").is_err());
    assert!(plugin.validate_and_normalize("file_with_question_?.txt").is_err());

    // Reserved names
    assert!(plugin.validate_and_normalize("CON").is_err());
    assert!(plugin.validate_and_normalize("folder/PRN/file").is_err());
    assert!(plugin.validate_and_normalize("lpt1").is_err()); // Case insensitive check
}
