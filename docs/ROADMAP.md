# Yol haritası

## M0 — karar ve iskelet (şimdi)

- [x] Yeni proje adı ve kapsamı
- [x] Mimari taslağı
- [x] Kaynak ve lisans listesi
- [ ] İlk veri paketi ve arama motoru kararı
- [ ] Depo iskeletinin kurulması

## M1 — çalışan çekirdek

Amaç: `qena fetch` → `qena index` → `qena serve` zincirinin uçtan uca
çalışması.

- [ ] Cargo workspace, `qena.toml`, veri dizini
- [ ] SQLite katalog ve belge modeli
- [ ] Kaynak bildirimi ayrıştırıcı + `qena source list`
- [ ] Vikipedi indirici (seçili başlıklar, HTML → metin)
- [ ] Gutenberg indirici (Gutendex, EPUB/TXT)
- [ ] Rehber paketi (10 özgün kontrol listesi)
- [ ] Tantivy dizini + Türkçe analiz + artımlı indeksleme
- [ ] axum: arama, belge okuyucu, durum sayfası
- [ ] `qena doctor`, yapılandırılmış günlük, testler
- [ ] Örnek veriyle uçtan uca deneme

**Kabul:** ağ kapalıyken `qena serve` açılır; "deprem çantası", "istanbul",
"kitap" sorguları sonuç verir; her sonuçta kaynak + lisans görünür.

## M2 — harita ve web kaydı

- [ ] `pmtiles extract` ile bölge paketleri (Türkiye + komşular)
- [ ] PMTiles okuma ve `/tiles/...` uç noktası
- [ ] MapLibre + yerel font/stil; sokak, insani, fiziki/idari stiller
- [ ] Natural Earth katmanları
- [ ] `qena save` (monolith): tek dosya HTML + WARC
- [ ] Web kuyruğu ve snapshot görüntüleyici
- [ ] systemd kullanıcı birimi

**Kabul:** internet kapalıyken harita kaydırılır/yakınlaşır; kaydedilmiş bir
sayfa açılır; yeni URL kuyruğa girip işlenir.

## M3 — genişleme ve taşınabilirlik

- [ ] Tam Türkçe Vikipedi ZIM içe aktarma
- [ ] Kitap rafı ve okuyucu (EPUB/PDF)
- [ ] `qena export` / `qena import` (.tar.zst + SHA-256)
- [ ] Statik musl derleme ve kurulum betiği
- [ ] Yedekleme ve sürüm notları

**Kabul:** temiz bir makinede tek ikili + veri paketiyle arama, okuma ve harita
çalışır; ağ gerekmez.

## Ölçütler

- `cargo test` ve `cargo clippy -- -D warnings` temiz.
- `qena-index` ve `qena-web` kodunda ağ çağrısı yok (kod incelemesi + test).
- Her belgede `license`, `attribution`, `source_url` dolu.
- Türkçe arama: `İ/ı/ş/ğ/ü/ö/ç` katlanır; ekler stemmer ile eşleşir.
- Veri dizini yeniden başlatmada korunur; dizin bozulursa `--rebuild` ile
  yeniden üretilir.
- Lisansı bilinmeyen kaynak indirilmez.
