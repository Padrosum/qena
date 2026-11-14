# Project Qena

**Kötü günler için yerel bilgi sandığı.** İnternet, bulut ya da dışarıdaki bir
servise ihtiyaç duymadan çalışan; aranabilir, okunabilir, haritalanabilir ve
taşınabilir bir çevrimdışı arşiv.

![Rust](https://img.shields.io/badge/Rust-1.85%2B-orange)
![Lisans](https://img.shields.io/badge/lisans-GPL--3.0-blue)
![Öncelik](https://img.shields.io/badge/öncelik-çevrimdışı-success)

> Bu depo **veri içermez**. Uygulama kodu, kaynak tanımları, rehberler ve
> arayüz varlıkları buradadır; Vikipedi, kitap ve harita verisi `qena fetch`
> ile kendi makinenize indirilir. İndirilen veri `data/` altında kalır ve
> `.gitignore` ile depo dışında tutulur.

---

## İçindekiler

- [Nedir?](#nedir)
- [Özellikler](#özellikler)
- [Hızlı başlangıç](#hızlı-başlangıç)
- [Komutlar](#komutlar)
- [Hazır paketler](#hazır-paketler)
- [Kaynak tanımları](#kaynak-tanımları)
- [Mimari](#mimari)
- [Depo düzeni](#depo-düzeni)
- [Arama](#arama)
- [Haritalar](#haritalar)
- [Web kaydı](#web-kaydı)
- [Veri düzeni](#veri-düzeni)
- [Kaynak ekleme](#kaynak-ekleme)
- [Lisans ve atıf](#lisans-ve-atıf)
- [Linux kurulumu](#linux-kurulumu)
- [Taşıma: export/import](#taşıma-exportimport)
- [Yedekleme](#yedekleme)
- [Geliştirme](#geliştirme)
- [Sorun giderme](#sorun-giderme)
- [Yol haritası](#yol-haritası)
- [Sürüm notları](#sürüm-notları)
- [Gizlilik ve güvenlik](#gizlilik-ve-güvenlik)

---

## Nedir?

Qena, tek bir makinede çalışan yerel bir bilgi arşividir. Amaç; elektrik,
internet veya dış servis kesintisi olduğunda temel bilgiye ulaşmaya devam
etmektir.

Üç katmandan oluşur:

1. **İndirme** (`qena fetch`): yalnızca açıkça seçilen kaynakları indirir.
2. **Dizinleme** (`qena index`): indirilen içeriği yerel Tantivy dizinine yazar.
3. **Görüntüleme** (`qena serve`): `127.0.0.1` üzerinde çalışan web arayüzü.

Dizinleyici ve web sunucusu **ağa çıkmaz**. Ağ yalnızca `fetch` ve `save`
komutlarında kullanılır.

## Özellikler

- **Türkçe arama**: Tantivy + Türkçe stemmer + aksan katlama.
  `gida` → “Gıda”, `kitaplar` → “kitap”, `istanbul` → “İstanbul” eşleşir.
- **Hazır paketler**: `qena package list/show/fetch` ile çekirdek, hızlı
  başlangıç, kitap, rehber ve harita paketleri.
- **Vikipedi**: seçili maddeler, isteğe bağlı tam Türkçe Vikipedi ZIM (yol
  haritasında).
- **Kitaplar**: Project Gutenberg’den EPUB/TXT; bölümlü EPUB okuyucu.
- **Rehberler**: deprem, su, gıda, ilk yardım, enerji, iletişim, tahliye,
  barınma, hijyen ve belgeler için özgün kontrol listeleri.
- **Haritalar**: PMTiles + MapLibre; z15 Türkiye, sokak (açık/koyu), insani
  yüksek kontrast ve Natural Earth fiziki/idari stiller.
- **Web kaydı**: monolith ile tek dosyalık HTML snapshot; yerel kuyruk.
- **Taşınabilir paket**: `qena export` / `qena import`; SHA-256 doğrulamalı
  `.tar.zst` arşivi.
- **Yedekleme**: `scripts/backup.sh` ile zaman damgalı, döngüsel yedek.
- **Tek ikili**: Rust + axum; Node, Python veya ayrı veritabanı servisi yok.
- **Yerel**: varsayılan olarak yalnızca `127.0.0.1:8787`.

## Hızlı başlangıç

### Gereksinimler

- Linux (Arch, Debian/Ubuntu, Fedora…)
- Rust 1.85+ (`cargo`, `rustc`)
- Harita indirmek için `pmtiles` ikilisi
- Demo paket için ~2 GB, z15 haritalar ve daha fazla kitap için daha fazla disk

`pmtiles` kurulumu (Linux x86_64, sabit sürüm):

```bash
curl -sSL -o /tmp/pmtiles.tar.gz \
  https://github.com/protomaps/go-pmtiles/releases/download/v1.31.2/go-pmtiles_1.31.2_Linux_x86_64.tar.gz
tar -xzf /tmp/pmtiles.tar.gz -C /tmp
install -m 755 /tmp/pmtiles ~/.local/bin/pmtiles
pmtiles version
```

### Derleme

```bash
git clone https://github.com/Padrosum/qena.git
cd qena
cargo build --release
./target/release/qena init
```

`qena init` şunları oluşturur:

- `qena.toml` (yoksa)
- `data/` ve alt dizinleri
- `data/catalog.db` SQLite kataloğu

### İlk içerik

En hızlı yol, hazır bir paket indirmektir:

```bash
./target/release/qena package list
./target/release/qena package fetch hizli-baslangic
./target/release/qena index
./target/release/qena search "deprem çantası"
./target/release/qena serve
```

`hizli-baslangic` yaklaşık 20 MB’dır: 10 Vikipedi maddesi, 3 kitap, 10 rehber
ve Orta Doğu haritası. Daha büyük paketler için `qena package show cekirdek`.

Tek tek kaynak seçmek isterseniz:

```bash
./target/release/qena source list

./target/release/qena fetch wikipedia-tr-curated --limit 20
./target/release/qena fetch gutenberg-secili --limit 5
./target/release/qena fetch rehberler
./target/release/qena fetch protomaps-turkiye

./target/release/qena index
./target/release/qena search "deprem çantası"
./target/release/qena serve
```

Ardından `http://127.0.0.1:8787` adresini normal bir tarayıcıda açın.

## Komutlar

| Komut | Açıklama |
|---|---|
| `qena init` | `qena.toml`, veri dizini ve kataloğu hazırlar |
| `qena source list` | `sources/*.toml` kaynaklarını listeler |
| `qena package list` | `packages/*.toml` hazır paketlerini listeler |
| `qena package show <id>` | Paket ayrıntılarını ve kaynaklarını gösterir |
| `qena package fetch <id>` | Paketteki kaynakları indirir |
| `qena fetch <id>` | Tek bir kaynağı indirir |
| `qena fetch --category maps` | Kategorideki tüm kaynakları indirir |
| `qena fetch --queue` | Web formundan gelen URL kuyruğunu işler |
| `qena index` | Yeni/değişen belgeleri dizinler |
| `qena index --rebuild` | Dizini sıfırdan kurar |
| `qena search "sorgu"` | Terminalde arar |
| `qena serve` | Yerel web arayüzünü başlatır |
| `qena save <url>` | Tek URL’yi hemen arşive kaydeder |
| `qena export <dosya>` | Veriyi `.tar.zst` paketine dönüştürür |
| `qena import <dosya>` | Paketi açar ve SHA-256 ile doğrular |
| `qena doctor` | Dizin, katalog, harita araçları ve varlıkları denetler |
| `qena status` | Belge, dizin ve kuyruk özeti |

Yararlı bayraklar:

```bash
qena fetch <id> --limit 20 --dry-run
qena fetch <id> --force        # var olan dosyayı yenile
qena search "su" --kind guide --lang tr --limit 5
qena serve --port 8787 --host 127.0.0.1
```

## Kaynak tanımları

Depoda hazır gelen kaynak tanımları:

| Kaynak | Kategori | Tür | İçerik |
|---|---|---|---|
| `wikipedia-tr-curated` | `wikipedia` | `wikipedia_api` | 62 seçili Türkçe madde |
| `gutenberg-secili` | `books` | `gutenberg` | 20 kamu malı kitap |
| `rehberler` | `guides` | `guide_pack` | 10 özgün kontrol listesi |
| `protomaps-turkiye` | `maps` | `protomaps_region` | Türkiye z0-15 (~1,6 GB) |
| `protomaps-orta-dogu` | `maps` | `protomaps_region` | Orta Doğu z0-8 (~5 MB) |
| `protomaps-balkanlar` | `maps` | `protomaps_region` | Balkanlar z0-8 (~7 MB) |

Bu liste **örnek/başlangıç** paketidir; kendi listelerinizi `lists/` altına
ekleyip kaynak tanımlarını çoğaltabilirsiniz.

## Hazır paketler

Tek tek kaynak seçmek yerine hazır paketlerden birini indirebilirsiniz:

```bash
qena package list
qena package list --long
qena package list --category haritalar
qena package list --installed
qena package show hizli-baslangic
qena package fetch hizli-baslangic
qena package fetch cekirdek
```

| Paket | Kategori | İçerik | Boyut |
|---|---|---|---|
| `minimal` | başlangıç | 5 Vikipedi, 1 kitap, rehberler | ~2 MB |
| `hizli-baslangic` | başlangıç | 10 Vikipedi, 3 kitap, rehberler, Orta Doğu | ~20 MB |
| `wikipedia-tr-20` | Vikipedi | 20 Vikipedi maddesi | ~3 MB |
| `wikipedia-tr` | Vikipedi | 62 Vikipedi maddesi | ~11 MB |
| `kitaplar-5` | kitaplar | 5 Gutenberg kitabı | ~10 MB |
| `kitaplar` | kitaplar | 20 Gutenberg kitabı | ~39 MB |
| `rehberler` | rehberler | 10 kontrol listesi | ~20 KB |
| `haritalar-orta-dogu` | haritalar | Orta Doğu z0-8 | ~5 MB |
| `haritalar-balkanlar` | haritalar | Balkanlar z0-8 | ~7 MB |
| `haritalar-anadolu` | haritalar | Türkiye z15 | ~1,6 GB |
| `haritalar` | haritalar | Türkiye + Orta Doğu + Balkanlar | ~1,6 GB |
| `cekirdek` | arşiv | 62 Vikipedi, 20 kitap, rehberler, üç harita | ~1,7 GB |

`list` varsayılan olarak durum (`hazır` / `kısmi` / `yok`) ve kaynak sayısını
gösterir; `--long` açıklama, lisans, etiket ve kaynakları da ekler.
Paket biçimi ve yeni paket ekleme: [`packages/README.md`](packages/README.md).

## Mimari

```
 packages/*.toml ── qena package fetch <id>
        │
        ▼
 sources/*.toml ── qena fetch <id>
        │
        ▼
 data/content/...  +  data/catalog.db
        │  qena index
        ▼
 data/index/  (Tantivy)
        │  qena serve
        ▼
 http://127.0.0.1:8787
```

| Crate | Görev | Ağ |
|---|---|---|
| `qena-core` | Yapılandırma, katalog (SQLite), model, paket, lisans | Yok |
| `qena-fetch` | Vikipedi, Gutenberg, harita, rehber, web kaydı | Var |
| `qena-index` | Tantivy şeması, Türkçe analiz, HTML/EPUB/PDF metni | Yok |
| `qena-web` | axum: arama, okuyucu, harita, rehber, kuyruk | Yalnız yerel |
| `qena-cli` | `qena` komutu ve alt komutlar | Duruma göre |

Ayrıntılı tasarım: [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md).

## Depo düzeni

```
qena/
├─ crates/
│  ├─ qena-core/     # model, katalog, paketler, yapılandırma
│  ├─ qena-fetch/    # ağ: Vikipedi, kitap, harita, web kaydı
│  ├─ qena-index/    # Tantivy, Türkçe analiz, metin çıkarma
│  ├─ qena-web/      # axum: arama, okuyucu, harita, rehber
│  └─ qena-cli/      # `qena` komut satırı
├─ sources/          # kaynak bildirimleri (TOML)
├─ packages/         # hazır veri paketleri (TOML)
├─ lists/            # seçili Vikipedi / kitap listeleri
├─ guides/           # özgün kontrol listeleri
├─ assets/           # MapLibre, fontlar, stiller, Natural Earth
├─ packaging/        # systemd, kurulum, musl notları
├─ scripts/          # yedekleme
├─ docs/             # mimari, kaynaklar, yol haritası, yedekleme
├─ CHANGELOG.md
└─ README.md
```

## Arama

- Motor: **Tantivy** (MIT).
- Türkçe zincir: `SimpleTokenizer → LowerCaser → AsciiFoldingFilter →
  Stemmer(Turkish)`.
- Alanlar: başlık, gövde, başlıklar, tür, kaynak, dil, URL, lisans, tarih.
- Özetler `SnippetGenerator` ile vurgulanır.
- Dizin artımlıdır; `--rebuild` yalnızca dizini yeniden üretir, veriyi silmez.

Örnek:

```bash
qena search "gida" --limit 5
qena search "deprem" --kind guide
qena search "istanbul" --lang tr
```

## Haritalar

Haritalar **PMTiles** olarak saklanır ve **MapLibre GL JS** ile çizilir.

- Türkiye: z0-15, ~1,6 GB.
- Orta Doğu: z0-8, ~5 MB.
- Balkanlar: z0-8, ~7 MB.
- Stiller:
  - Sokak (açık)
  - Sokak (koyu)
  - İnsani / yüksek kontrast (HOT esinli)
  - Fiziki / idari (Natural Earth, kamu malı)
- Bölge seçici ve stil düğmeleri harita sayfasındadır.
- Karolar `qena-web` içindeki Rust `pmtiles` kütüphanesiyle okunur; tarayıcıya
  **mutlak** `/tiles/{bölge}/{z}/{x}/{y}` adresleri verilir.
- Atıf haritada kalıcıdır: “© OpenStreetMap katkıcıları (ODbL) · Protomaps ·
  Natural Earth”.

Harita indirmek `pmtiles` ikilisini gerektirir; okuma ve sunma Rust tarafındadır.

## Web kaydı

```bash
qena save https://example.com
```

- monolith kütüphanesi sayfayı tek dosyalık HTML olarak kaydeder.
- Dosya: `data/content/web/<host>/<zaman>-<slug>.html`
- Web formu (`/save`) yalnızca kuyruğa yazar; `qena fetch --queue` işler.
- Kaydedilmiş sayfa orijinali CSP `sandbox` ile sunulur; arayüz kendi
  kodunuzu çalıştırmaz.
- Her kayıt ayrı bir belge kimliği alır; eski kayıtlar korunur.

## Veri düzeni

```
data/
├─ catalog.db
├─ content/
│  ├─ wiki/tr/*.html
│  ├─ books/gutenberg/*.epub
│  ├─ guides/*.md
│  └─ web/<host>/*.html
├─ maps/*.pmtiles
├─ index/                 # Tantivy
└─ licenses/
```

`data/`, `*.pmtiles`, `*.tar.zst` ve `target/` git’e girmez. Yeni bir makinede
veriyi `qena export/import` ile taşıyın.

## Kaynak ekleme

Her kaynak `sources/<id>.toml` dosyasıdır. Hazır paketler `packages/<id>.toml`
altında, kaynak kimliklerini ve isteğe bağlı limitleri bir araya getirir.
Örnek:

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
delay_ms = 1000
```

`license` ve `attribution` zorunludur. Desteklenen türler:
`wikipedia_api`, `gutenberg`, `protomaps_region`, `guide_pack`, `url_list`,
`web_save`. `wikisource_api`, `standard_ebooks`, `natural_earth` ve `zim`
yol haritasındadır.

## Lisans ve atıf

- **Kod**: GPL-3.0-or-later (`LICENSE`).
- **Vikipedi**: CC BY-SA 4.0; madde bağlantısı ve atıf arayüzde.
- **OpenStreetMap / PMTiles**: ODbL 1.0; haritada atıf.
- **Natural Earth**: kamu malı.
- **Project Gutenberg**: kamu malı (ABD).
- **Qena rehberleri**: CC BY-SA 4.0.
- **Web kayıtları**: sitenin kendi koşulları; kullanıcı sorumluluğunda.

Ayrıntılar: [`docs/SOURCES.md`](docs/SOURCES.md) ve
[`assets/ATTRIBUTION.md`](assets/ATTRIBUTION.md).

## Linux kurulumu

Kullanıcı servisi olarak kurmak için:

```bash
./packaging/install.sh
systemctl --user enable --now qena.service
systemctl --user status qena.service
```

Varsayılanlar:

- Depo: `~/Projeler/qena`
- İkili: `~/.local/bin/qena`
- Servis: `~/.config/systemd/user/qena.service`
- Adres: `http://127.0.0.1:8787`

Farklı dizinler:

```bash
QENA_DIR=/opt/qena BIN_DIR=$HOME/.local/bin ./packaging/install.sh
```

Statik musl derleme talimatı `packaging/README.md` içinde; bu makinede glibc
release derlemesi doğrulandı.

## Taşıma: export/import

```bash
qena export qena-arsiv.tar.zst
qena export qena-arsiv.tar.zst --with-binary
qena import qena-arsiv.tar.zst --into ./qena-arsiv
cd qena-arsiv
qena --config qena.toml serve
```

Pakette `data/`, `sources/`, `packages/`, `lists/`, `guides/`, `assets/`,
`docs/`, `packaging/`, `scripts/`, `README.md`, `CHANGELOG.md` ve taşınabilir
`qena.toml` bulunur. `MANIFEST.json` her dosyanın SHA-256’sını taşır; import
sırasında doğrulanır.

## Yedekleme

Otomatik yedek betiği:

```bash
./scripts/backup.sh /mnt/yedek/qena
```

Betik `qena export` ile zaman damgalı `.tar.zst` üretir, SHA-256 yazar ve son
5 paketi tutar. Ayrıntılar: [`docs/BACKUP.md`](docs/BACKUP.md).

## Geliştirme

```bash
cargo fmt
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo build --release
```

Kurallar:

- Veri dosyalarını (`data/`, `*.pmtiles`, `*.zim`) commit etmeyin.
- Yeni kaynak türü eklerken `qena-fetch` içinde indiriciyi, `sources/README.md`
  içinde belgeyi ve testi birlikte ekleyin.
- Yeni paket eklerken `packages/*.toml` biçimini ve `packages/README.md`
  tablosunu güncelleyin.
- Ağ çağrıları yalnızca `qena-fetch` içinde olmalı; `qena-index` ve `qena-web`
  ağa çıkmamalı.
- Lisansı ve atıfı olmayan içerik kabul edilmez.

## Sorun giderme

### Harita boş görünüyor

1. Sunucunun çalıştığını doğrulayın: `qena serve`
2. `http://127.0.0.1:8787/maps` adresini **normal bir tarayıcıda** açın.
3. Sol üstteki durum yazısına bakın:
   - `Harita hazır` → sorun yok.
   - `Harita hatası: …` → hata mesajını kaydedin.
   - `Stil yüklendi · kaynak bekleniyor…` → karo istekleri engellenmiş olabilir.
4. `qena doctor` ve `data/maps/` dizinini kontrol edin.
5. `assets` dizininin doğru yerde olduğundan emin olun (`qena.toml` →
   `assets_dir`).

### `pmtiles` bulunamadı

`~/.local/bin/pmtiles` kurulu değilse `qena fetch protomaps-turkiye` hata verir.
`packaging/README.md` içindeki kurulum adımını izleyin.

### Vikipedi 429 veriyor

`delay_ms` değerini artırın (ör. 1500) ve tekrar deneyin. Qena `Retry-After`
ile geri çekilir; yine de çok hızlı istek atmayın.

### Port kullanımda

```bash
qena serve --port 8788
```

### Dizin bozuk

```bash
qena index --rebuild
```

## Yol haritası

- [x] M1 — Tantivy, SQLite, CLI, axum, Vikipedi/Gutenberg/rehberler
- [x] M2 — PMTiles + MapLibre, z15, web kaydı, kuyruk
- [x] Export/import, systemd, bölümlü kitap okuyucu
- [x] Hazır paket CLI (`qena package list/show/fetch`)
- [x] Yedekleme (`scripts/backup.sh`) ve sürüm notları
- [ ] Tam Türkçe Vikipedi ZIM içe aktarma
- [ ] Tam HOT sembolojisi ve z15+ haritalar
- [ ] Statik musl ikili
- [ ] EPUB/PDF okuyucu iyileştirmeleri
- [ ] Daha geniş küratörlü kitap/Vikipedi listeleri

## Sürüm notları

Sürüm geçmişi: [`CHANGELOG.md`](CHANGELOG.md). Son sürüm **0.1.0** (14 Kasım
2026).

## Gizlilik ve güvenlik

- Sunucu varsayılan olarak yalnızca `127.0.0.1` dinler.
- Kaydedilmiş web sayfaları güvenilmeyen içerik kabul edilir; sandbox’lı
  sunulur.
- `data/` kişisel arşivinizdir; depoya eklemeyin.
- Web kaydı yaparken sitenin koşullarına ve `robots.txt`’e saygı gösterin.
- Lisansı belirsiz içeriği arşive almayın.

## Teşekkür

Qena; açık kaynak topluluklarının işine yaslanır: Tantivy, axum, SQLite,
MapLibre, Protomaps, Natural Earth, Project Gutenberg, monolith ve diğerleri.
Ayrıntılı lisanslar `Cargo.lock`, `assets/` ve `docs/SOURCES.md` içindedir.

---

**Qena** — bilgiyi yanınızda tutun.
