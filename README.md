# QuickConvert

QuickConvert is a Rust + GTK4/libadwaita desktop utility for **Imperial ↔ Metric** unit conversion. It targets first-year engineering workflows: length, weight, temperature, volume, and speed, with precise decimal arithmetic via [`rust_decimal`](https://docs.rs/rust_decimal).

Runs on **Linux** (KDE/GNOME) and **Windows** (64-bit).

## Features

- Imperial and Metric selectors with one **startup default** preference
- Categories: Length, Area, Weight, Volume, Speed, Force, Pressure, Energy, Power, Temperature, Angle
- Input/output unit dropdowns with **Swap Units**
- Configurable decimal precision (2–6 places, half-up rounding)
- Input validation (rejects scientific notation and extreme magnitudes)
- Preferences stored in `~/.config/quickconvert/preferences.json`
- KDE application menu entry via `.desktop` file
- **System tray** on Linux (KDE StatusNotifierItem; Wayland and X11)
- **Minimize to tray** when closing or pressing **Esc** (default; toggle in Preferences)
- **Global keyboard shortcut** on KDE Plasma (default **Meta+Shift+C**; configurable in Preferences)

## Requirements

### Build dependencies (Debian/Ubuntu)

```bash
sudo apt install build-essential pkg-config libgtk-4-dev libadwaita-1-dev libdbus-1-dev
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

### Windows (download or build)

**Pre-built binaries:** see [GitHub Releases](https://github.com/jcgit1/QuickConvert/releases). Download `quickconvert-windows-x64.zip`, extract the folder, and run `quickconvert.exe`.

CI builds the Windows zip automatically when a version tag is pushed (for example `v0.1.0`).

**Build on Windows with MSYS2:**

1. Install [MSYS2](https://www.msys2.org/) and open the **MINGW64** shell.
2. Install dependencies:

```bash
pacman -S --needed mingw-w64-x86_64-toolchain mingw-w64-x86_64-pkgconf \
  mingw-w64-x86_64-gtk4 mingw-w64-x86_64-libadwaita
```

3. Install Rust (GNU toolchain): https://rustup.rs/ — choose the `stable-gnu` target.
4. Build and package:

```bash
export PKG_CONFIG_PATH=/mingw64/lib/pkgconfig
export PATH="/mingw64/bin:$PATH"
cargo build --release
bash scripts/package-windows.sh
```

The portable zip is at `dist/quickconvert-windows-x64.zip`.

Preferences on Windows: `%APPDATA%\com\quickconvert\QuickConvert\preferences.json`

### Windows performance

GTK4 apps on Windows can stutter when **dragging an unfocused window** (a known GTK issue; animations make it worse). QuickConvert mitigates this by disabling GTK animations on Windows, debouncing UI updates, and avoiding unnecessary dropdown rebuilds.

If the window still feels sluggish while moving:

1. Use a **release** build (`cargo build --release`), not debug.
2. Launch via `quickconvert.bat` from the release zip (sets `GSK_RENDERER=ngl` when unset).
3. **Click the window once** to focus it before dragging, instead of dragging immediately from the title bar.

## Build

```bash
cargo build --release
```

The binary is `target/release/quickconvert`.

## Install

### User install (recommended)

```bash
cargo build --release
bash scripts/install-icons.sh
install -Dm755 target/release/quickconvert ~/.local/bin/quickconvert
install -Dm644 quickconvert.desktop ~/.local/share/applications/quickconvert.desktop
install -Dm644 quickconvert.desktop ~/.local/share/kglobalaccel/com.quickconvert.QuickConvert.desktop
update-desktop-database ~/.local/share/applications 2>/dev/null || true
kbuildsycoca6 --noincremental 2>/dev/null || true
```

The copy under `kglobalaccel/` registers the global shortcut (default **Meta+Shift+C**) with KDE Plasma. Change it in app **Preferences → Global shortcut** or in System Settings → Keyboard → Shortcuts.

Ensure `~/.local/bin` is on your `PATH`.

### System install

```bash
cargo build --release
sudo install -Dm755 target/release/quickconvert /usr/local/bin/quickconvert
sudo bash scripts/install-icons.sh
sudo install -Dm644 quickconvert.desktop /usr/local/share/applications/quickconvert.desktop
sudo install -Dm644 quickconvert.desktop /usr/local/share/kglobalaccel/com.quickconvert.QuickConvert.desktop
sudo kbuildsycoca6 --noincremental 2>/dev/null || true
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

## Publishing a release (maintainers)

Push a version tag; GitHub Actions builds Linux and Windows artifacts and attaches them to the release:

```bash
git tag v0.1.0
git push origin v0.1.0
```

Workflows: `.github/workflows/ci.yml` (every push) and `.github/workflows/release.yml` (tags).

**Latest release:** [v0.1.1](https://github.com/jcgit1/QuickConvert/releases/tag/v0.1.1) — includes `quickconvert-windows-x64.zip` and `quickconvert-linux-x64.tar.gz`.

## TODO & feature requests

See [TODO.md](TODO.md). Highlights:

- Windows code signing for trusted installs (SmartScreen / enterprise policy)

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
    ├── tray.rs             # KDE system tray (Linux)
    ├── shortcut.rs         # KDE global shortcut (Linux)
    └── error.rs            # user-facing errors
```

## License

This project is licensed under the **GNU Affero General Public License v3.0** (AGPL-3.0). See [LICENSE](LICENSE).
