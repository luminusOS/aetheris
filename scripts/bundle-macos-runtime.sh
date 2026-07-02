#!/usr/bin/env bash
set -euo pipefail

app_bundle="${1:?usage: scripts/bundle-macos-runtime.sh path/to/Aetheris.app}"
brew_prefix="${2:-$(brew --prefix)}"

if [ ! -d "$app_bundle/Contents/MacOS" ]; then
  echo "Not a macOS app bundle: $app_bundle" >&2
  exit 1
fi

frameworks="$app_bundle/Contents/Frameworks"
resources="$app_bundle/Contents/Resources"
share="$resources/share"
mkdir -p "$frameworks" "$share"

launcher="$app_bundle/Contents/MacOS/aetheris"
main_binary="$app_bundle/Contents/MacOS/aetheris-bin"
if [ ! -f "$launcher" ] && [ ! -f "$main_binary" ]; then
  echo "Missing app executable: $launcher" >&2
  exit 1
fi

if [ ! -f "$main_binary" ]; then
  mv "$launcher" "$main_binary"
fi

cat > "$launcher" <<'EOF'
#!/bin/bash
DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BUNDLE_ROOT="$(dirname "$DIR")"
RESOURCES="$BUNDLE_ROOT/Resources"

export PATH="/usr/bin:/bin:/usr/sbin:/sbin"
export DYLD_LIBRARY_PATH="$BUNDLE_ROOT/Frameworks"
export XDG_DATA_DIRS="$RESOURCES/share"
export GTK_DATA_PREFIX="$RESOURCES"
export GDK_PIXBUF_MODULE_FILE="$RESOURCES/lib/gdk-pixbuf-2.0/2.10.0/loaders.cache"
export GIO_MODULE_DIR="$RESOURCES/lib/gio/modules"

exec "$DIR/aetheris-bin" "$@"
EOF
chmod +x "$launcher"

copy_tree() {
  local src="$1"
  local dest="$2"
  if [ -d "$src" ]; then
    rm -rf "$dest"
    mkdir -p "$(dirname "$dest")"
    cp -RL "$src" "$dest"
  fi
}

copy_dylib() {
  local src="$1"
  local name dest
  name="$(basename "$src")"
  dest="$frameworks/$name"
  if [ -f "$dest" ]; then
    return 1
  fi
  cp "$src" "$dest"
  chmod u+w "$dest"
  install_name_tool -id "@executable_path/../Frameworks/$name" "$dest" 2>/dev/null || true
  return 0
}

fix_binary() {
  local binary="$1"
  local new_libs=()
  local rpath
  rpath="@loader_path/$(python3 -c 'import os, sys; print(os.path.relpath(sys.argv[1], os.path.dirname(sys.argv[2])))' "$frameworks" "$binary")"

  while IFS= read -r lib; do
    [ -n "$lib" ] || continue
    local name src
    name="$(basename "$lib")"
    src="$lib"

    if [[ "$lib" == @rpath/* || "$lib" == @loader_path/* ]]; then
      src="$brew_prefix/lib/$name"
    fi

    if [ -f "$src" ]; then
      if copy_dylib "$src"; then
        new_libs+=("$frameworks/$name")
      fi
      install_name_tool -change "$lib" "@rpath/$name" "$binary" 2>/dev/null || true
    fi
  done < <(otool -L "$binary" 2>/dev/null | awk -v brew="$brew_prefix" '$1 ~ brew || $1 ~ /^@rpath\// || $1 ~ /^@loader_path\// {print $1}')

  if ! otool -l "$binary" 2>/dev/null | grep -q "$rpath"; then
    install_name_tool -add_rpath "$rpath" "$binary" 2>/dev/null || true
  fi

  printf '%s\n' "${new_libs[@]}"
}

queue=()
while IFS= read -r lib; do
  [ -n "$lib" ] && queue+=("$lib")
done < <(fix_binary "$main_binary")

pass=0
while [ "${#queue[@]}" -gt 0 ]; do
  pass=$((pass + 1))
  if [ "$pass" -gt 30 ]; then
    echo "Stopping dylib recursion after 30 passes." >&2
    break
  fi

  next=()
  for lib in "${queue[@]}"; do
    while IFS= read -r new_lib; do
      [ -n "$new_lib" ] && next+=("$new_lib")
    done < <(fix_binary "$lib")
  done
  queue=("${next[@]}")
done

copy_tree "$brew_prefix/share/glib-2.0/schemas" "$share/glib-2.0/schemas"
copy_tree "$brew_prefix/share/gtk-4.0" "$share/gtk-4.0"
copy_tree "$brew_prefix/share/gtksourceview-5" "$share/gtksourceview-5"
copy_tree "$brew_prefix/share/icons/hicolor" "$share/icons/hicolor"
copy_tree "$brew_prefix/share/icons/Adwaita" "$share/icons/Adwaita"
copy_tree "$brew_prefix/share/themes" "$share/themes"
copy_tree "$brew_prefix/lib/gdk-pixbuf-2.0" "$resources/lib/gdk-pixbuf-2.0"
copy_tree "$brew_prefix/lib/gio" "$resources/lib/gio"
if [ -d "data/icons/hicolor" ]; then
  mkdir -p "$share/icons/hicolor"
  cp -R data/icons/hicolor/. "$share/icons/hicolor/"
fi

while IFS= read -r plugin; do
  [ -n "$plugin" ] && fix_binary "$plugin" >/dev/null
done < <(find "$resources/lib" \( -name '*.so' -o -name '*.dylib' \) 2>/dev/null)

if command -v glib-compile-schemas >/dev/null 2>&1 && [ -d "$share/glib-2.0/schemas" ]; then
  glib-compile-schemas "$share/glib-2.0/schemas" 2>/dev/null || true
fi

if command -v gtk4-update-icon-cache >/dev/null 2>&1; then
  gtk4-update-icon-cache -f -t "$share/icons/hicolor" 2>/dev/null || true
  gtk4-update-icon-cache -f -t "$share/icons/Adwaita" 2>/dev/null || true
fi

find "$frameworks" -name '*.dylib' -exec codesign --force --sign - {} \; 2>/dev/null || true
find "$resources/lib" \( -name '*.so' -o -name '*.dylib' \) -exec codesign --force --sign - {} \; 2>/dev/null || true
codesign --force --sign - "$main_binary" 2>/dev/null || true
codesign --force --sign - "$launcher" 2>/dev/null || true
codesign --force --deep --sign - "$app_bundle" 2>/dev/null || true
