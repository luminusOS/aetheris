#!/usr/bin/env sh
set -eu

domain="org.luminusos.Aetheris"
pot="po/${domain}.pot"

xgettext \
  --from-code=UTF-8 \
  --keyword=tr \
  --keyword=tr_format:1 \
  --keyword=trn:1,2 \
  --add-comments=Translators \
  --package-name=Aetheris \
  --output="$pot" \
  --files-from=po/POTFILES.in

while IFS= read -r lang; do
  case "$lang" in
    ""|\#*) continue ;;
  esac

  po_file="po/${lang}.po"
  if [ -f "$po_file" ]; then
    msgmerge --update --backup=none "$po_file" "$pot"
  else
    msginit --no-translator --locale="$lang" --input="$pot" --output-file="$po_file"
  fi
done < po/LINGUAS
