#!/usr/bin/env bash
set -euo pipefail

output="${1:-target/macos/aetheris.icns}"
iconset="${2:-target/macos/aetheris.iconset}"

rm -rf "$iconset"
mkdir -p "$iconset" "$(dirname "$output")"

copy_icon() {
  local src="$1"
  local dest="$2"
  if [ ! -f "$src" ]; then
    echo "Missing icon source: $src" >&2
    exit 1
  fi
  cp "$src" "$iconset/$dest"
}

copy_icon data/icons/hicolor/512x512/apps/org.luminusos.Aetheris.png icon_512x512.png
copy_icon data/icons/hicolor/512x512/apps/org.luminusos.Aetheris.png icon_256x256@2x.png
copy_icon data/icons/hicolor/256x256/apps/org.luminusos.Aetheris.png icon_256x256.png
copy_icon data/icons/hicolor/256x256/apps/org.luminusos.Aetheris.png icon_128x128@2x.png
copy_icon data/icons/hicolor/128x128/apps/org.luminusos.Aetheris.png icon_128x128.png
copy_icon data/icons/hicolor/128x128/apps/org.luminusos.Aetheris.png icon_64x64@2x.png
copy_icon data/icons/hicolor/64x64/apps/org.luminusos.Aetheris.png icon_64x64.png
copy_icon data/icons/hicolor/64x64/apps/org.luminusos.Aetheris.png icon_32x32@2x.png

if command -v magick >/dev/null 2>&1; then
  magick data/icons/hicolor/512x512/apps/org.luminusos.Aetheris.png -resize 1024x1024 "$iconset/icon_512x512@2x.png"
  magick data/icons/hicolor/64x64/apps/org.luminusos.Aetheris.png -resize 32x32 "$iconset/icon_32x32.png"
  magick data/icons/hicolor/64x64/apps/org.luminusos.Aetheris.png -resize 16x16 "$iconset/icon_16x16.png"
  magick data/icons/hicolor/64x64/apps/org.luminusos.Aetheris.png -resize 32x32 "$iconset/icon_16x16@2x.png"
else
  copy_icon data/icons/hicolor/512x512/apps/org.luminusos.Aetheris.png icon_512x512@2x.png
  copy_icon data/icons/hicolor/64x64/apps/org.luminusos.Aetheris.png icon_32x32.png
  copy_icon data/icons/hicolor/64x64/apps/org.luminusos.Aetheris.png icon_16x16.png
  copy_icon data/icons/hicolor/64x64/apps/org.luminusos.Aetheris.png icon_16x16@2x.png
fi

iconutil -c icns "$iconset" -o "$output"
