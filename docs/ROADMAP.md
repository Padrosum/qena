# Yol haritası

## M0 — karar ve iskelet

- [x] Yeni proje adı ve kapsamı
- [x] Mimari taslağı
- [x] Kaynak ve lisans listesi
- [x] İlk veri paketi ve arama motoru kararı
- [x] Depo iskeletinin kurulması

## M1 — çalışan çekirdek

Amaç: `qena fetch` → `qena index` → `qena serve` zincirinin uçtan uca
çalışması.

- [x] Cargo workspace, `qena.toml`, veri dizini
- [x] SQLite katalog ve belge modeli
- [x] Kaynak bildirimi ayrıştırıcı + `qena source list`
- [x] Vikipedi indirici (seçili başlıklar, HTML → metin)
- [x] Gutenberg indirici (doğrudan EPUB/TXT)
- [x] Rehber paketi (10 özgün kontrol listesi)
- [x] Tantivy dizini + Türkçe analiz + artımlı indeksleme
- [x] axum: arama, belge okuyucu, durum sayfası
- [x] `qena doctor`, yapılandırılmış günlük, testler
- [x] Örnek veriyle uçtan uca deneme

**Kabul:** ağ kapalıyken `qena serve` açılır; "deprem çantası", "istanbul",
"kitap" sorguları sonuç verir; her sonuçta kaynak + lisans görünür.

## M2 — harita ve web kaydı

- [x] `pmtiles extract` ile bölge paketleri (Türkiye z0-14, Orta Doğu ve
      Balkanlar z0-8)
- [x] PMTiles okuma ve `/tiles/...` uç noktası
- [x] MapLibre + yerel font/stil; sokak (açık/koyu), fiziki/idari
- [x] Bölge seçici ve bölgeye göre dinamik stil
- [x] Natural Earth katmanları
- [x] `qena save` (monolith): tek dosya HTML
- [x] Web kuyruğu ve snapshot görüntüleyici
- [x] systemd kullanıcı birimi + `packaging/install.sh`
- [x] z14 Türkiye paketi ve yüksek kontrast/insani stil
- [ ] Tam HOT sembolojisi ve daha yüksek zoom (z15+)

**Kabul:** internet kapalıyken harita kaydırılır/yakınlaşır; kaydedilmiş bir
sayfa açılır; yeni URL kuyruğa girip işlenir.

## M3 — genişleme ve taşınabilirlik

- [ ] Tam Türkçe Vikipedi ZIM içe aktarma
- [x] Kitap okuyucu (EPUB bölümleri + içindekiler; PDF indirme)
- [x] `qena export` / `qena import` (.tar.zst + SHA-256)
- [ ] Statik musl derleme (glibc release derlemesi test edildi)
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
