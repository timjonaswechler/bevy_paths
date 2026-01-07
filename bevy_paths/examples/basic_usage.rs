use bevy::prelude::*;
use bevy_paths::prelude::*;

// 1. Define Static Path (No fields)
#[derive(Path, Reflect, Debug)]
#[file("saves/settings.ini")]
struct SettingsFile;

// 2. Define Dynamic Path (With fields)
// The macro ensures fields match the placeholders {region} and {id}
#[derive(Path, Reflect, Debug)]
#[file("levels/{region}/dungeon_{id}.map")]
struct DungeonMap {
    region: String,
    id: u8,
}

fn main() {
    println!("--- Starting Bevy Paths (Typed) Example ---");

    let paths_plugin =
        PathRegistryPlugin::new("MyStudio", "MyGame", "ExampleApp").with_base_path("assets_debug");

    App::new()
        .add_plugins((
            MinimalPlugins, // Simplest bevy plugin group for headless apps
            bevy::log::LogPlugin::default(),
        ))
        .add_plugins(paths_plugin)
        .add_systems(Startup, check_paths)
        .run();

    println!("--- Example Finished Successfully ---");
}

fn check_paths(registry: Res<PathRegistry>) {
    info!("Project Root: {:?}", registry.project_root());

    // 3. Resolve Static Path
    let settings_path = registry.resolve(&SettingsFile);
    info!("Settings Path: {:?}", settings_path);

    // 4. Resolve Dynamic Path
    let dungeon = DungeonMap {
        region: "toxic_sewers".to_string(),
        id: 5,
    };
    let dungeon_path = registry.resolve(&dungeon);
    info!("Dungeon Path: {:?}", dungeon_path);

    // Verify
    if dungeon_path.to_string_lossy().contains("toxic_sewers") {
        info!("✅ Dynamic resolution worked!");
    } else {
        error!("❌ Resolution failed!");
    }
}
