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
  - `/library`, `/library/{id}` kitap rafı
  - `/guides`, `/guides/{slug}` kontrol listeleri
  - `/maps`, `/tiles/{region}/{z}/{x}/{y}.mvt`
  - `/save` (GET form, POST kuyruk), `/status`
- Güvenlik: yalnızca `127.0.0.1`; `Content-Security-Policy`; kaydedilmiş web
  sayfaları sandbox'lı iframe içinde ve varsayılan olarak scriptsiz gösterilir;
  path traversal koruması.
- Statik dosyalar `include_dir!` ile ikiliye gömülür; harita/font gibi büyük
  varlıklar `data/assets/` altında durur.

## 8. Haritalar

- **Vektör karo**: Protomaps günlük derlemesinden `pmtiles extract` ile bölge
  bazlı `PMTiles` üretimi. Veri OpenStreetMap (ODbL), üretim Protomaps.
- **Görüntüleme**: MapLibre GL JS (BSD) + yerel font glyph'leri. `qena-web`,
  PMTiles dosyasını HTTP Range ile okur ve `/tiles/...` ZXY uç noktası sunar.
- **Stiller**:
  - `Sokak (açık)` / `Sokak (koyu)`: OSM yolları, su, bina, etiketler.
  - `İnsani`: yüksek kontrast, afet okunabilirliği.
  - `Fiziki / İdari`: Natural Earth (kamu malı) ülke sınırları, kıyılar,
    nehirler, göller, yerleşimler; sokak verisi gerektirmez.
- **Bölgeler**: Türkiye, Kıbrıs, Suriye, Irak, İran, Lübnan, Ürdün,
  İsrail-Filistin, Yunanistan, Bulgaristan. Avrupa için seçili ülke/bbox.
- **Atıf**: harita üzerinde "© OpenStreetMap katkıcıları (ODbL) · Protomaps ·
  Natural Earth" kalıcı olarak görünür.
- `pmtiles` ikilisi derleme/indirme aracıdır; `qena doctor` varlığını denetler.
  İleride yerel Rust çıkarımına geçilebilir.

## 9. Web kaydetme

- `qena save <url>`: monolith kütüphanesi (CC0) ile tek dosyalık HTML ve
  isteğe bağlı WARC üretir.
- Aynı URL'nin her kaydı zaman damgalı bir `snapshot` olur; eski sürüm korunur.
- Kuyruk: web formundan gelen istekler `web_queue` tablosuna yazılır; ayrı bir
  `qena fetch --queue` çalışması işler. Sunucu kendi başına dışarı çıkmaz.
- Varsayılan: `robots.txt`'e saygı, alan adı başına gecikme, eşzamanlılık 1.
- Görüntüleme: metin çıkarımı + sandbox'lı orijinal; indirme bağlantısı.

## 10. Kaynak bildirimleri

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

`kind` değerleri: `wikipedia_api`, `wikisource_api`, `gutenberg`,
`standard_ebooks`, `protomaps_region`, `natural_earth`, `guide_pack`, `url_list`,
`web_save`, `zim` (ileride).

Komutlar:

```bash
qena source list
qena fetch wikipedia-tr-curated --limit 50
qena fetch --category maps --source osm-turkey
qena index
qena search "deprem çantası"
qena serve
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
- Statik musl hedefi: `x86_64-unknown-linux-musl`; tek ikili.
- `packaging/qena.service`: systemd kullanıcı birimi.
- `qena doctor`: veri dizini izinleri, dizin bütünlüğü, harita araçları,
  fontlar ve disk alanını denetler.
- `qena export/import`: `.tar.zst` paket + SHA-256 manifest; USB'ye kopyalanır.

## 13. Fazlar

1. **M1 — çekirdek**: katalog, Tantivy, Türkçe arama, axum arayüzü; bir
   Vikipedi listesi + bir Gutenberg kitabı + rehberlerle uçtan uca.
2. **M2 — harita ve web**: PMTiles bölgeleri, MapLibre, `qena save`, kuyruk.
3. **M3 — genişleme**: tam Türkçe Vikipedi ZIM, kitap rafı, export/import,
   systemd, doctor.

Ayrıntılı kabul ölçütleri `docs/ROADMAP.md` içinde.
