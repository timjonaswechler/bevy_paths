use bevy::prelude::*;
use bevy_paths::prelude::*;

// 1. Define a static path
#[derive(Path, Reflect, Debug)]
#[file("settings/config.toml")]
struct SettingsFile;

// 2. Define a dynamic path
#[derive(Path, Reflect, Debug)]
#[file("levels/{name}/map.dat")]
struct Level {
    name: String,
}

fn main() {
    // Initialize Bevy App
    App::new()
        .add_plugins(MinimalPlugins)
        .add_systems(Startup, (load_settings, load_level))
        .run();
}

fn load_settings() {
    let settings = SettingsFile;
    let settings_path = settings.resolve().expect("Failed to resolve path");
    println!("Settings path: {:?}", settings_path);
}

fn load_level() {
    let dungeon = Level {
        name: "dungeon_1".to_string(),
    };
    let dungeon_path = dungeon.resolve().expect("Failed to resolve path");
    println!("Dungeon path: {:?}", dungeon_path);
}
