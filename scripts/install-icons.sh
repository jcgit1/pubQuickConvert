#!/usr/bin/env bash
# Install QuickConvert icons into the user icon theme (hicolor).
set -euo pipefail

ICON_ID="com.quickconvert.QuickConvert"
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SRC="${ROOT}/data/icons/hicolor"
DEST="${XDG_DATA_HOME:-${HOME}/.local/share}/icons/hicolor"

if [[ ! -d "${SRC}" ]]; then
  echo "Missing ${SRC}. Run from a full checkout." >&2
  exit 1
fi

for size_dir in "${SRC}"/*/apps; do
  [[ -d "${size_dir}" ]] || continue
  size="$(basename "$(dirname "${size_dir}")")"
  install -Dm644 "${size_dir}/${ICON_ID}.png" "${DEST}/${size}/apps/${ICON_ID}.png"
done

if command -v gtk-update-icon-cache >/dev/null 2>&1; then
  gtk-update-icon-cache -f -t "${DEST}" 2>/dev/null || true
fi

echo "Installed icons to ${DEST}"
