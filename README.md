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

### 2. Resolve Paths in Systems

```rust
fn check_paths( _ : Commands) {
    // 1. Create a varibale of the path struct defined before. 
    let map = RegionMap { save_name: "MySaveGame".into(), x: 10, y: 20 };
    
    // 2. Resolve to: ".../save/MySaveGame/region_10_20.map"
    let path = map.resolve().expect("Failed to resolve path");
}
```

## Guarantees

- **No Traversal:** `..` and `.` components are forbidden to prevent directory traversal attacks or messiness.
- **Portability:** Path components are normalized (NFC) and checked against common restricted filenames (like `CON` or `PRN`) and characters (like `*`, `?`) to support cross-platform compatibility.

> **Note:** While this crate implements standard validation rules for Windows file names, these checks have not yet been verified on a native Windows system.
