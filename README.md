# hyprlauncher

Launcher overlay for Hyprland/Wayland built with Rust + Smithay Client Toolkit.

## Features

- Fullscreen `wlr-layer-shell` overlay.
- Keyboard and pointer navigation.
- Desktop entry scanning from XDG application directories.
- Hyprcolor palette integration through `~/.cache/hyprcolors/colors.json`.
- Wallpaper preview from the `wallpaper` field exported by hyprcolor.
- SHM + tiny-skia rendering. No GTK, no Qt, no Electron.

## Usage

```bash
cargo run --release
```

Keyboard:

- Type to search.
- `Up` / `Down`: move selection.
- `Enter`: launch selected app.
- `Esc`: close.
- `Backspace`: delete character.

Optional:

```bash
cargo run --release -- --max-results 9 --width 760 --height 420
```
