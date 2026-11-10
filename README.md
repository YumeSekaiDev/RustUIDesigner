# Form Designer

Visual Basic–style form designer and small Rust project workspace (`eframe` / `egui` **0.37.0**).

## Run

```bash
cd form_designer
cargo run
```

## Workspace

Tabs: **Design** | **Code** | **Terminal**

| Panel | Role |
|-------|------|
| Explorer | Files in the open folder; click a `.rs` file to open it in Code |
| Toolbox / Canvas / Inspector | Design-time form editing (Design tab) |
| Code | File editor or generated `main.rs` + `app.rs` preview |
| Terminal | `cargo init` / `cargo build` / `cargo run` with live output |

## Open Folder

**File → Open Folder…**

1. If `src/main.rs` or `src/app.rs` exists → load the project (and `form.json` when present).
2. Otherwise → ask *“No Rust files found. Create a new Cargo project here?”*
3. On Yes → write `Cargo.toml`, `src/main.rs`, `src/app.rs`, and `form.json`.

**File → Save** writes `form.json` and regenerates `src/main.rs` + `src/app.rs`.

### Generated code

- `main.rs` — window bootstrap only (`eframe::run_native`)
- `app.rs` — designed GUI (`DesignedApp` with absolute-positioned controls)

## Project folder + Run

The open folder path stays in app state. **▶ Run** (enabled when `Cargo.toml` exists) saves/regenerates sources, then spawns `cargo run` without blocking the designer.
