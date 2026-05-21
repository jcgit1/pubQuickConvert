#!/usr/bin/env bash
# Bundle quickconvert.exe and runtime DLLs for Windows (run inside MSYS2 MINGW64).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

EXE="target/release/quickconvert.exe"
DIST_NAME="quickconvert-windows-x64"
DIST="dist/${DIST_NAME}"

if [[ ! -f "$EXE" ]]; then
  echo "Missing ${EXE} — run: cargo build --release" >&2
  exit 1
fi

rm -rf "$DIST"
mkdir -p "$DIST"

cp "$EXE" "$DIST/"

# Runtime libraries from MSYS2 MINGW64.
ldd "$EXE" | grep -i 'mingw64' | awk '{print $3}' | sort -u | while read -r dll; do
  if [[ -f "$dll" ]]; then
    cp "$dll" "$DIST/"
  fi
done

# GTK / libadwaita assets (themes, icons, schemas).
MINGW_SHARE="${MINGW_PREFIX:-/mingw64}/share"
for dir in gtk-4.0 glib-2.0 icons Adwaita; do
  if [[ -d "${MINGW_SHARE}/${dir}" ]]; then
    mkdir -p "${DIST}/share"
    cp -r "${MINGW_SHARE}/${dir}" "${DIST}/share/"
  fi
done

# GdkPixbuf loaders (image backends).
MINGW_LIB="${MINGW_PREFIX:-/mingw64}/lib"
if [[ -d "${MINGW_LIB}/gdk-pixbuf-2.0" ]]; then
  mkdir -p "${DIST}/lib"
  cp -r "${MINGW_LIB}/gdk-pixbuf-2.0" "${DIST}/lib/"
fi

cat > "${DIST}/README-WINDOWS.txt" <<'EOF'
QuickConvert for Windows
========================

1. Extract this folder anywhere (e.g. Desktop\QuickConvert).
2. Run quickconvert.exe.

Preferences are stored at:
  %APPDATA%\com\quickconvert\QuickConvert\preferences.json

Requirements: 64-bit Windows 10 or later. All needed DLLs are included
in this folder; do not delete them.

If Windows SmartScreen blocks the app, choose "More info" → "Run anyway"
(we do not sign binaries yet).

Performance tips (if the window lags while dragging):
  - Use quickconvert.exe from a release build (not debug).
  - Try setting GSK_RENDERER=ngl before launch (GPU rendering).
  - Avoid dragging from an empty area of the title bar if the app was
    unfocused; click the window once first, then move it.
EOF

cat > "${DIST}/quickconvert.bat" <<'EOF'
@echo off
setlocal
if not defined GSK_RENDERER set GSK_RENDERER=ngl
start "" "%~dp0quickconvert.exe"
EOF

mkdir -p dist
rm -f "dist/${DIST_NAME}.zip"
(
  cd dist
  if command -v zip >/dev/null 2>&1; then
    zip -rq "${DIST_NAME}.zip" "${DIST_NAME}"
  else
    powershell -Command "Compress-Archive -Path '${DIST_NAME}' -DestinationPath '${DIST_NAME}.zip' -Force"
  fi
)

echo "Created dist/${DIST_NAME}.zip"
