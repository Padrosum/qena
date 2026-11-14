#!/usr/bin/env bash
set -euo pipefail

QENA_DIR="${QENA_DIR:-$HOME/Projeler/qena}"
DEST="${1:-$HOME/qena-yedek}"
KEEP="${QENA_BACKUP_KEEP:-5}"
BIN="$QENA_DIR/target/release/qena"

if [[ ! -x "$BIN" ]]; then
  echo "Qena ikilisi yok: $BIN" >&2
  echo "Önce: cargo build --release --manifest-path \"$QENA_DIR/Cargo.toml\"" >&2
  exit 1
fi

mkdir -p "$DEST"
STAMP="$(date +%Y%m%d-%H%M%S)"
OUT="$DEST/qena-$STAMP.tar.zst"

echo "Yedek alınıyor: $OUT"
(
  cd "$QENA_DIR"
  "$BIN" --config "$QENA_DIR/qena.toml" export "$OUT"
)

sha256sum "$OUT" > "$OUT.sha256"
echo "SHA-256: $OUT.sha256"

mapfile -t OLD < <(ls -1t "$DEST"/qena-*.tar.zst 2>/dev/null | tail -n +$((KEEP + 1)))
for file in "${OLD[@]:-}"; do
  [[ -z "$file" ]] && continue
  echo "Eski yedek siliniyor: $file"
  rm -f "$file" "$file.sha256"
done

echo "Son $KEEP yedek tutuluyor: $DEST"
