# Bevy Paths

A clean, type-safe, and cross-platform path management plugin for [Bevy](https://bevyengine.org/).

## Why `bevy_paths`?

In game development, handling file paths cleanly is surprisingly hard. You often end up with:
- Hardcoded strings (`"assets/saves/"`) scattered across systems.
- Platform-specific issues (Windows `\` vs. Unix `/`).
- Confusion about where user data (saves, logs) should actually live.

`bevy_paths` provides a **central registry** for all your project's important directories. It acts as the "Single Source of Truth" for your file structure, supporting both static directories and dynamic path templates.

## Features

- **Type-Safe Access:** Use Rust types (Marker Structs) instead of magic strings to access paths.
- **Dynamic Templates:** Define paths with placeholders like `saves/{slot}/data.json` and resolve them at runtime.
- **Cross-Platform Safety:** Automatically handles OS-specific separators and validates paths against Windows naming constraints on **all** platforms.
- **Project-Aware:** Keeps your data organized under a unified project root (`<base>/<studio>/<project>`).
- **Release-Ready:** Debug helpers (like overriding paths) are stripped out in release builds.

## Usage

### 1. Register Paths & Templates

Define your paths using marker structs and register them at startup. You can use simple paths or templates with `{}` placeholders.

```rust
use bevy_paths::{PathMarker, PathRegistryPlugin};

struct SaveDir;
impl PathMarker for SaveDir {}

struct LevelData;
impl PathMarker for LevelData {}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    App::new()
        .add_plugins(MinimalPlugins)
        .add_plugins(PathRegistryPlugin::new("MyStudio", "MyGame", "Client")
            // Static path
            .register::<SaveDir>("saves")?
            // Dynamic template path
            .register_template::<LevelData>("cache/levels/{id}.map")?
        )
        .add_systems(Startup, my_system)
        .run();
    Ok(())
}
```

### 2. Access and Resolve Paths

Inject the `PathRegistry` resource to retrieve your absolute paths.

```rust
use bevy_paths::PathRegistry;

fn my_system(registry: Res<PathRegistry>) {
    // 1. Static Path
    if let Some(path) = registry.get::<SaveDir>() {
        let save_file = path.join("slot_1.json");
        // Result: .../MyGame/saves/slot_1.json
    }

    // 2. Dynamic Template
    if let Some(path) = registry.resolve::<LevelData>("id", "dungeon_01") {
        // Result: .../MyGame/cache/levels/dungeon_01.map
    }
}
```

## Guarantees

- **No Traversal:** `..` and `.` components are forbidden during registration.
- **Portability:** Path components are normalized (NFC) and checked for Windows-reserved names (like `CON` or `PRN`) and invalid characters (`*`, `:`, etc.), ensuring your save games can be shared across all operating systems.