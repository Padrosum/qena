# Qena mimarisi

Bu belge, Project Qena'nın teknik taslağıdır. Amaç; çevrimdışı çalışan, tek
makinede kurulabilen, kaynaklarını açıkça seçen ve veriyi kendi dizininde tutan
bir bilgi arşivi kurmak.

## 1. Sınırlar

- **Motorda ağ yok.** İndirme ayrı bir bileşendir (`qena-fetch`). Dizinleyici
  (`qena-index`) ve web sunucusu (`qena-web`) yalnızca yerel dosyaları okur.
- **Kullanıcı ne indireceğini seçer.** Her kaynak bir TOML bildirimidir ve
  varsayılan olarak kapalıdır. `qena fetch` açıkça çağrılmadan veri inmez.
- **Tek ikili + veri dizini.** Rust ile derlenen `qena`; web arayüzü
  `127.0.0.1` üzerinde çalışır. Node/Python/servis bağımlılığı yoktur.
- **Lisans ve atıf kayıttadır.** Her belge kaynak, lisans, URL ve indirme
  zamanıyla saklanır; arayüzde gösterilir.

## 2. Bileşenler

| Crate | Görev | Ağ |
|---|---|---|
| `qena-core` | Yapılandırma, veri yolları, katalog (SQLite), belge modeli, lisans/atıf | Yok |
| `qena-fetch` | Vikipedi, kitap, harita, rehber ve web kaydı indiricileri | Var |
| `qena-index` | Tantivy şeması, Türkçe analiz, HTML/EPUB/PDF/Markdown metin çıkarma | Yok |
| `qena-web` | axum tabanlı yerel arayüz: arama, okuyucu, harita, rehber, kayıt kuyruğu | Yalnızca yerel |
| `qena-cli` | `qena` komutu; alt komutları yönetir | Duruma göre |

`qena-fetch` ile `qena-index` arasındaki tek bağ `data/catalog.db` ve
`data/content/` dizinidir. Böylece indirme ile dizinleme ayrı ayrı
çalıştırılabilir, biri bozulduğunda diğeri etkilenmez.

## 3. Veri akışı

```
packages/*.toml
      │  qena package fetch <id>
      ▼
sources/*.toml
      │  qena fetch <source> [--limit] [--region]
      ▼
data/content/...  +  data/catalog.db
      │  qena index [--rebuild]
      ▼
data/index/  (Tantivy)
      │  qena serve --port 8787
      ▼
http://127.0.0.1:8787
```

- `fetch` idempotenttir: aynı belge ikinci kez indirilmez; `ETag` /
  `Last-Modified` / SHA-256 ile atlanır.
- `index` artımlıdır: yalnızca yeni veya değişmiş belgeleri işler.
- `serve` hiçbir zaman indirme başlatmaz; yalnızca `web_queue` tablosundaki
  kullanıcı isteklerini gösterir.

## 4. Dizin düzeni

```
data/
├─ catalog.db                 # SQLite: kaynaklar, belgeler, sürümler, kuyruk
├─ content/
│  ├─ wiki/tr/<slug>.html
│  ├─ books/<source>/<slug>.epub
│  ├─ books/<source>/<slug>.txt
│  ├─ guides/<slug>.md
│  └─ web/<host>/<zaman>-<slug>.html
├─ maps/<bolge>.pmtiles
├─ assets/                    # MapLibre, fontlar, stil dosyaları
├─ index/                     # Tantivy
└─ licenses/<source>.txt      # lisans ve atıf metinleri
```

Varsayılan veri dizini `./data`; `--data-dir` veya `qena.toml` ile
değiştirilebilir. Dizin git'e girmez.

## 5. Katalog (SQLite)

Tablolar:

- `sources(id, category, title, kind, license, attribution, fetched_at)`
- `documents(id, source_id, kind, title, lang, url, license, path, mime,
  hash, fetched_at, revision, metadata_json)`
- `snapshots(id, document_id, captured_at, format, path, hash)`
- `fetch_runs(id, source_id, started_at, finished_at, status, items, bytes, error)`
- `web_queue(id, url, status, created_at, started_at, error)`

`documents.id` kaynak içinde kararlıdır (ör. `wikipedia-tr:Deprem`). Aynı URL
yeniden kaydedilirse yeni bir `snapshots` satırı oluşur; eski sürüm silinmez.

## 6. Arama (Tantivy)

- Motor: **Tantivy 0.27** (MIT). Hazır GPL lisanslı bir Rust arama motoru
  olmadığı için kütüphane olarak kullanılır; Qena'nın kendisi GPL-3.0'dır.
- Türkçe analiz zinciri:
  `SimpleTokenizer → LowerCaser → AsciiFoldingFilter → Stemmer(Turkish)`.
  Böylece `İstanbul/istanbul`, `şehir/sehir`, `kitaplar/kitap` eşleşir.
- Alanlar: `title` (yüksek ağırlık), `headings`, `body`, `kind`, `source`,
  `lang`, `tags`, `url`, `license`, `fetched_at`.
- Sorgu: serbest metin + alan filtreleri (`kind:kitap`, `lang:tr`,
  `source:gutenberg`). Sonuç yoksa fuzzy/prefix yedeği denenir.
- Özet: `SnippetGenerator` ile vurgulu parça; başlık ve kaynak gösterilir.
- Sıralama: BM25 + başlık/ağırlık + tazelik (web kayıtlarında).
- Dizin yeniden oluşturma: `qena index --rebuild` (veri silinmez).

## 7. Web arayüzü

- `axum` + sunucu tarafı şablonlar (Askama/Maud). JS yalnızca harita, rehber
  işaretleri ve küçük etkileşimler için; derleme adımı yok.
- Rotalar:
  - `/` arama, kategoriler, istatistik
  - `/search?q=&kind=&source=&lang=&page=`
  - `/doc/{id}` belge okuyucu
  - `/raw/{id}` orijinal dosya (HTTP Range destekli)
  - `/library`, `/library/{id}` kitap rafı ve bölümlü EPUB okuyucu
  - `/guides`, `/guides/{slug}` kontrol listeleri
  - `/maps`, `/tiles/{region}/{z}/{x}/{y}`
  - `/save` (GET form, POST kuyruk), `/status`
- Güvenlik: yalnızca `127.0.0.1`; kaydedilmiş web sayfaları sandbox'lı
  `Content-Security-Policy` ile sunulur; path traversal koruması.
- Statik varlıklar (MapLibre, fontlar, stiller, Natural Earth) `assets/`
  altındadır ve `/assets` üzerinden sunulur. Büyük veri `data/` altında kalır.

## 8. Haritalar

- **Vektör karo**: Protomaps günlük derlemesinden `pmtiles extract` ile bölge
  bazlı `PMTiles` üretimi. Veri OpenStreetMap (ODbL), üretim Protomaps.
- **Görüntüleme**: MapLibre GL JS (BSD) + yerel font glyph'leri. `qena-web`,
  `pmtiles` Rust kütüphanesiyle PMTiles dosyasını okur ve `/tiles/...` ZXY uç
  noktası sunar. Türkiye z0-15 ~1,6 GB; Orta Doğu ve Balkanlar z0-8 ~5–7 MB.
  Bölge seçildiğinde stil kaynağı `/styles/{region}/{theme}` üzerinden
  dinamik üretilir.
- **Stiller**: sokak (açık/koyu), yüksek kontrast/insani (grayscale tabanlı),
  fiziki/idari. Tam HOT sembolojisi henüz yok; yüksek kontrast afet okuması
  için ilk sürüm.
- **Stiller**:
  - `Sokak (açık)` / `Sokak (koyu)`: OSM yolları, su, bina, etiketler.
  - `Fiziki / İdari`: Natural Earth (kamu malı) ülke sınırları, kıyılar,
    nehirler, göller, yerleşimler; sokak verisi gerektirmez.
  - `İnsani`: yüksek kontrast (M3'te eklenecek).
- **Bölgeler**: Türkiye, Kıbrıs, Suriye, Irak, İran, Lübnan, Ürdün,
  İsrail-Filistin, Yunanistan, Bulgaristan. Avrupa için seçili ülke/bbox.
- **Atıf**: harita üzerinde "© OpenStreetMap katkıcıları (ODbL) · Protomaps ·
  Natural Earth" kalıcı olarak görünür.
- `pmtiles` ikilisi indirme/çıkarım aracıdır; `qena doctor` varlığını denetler.

## 9. Web kaydetme

- `qena save <url>`: monolith kütüphanesi (CC0/0BSD) ile tek dosyalık HTML
  üretir; dosya `data/content/web/<host>/` altına zaman damgasıyla yazılır.
- Web formu yalnızca `web_queue` tablosuna yazar; `qena fetch --queue` ayrı
  çalışır ve kayıtları işler. Sunucu kendi başına dışarı çıkmaz.
- Her kayıt ayrı bir belge kimliği alır; aynı URL'nin eski kayıtları korunur.
- Görüntüleme: metin çıkarımı + sandbox'lı orijinal; indirme bağlantısı.
- Varsayılan: alan adı başına gecikme; toplu kazıma yok. WARC çıktısı M3'te.

## 10. Kaynak bildirimleri

`packages/*.toml` dosyaları, bir veya daha fazla kaynağı isteğe bağlı limitlerle
birleştiren hazır indirme planlarıdır. `qena package list` bunları listeler,
`qena package show <id>` ayrıntıyı gösterir, `qena package fetch <id>` kaynakları
sırayla indirir. Paketler kaynak tanımının yerine geçmez; yalnızca plan düzeyinde
kalır.

Örnek (`sources/wikipedia-tr-curated.toml`):

```toml
[source]
id = "wikipedia-tr-curated"
category = "wikipedia"
title = "Türkçe Vikipedi — seçili maddeler"
kind = "wikipedia_api"
license = "CC-BY-SA-4.0"
attribution = "Vikipedi, özgür ansiklopedi"
enabled = false

[source.params]
lang = "tr"
titles_file = "lists/wikipedia-tr-essential.txt"
with_images = false
```

`kind` değerleri: `wikipedia_api`, `gutenberg`, `protomaps_region`,
`guide_pack`, `url_list`, `web_save`; `wikisource_api`, `standard_ebooks`,
`natural_earth` ve `zim` ileride.

Komutlar:

```bash
qena source list
qena fetch wikipedia-tr-curated --limit 50
qena fetch protomaps-turkiye
qena index
qena search "deprem çantası"
qena serve
qena save https://example.com
qena fetch --queue
```

## 11. Lisans ve atıf

- Kod: GPL-3.0-or-later. Bağımlılıklar GPL ile uyumlu (MIT, Apache-2.0,
  BSD, CC0).
- İçerik lisansları kaynak bazında:
  - Vikipedi: CC BY-SA 4.0
  - OpenStreetMap: ODbL 1.0
  - Natural Earth: kamu malı
  - Gutenberg / Standard Ebooks: kamu malı (ABD)
  - Qena rehberleri: CC BY-SA 4.0
  - Web kayıtları: sitenin kendi lisansı; kullanıcı sorumluluğunda
- Her belge için `license`, `attribution`, `source_url` alanları zorunludur.
  Lisansı bilinmeyen içerik dizinlenmez.

## 12. Linux ve taşınabilirlik

- `rustls` (OpenSSL yok), `rusqlite` bundled (sistem SQLite yok).
- Release ikilisi (glibc) derlendi ve test edildi (~34 MB). Statik musl hedefi
  `x86_64-unknown-linux-musl`; bu makinede `rustup` bulunmadığı için henüz
  doğrulanmadı.
- `packaging/qena.service.in` + `packaging/install.sh`: systemd kullanıcı
  birimi; ikili `~/.local/bin/qena`, servis `qena.service`.
- `qena doctor`: veri dizini izinleri, dizin bütünlüğü, `pmtiles` aracı,
  harita paketleri ve kuyruğu denetler.
- `qena export/import`: `.tar.zst` paket + SHA-256 manifest; `data/`,
  `sources/`, `lists/`, `guides/`, `assets/`, `docs/` ve taşınabilir
  `qena.toml` taşınır; içe aktarmada her dosya doğrulanır.

## 13. Fazlar

1. **M1 — çekirdek**: katalog, Tantivy, Türkçe arama, axum arayüzü; bir
   Vikipedi listesi + bir Gutenberg kitabı + rehberlerle uçtan uca.
2. **M2 — harita ve web**: PMTiles bölgeleri, MapLibre, `qena save`, kuyruk.
3. **M3 — genişleme**: tam Türkçe Vikipedi ZIM, kitap okuyucu, export/import,
   systemd, statik musl. Export/import ve systemd tamamlandı; ZIM, okuyucu ve
   musl açık.

Ayrıntılı kabul ölçütleri `docs/ROADMAP.md` içinde.
