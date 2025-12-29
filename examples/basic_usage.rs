use {
    bevy_app::{App, Startup},
    bevy_ecs::prelude::Res,
    bevy_log::{LogPlugin, error, info},
    bevy_paths::{PathMarker, PathRegistry, PathRegistryPlugin},
};

// 1. Define Marker Structs
struct SaveDirectory;
impl PathMarker for SaveDirectory {}

struct LevelTemplate;
impl PathMarker for LevelTemplate {}

// Updated signature to support '?' operator
fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("--- Starting Bevy Paths Example ---");

    // 2. Configure Plugin cleanly
    let paths_plugin = PathRegistryPlugin::new("MyStudio", "MyGame", "ExampleApp")
        .with_base_path("assets_debug")
        .register::<SaveDirectory>("saves")?
        .register_template::<LevelTemplate>("levels/{id}.map")?;

    App::new()
        .add_plugins((
            bevy_app::TaskPoolPlugin::default(),
            bevy_diagnostic::FrameCountPlugin,
            bevy_time::TimePlugin,
            bevy_app::ScheduleRunnerPlugin::default(),
            LogPlugin::default(),
        ))
        .add_plugins(paths_plugin)
        .add_systems(Startup, check_paths)
        .run();

    println!("--- Example Finished Successfully ---");
    Ok(())
}

fn check_paths(registry: Res<PathRegistry>) {
    info!("Project Root: {:?}", registry.project_root());

    // 3. Access Static Path
    if let Some(save_dir) = registry.get::<SaveDirectory>() {
        info!("Save Directory: {:?}", save_dir);
    } else {
        error!("SaveDirectory not found!");
    }

    // 4. Resolve Template
    if let Some(level_path) = registry.resolve::<LevelTemplate>("id", "dungeon_01") {
        info!("Resolved Level Path: {:?}", level_path);

        let path_str = level_path.to_string_lossy();
        if path_str.contains("dungeon_01.map") {
            info!("✅ Template resolution worked!");
        } else {
            error!("❌ Template resolution failed: {}", path_str);
        }
    } else {
        error!("LevelTemplate not found!");
    }
}
