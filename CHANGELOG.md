# Değişiklik günlüğü

## 0.1.0 — 2026-11-14

İlk çalışan sürüm.

### Eklenen

- Rust workspace: `qena-core`, `qena-fetch`, `qena-index`, `qena-web`,
  `qena-cli`.
- SQLite katalog ve `data/` içerik deposu.
- Tantivy tam metin arama; Türkçe stemmer ve aksan katlama.
- Vikipedi API indiricisi (seçili maddeler, 429 geri çekilmesi).
- Project Gutenberg EPUB/TXT indiricisi.
- 10 özgün afet/hazırlık rehberi.
- PMTiles bölge indiricisi; MapLibre arayüzü.
  - Türkiye z0-15
  - Orta Doğu z0-8
  - Balkanlar z0-8
  - Sokak (açık/koyu), insani (HOT esinli), fiziki/idari stiller.
- axum web arayüzü: arama, belge okuyucu, bölümlü EPUB okuyucu, kitap rafı,
  rehberler, harita, durum.
- monolith ile tek dosyalık web kaydı; URL kuyruğu ve `qena fetch --queue`.
- `qena export` / `qena import`: `.tar.zst` + SHA-256 manifest.
- `packaging/install.sh` ve systemd kullanıcı birimi.
- `scripts/backup.sh` ve `docs/BACKUP.md`.
- `qena doctor`, `qena status`.
- `qena package list/show/fetch`: hazır veri paketleri (çekirdek, hızlı
  başlangıç, haritalar, kitaplar, rehberler).

### Düzeltilen

- Harita karoları MapLibre için mutlak URL’ye çevrildi; göreli `/tiles/...`
  adresleri `Request constructor` hatası veriyordu.
- `assets/` dizini çalışma klasöründen bağımsız hale getirildi
  (`assets_dir`).
- Wikipedia 429 yanıtlarında `Retry-After` ile bekleme.

### Açık

- Tam Türkçe Vikipedi ZIM içe aktarma.
- Tam HOT sembolojisi ve z15+ diğer bölgeler.
- Statik musl ikili.
- Daha geniş küratörlü listeler.
