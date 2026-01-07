# Bevy Paths

A clean, type-safe, and cross-platform path management plugin for [Bevy](https://bevyengine.org/).

## Why `bevy_paths`?

In game development, handling file paths cleanly is surprisingly hard. You often end up with:
- Hardcoded strings (`"assets/saves/"`) scattered across systems.
- Platform-specific issues (Windows `\` vs. Unix `/`).
- Confusion about where user data (saves, logs) should actually live.

`bevy_paths` provides a **central registry** for all your project's important directories. It acts as the "Single Source of Truth" for your file structure, supporting both static directories and dynamic path templates.

## Features

- **Ergonomic API:** Define paths as Rust structs. No string literals in your systems.
- **Type-Safe Templates:** Dynamic paths use struct fields (e.g., `id: u8`) to automatically populate templates.
- **Cross-Platform Safety:** Automatically handles OS-specific separators and implements validation for common naming constraints.
- **Project-Aware:** Keeps your data organized under a unified project root (`<base>/<studio>/<project>`).

## Usage

### 1. Define Your Paths

Derive `Path` and `Reflect` on your structs. Use the `#[file("...")]` attribute to define the relative path or template.

```rust
use bevy::prelude::*;
use bevy_paths::prelude::*;

// 1. Static Path (No fields)
#[derive(Path, Reflect, Debug)]
#[file("saves/settings.ini")]
struct SettingsFile;

// 2. Dynamic Path (With fields)
// The macro automatically matches struct fields to {placeholders}
#[derive(Path, Reflect, Debug)]
#[file("levels/{region}/dungeon_{id}.map")]
struct DungeonMap {
    region: String,
    id: u8,
}
```

### 2. Configure the Plugin

Add the plugin to your app. You specify the Studio, Game, and App IDs to construct the root path.

```rust
fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(PathRegistryPlugin::new("MyStudio", "MyGame", "Client"))
        .add_systems(Startup, check_paths)
        .run();
}
```

### 3. Resolve Paths in Systems

Inject the `PathRegistry` resource to resolve your typed paths into absolute `PathBuf`s.

```rust
fn check_paths(registry: Res<PathRegistry>) {
    // 1. Resolve Static Path
    // Returns: .../MyStudio/MyGame/saves/settings.ini
    let settings_path = registry.resolve(&SettingsFile);
    
    // 2. Resolve Dynamic Path
    // Returns: .../MyStudio/MyGame/levels/swamp/dungeon_5.map
    let dungeon_path = registry.resolve(&DungeonMap {
        region: "swamp".to_string(),
        id: 5,
    });
}
```

### Optional: Registration

While not required, you can explicitly register paths at startup. This validates your templates (checking for invalid characters or reserved filenames) immediately when the app starts, rather than waiting for the first use.

```rust
PathRegistryPlugin::new("MyStudio", "MyGame", "Client")
    .register::<SettingsFile>()
    .register::<DungeonMap>()
```

## Guarantees

- **No Traversal:** `..` and `.` components are forbidden to prevent directory traversal attacks or messiness.
- **Portability:** Path components are normalized (NFC) and checked against common restricted filenames (like `CON` or `PRN`) and characters (like `*`, `?`) to support cross-platform compatibility.

> **Note:** While this crate implements standard validation rules for Windows file names, these checks have not yet been verified on a native Windows system.
