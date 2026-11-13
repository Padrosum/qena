# Linux kurulumu

## Kullanıcı servisi

```bash
cd ~/Projeler/qena
./packaging/install.sh
systemctl --user enable --now qena.service
```

Varsayılanlar:

- Depo: `~/Projeler/qena` (`QENA_DIR` ile değiştirilir)
- İkili: `~/.local/bin/qena` (`BIN_DIR`)
- Servis: `~/.config/systemd/user/qena.service`
- Adres: `http://127.0.0.1:8787`

Farklı dizinler için:

```bash
QENA_DIR=/opt/qena BIN_DIR=$HOME/.local/bin ./packaging/install.sh
```

## Paketleme

```bash
qena export qena-arsiv.tar.zst
qena export qena-arsiv.tar.zst --with-binary
qena import qena-arsiv.tar.zst --into ./qena-arsiv
cd qena-arsiv
qena --config qena.toml serve
```

Paket; `data/`, `sources/`, `lists/`, `guides/`, `assets/`, `docs/` ve
taşınabilir `qena.toml` içerir. `MANIFEST.json` her dosyanın SHA-256'sını
taşır; içe aktarma sırasında doğrulanır. Tutarlı bir paket için önce
`qena serve` sürecini durdur.

## Statik derleme

glibc ikilisi:

```bash
cargo build --release
```

musl hedefi kuruluysa:

```bash
rustup target add x86_64-unknown-linux-musl
cargo build --release --target x86_64-unknown-linux-musl
```

`rusqlite` bundled, `rustls` ve `zstd` C derlemeleri nedeniyle musl için
`musl-gcc` gerekebilir. Bu hedef bu makinede henüz doğrulanmadı; glibc
release ikilisi test edildi.
