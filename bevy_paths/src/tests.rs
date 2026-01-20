use super::*;
use bevy_paths_validation::*;
use bevy_reflect::Reflect;
use std::path::PathBuf;

// We need to implement TypedPath manually for tests if we want to avoid
// potential issues with the derive macro inside the crate's own unit tests
// (sometimes reliable, sometimes not depending on linking).
// But for simplicity, let's try using the macro if possible, or manual impls.

#[derive(Reflect)]
struct SavePath;

impl TypedPath for SavePath {
    const TEMPLATE: &'static str = "saves/slot_1";
    const PLACEHOLDERS: &'static [&'static str] = &[];
}

#[derive(Reflect)]
struct DynamicLevel {
    id: String,
}

impl TypedPath for DynamicLevel {
    const TEMPLATE: &'static str = "levels/{id}/map.dat";
    const PLACEHOLDERS: &'static [&'static str] = &["id"];
}

#[derive(Reflect)]
struct MultiVarPath {
    x: i32,
    y: i32,
}

impl TypedPath for MultiVarPath {
    const TEMPLATE: &'static str = "chunks/{x}_{y}.dat";
    const PLACEHOLDERS: &'static [&'static str] = &["x", "y"];
}

#[test]
fn test_static_path_resolution() {
    let save_dir = SavePath;
    let resolved = save_dir.resolve().expect("error to resolve");

    let expected = PathBuf::from("saves/slot_1");
    assert!(resolved.ends_with(expected));
}

#[test]
fn test_dynamic_path_resolution() {
    let level = DynamicLevel {
        id: "dungeon_1".to_string(),
    };
    let resolved = level.resolve().unwrap();

    let expected = PathBuf::from("levels/dungeon_1/map.dat");
    assert!(resolved.ends_with(expected));
}

#[test]
fn test_multi_variable_resolution() {
    let chunk = MultiVarPath { x: 10, y: -5 };
    let resolved = chunk.resolve().unwrap();

    let expected = PathBuf::from("chunks/10_-5.dat");
    assert!(resolved.ends_with(expected));
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
