use super::*;
use bevy_reflect::Reflect;

// We need to implement TypedPath manually for tests if we want to avoid
// potential issues with the derive macro inside the crate's own unit tests
// (sometimes reliable, sometimes not depending on linking).
// But for simplicity, let's try using the macro if possible, or manual impls.

#[derive(Reflect)]
struct SavePath;

impl TypedPath for SavePath {
    fn template() -> &'static str {
        "saves/slot_1"
    }
}

#[derive(Reflect)]
struct DynamicLevel {
    id: String,
}

impl TypedPath for DynamicLevel {
    fn template() -> &'static str {
        "levels/{id}/map.dat"
    }
}

#[derive(Reflect)]
struct MultiVarPath {
    x: i32,
    y: i32,
}

impl TypedPath for MultiVarPath {
    fn template() -> &'static str {
        "chunks/{x}_{y}.dat"
    }
}

#[test]
fn test_project_root_construction() {
    let base = PathBuf::from("/tmp/base");
    let registry = PathRegistry::new("MyStudio", "MyGame", "Client", base.clone());

    let root = registry.project_root();
    let expected = base.join("MyStudio").join("MyGame");
    assert_eq!(root, expected);
}

#[test]
fn test_static_path_resolution() {
    let base = PathBuf::from("/base");
    let registry = PathRegistry::new("S", "P", "A", base);

    let resolved = registry.resolve(&SavePath);
    // Project root is /base/S/P
    let expected = PathBuf::from("/base/S/P/saves/slot_1");
    assert_eq!(resolved, expected);
}

#[test]
fn test_dynamic_path_resolution() {
    let base = PathBuf::from("/base");
    let registry = PathRegistry::new("S", "P", "A", base);

    let level = DynamicLevel {
        id: "dungeon_1".to_string(),
    };
    let resolved = registry.resolve(&level);

    let expected = PathBuf::from("/base/S/P/levels/dungeon_1/map.dat");
    assert_eq!(resolved, expected);
}

#[test]
fn test_multi_variable_resolution() {
    let base = PathBuf::from("/base");
    let registry = PathRegistry::new("S", "P", "A", base);

    let chunk = MultiVarPath { x: 10, y: -5 };
    let resolved = registry.resolve(&chunk);

    // Should resolve to chunks/10_-5.dat
    let expected = PathBuf::from("/base/S/P/chunks/10_-5.dat");
    assert_eq!(resolved, expected);
}

#[test]
fn test_validation_logic_unit() {
    // Valid
    assert!(validate_structural_path("saves/data").is_ok());

    // Invalid Absolute
    if cfg!(unix) {
        assert!(validate_structural_path("/absolute").is_err());
    }

    // Invalid Traversal
    assert!(validate_structural_path("../parent").is_err());
    assert!(validate_structural_path("saves/../hack").is_err());

    // Invalid Empty
    assert!(validate_structural_path("").is_err());
}

#[test]
fn test_component_validation() {
    // We expect explicit invalid chars to fail
    assert!(validate_component("bad*name").is_err());
    assert!(validate_component("bad|name").is_err());
    assert!(validate_component("bad?name").is_err());

    // Reserved windows names
    assert!(validate_component("CON").is_err());
    assert!(validate_component("lpt1").is_err());
}
