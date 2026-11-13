#!/usr/bin/env bash
set -euo pipefail

QENA_DIR="${QENA_DIR:-$HOME/Projeler/qena}"
BIN_DIR="${BIN_DIR:-$HOME/.local/bin}"
UNIT_DIR="${UNIT_DIR:-$HOME/.config/systemd/user}"

if [[ ! -f "$QENA_DIR/Cargo.toml" ]]; then
  echo "Qena deposu bulunamadı: $QENA_DIR" >&2
  echo "QENA_DIR=/ yolunu ver." >&2
  exit 1
fi

echo "Derleniyor: $QENA_DIR"
cargo build --release --manifest-path "$QENA_DIR/Cargo.toml"

install -d "$BIN_DIR" "$UNIT_DIR"
install -m 755 "$QENA_DIR/target/release/qena" "$BIN_DIR/qena"

sed \
  -e "s|@QENA_DIR@|$QENA_DIR|g" \
  -e "s|@BIN@|$BIN_DIR/qena|g" \
  "$QENA_DIR/packaging/qena.service.in" > "$UNIT_DIR/qena.service"

systemctl --user daemon-reload
echo "Kuruldu: $BIN_DIR/qena"
echo "Başlat: systemctl --user enable --now qena.service"
echo "Durum:  systemctl --user status qena.service"
