#!/usr/bin/env sh
set -eu

domain="org.luminusos.Aetheris"
prefix="${1:-po}"

while IFS= read -r lang; do
  case "$lang" in
    ""|\#*) continue ;;
  esac

  install_dir="${prefix}/${lang}/LC_MESSAGES"
  mkdir -p "$install_dir"
  msgfmt "po/${lang}.po" -o "${install_dir}/${domain}.mo"
done < po/LINGUAS
