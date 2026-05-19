# QuickConvert

QuickConvert is a Rust + GTK4/libadwaita desktop utility for **Imperial ↔ Metric** unit conversion. It targets first-year engineering workflows: length, weight, temperature, and volume, with precise decimal arithmetic via [`rust_decimal`](https://docs.rs/rust_decimal).

## Features

- Imperial and Metric radio selectors with persistent **Default** preference
- Categories: Length, Weight, Temperature, Volume
- Input/output unit dropdowns with **Swap Units**
- Configurable decimal precision (2–6 places, half-up rounding)
- Input validation (rejects scientific notation and extreme magnitudes)
- Preferences stored in `~/.config/quickconvert/preferences.json`
- KDE application menu entry via `.desktop` file

## Requirements

### Build dependencies (Debian/Ubuntu)

```bash
sudo apt install build-essential pkg-config libgtk-4-dev libadwaita-1-dev
```

### Build dependencies (Fedora)

```bash
sudo dnf install gcc pkg-config gtk4-devel libadwaita-devel
```

### Build dependencies (Arch)

```bash
sudo pacman -S base-devel pkgconf gtk4 libadwaita
```

### Runtime

- GTK 4 and Libadwaita 1.x

## Build

```bash
cargo build --release
```

The binary is `target/release/quickconvert`.

## Install

### User install (recommended)

```bash
cargo build --release
install -Dm755 target/release/quickconvert ~/.local/bin/quickconvert
install -Dm644 quickconvert.desktop ~/.local/share/applications/quickconvert.desktop
```

Ensure `~/.local/bin` is on your `PATH`.

### System install

```bash
cargo build --release
sudo install -Dm755 target/release/quickconvert /usr/local/bin/quickconvert
sudo install -Dm644 quickconvert.desktop /usr/local/share/applications/quickconvert.desktop
```

Update the desktop database (optional):

```bash
update-desktop-database ~/.local/share/applications
# or
sudo update-desktop-database /usr/local/share/applications
```

## Usage example

1. Launch **QuickConvert** from the KDE application menu.
2. Select **Metric** and check **Default**.
3. Choose **Length** → input **Meters** → output **Feet**.
4. Enter `10.99` → result `36.06` (with 2 decimal places).
5. Preferences save automatically when you change defaults or precision.

## Project layout

```
QuickConvert/
├── Cargo.toml
├── LICENSE                 # AGPL-3.0
├── README.md
├── quickconvert.desktop
├── data/
│   └── preferences.json    # bundled defaults
└── src/
    ├── main.rs             # GTK4 / libadwaita UI
    ├── converter.rs        # pure conversion logic
    ├── prefs.rs            # preferences load/save
    └── error.rs            # user-facing errors
```

## License

This project is licensed under the **GNU Affero General Public License v3.0** (AGPL-3.0). See [LICENSE](LICENSE).
