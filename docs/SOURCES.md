# Kaynaklar, lisanslar ve boyutlar

Bu liste ilk paket için aday kaynakları gösterir. Boyutlar 12 Kasım 2026
itibarıyla indirme sunucularının bildirdiği değerlerdir; sonradan değişebilir.
Hiçbir kaynak varsayılan olarak açık değildir.

## Vikipedi

| Kaynak | Lisans | Yaklaşık boyut | Not |
|---|---|---|---|
| Seçili Türkçe/İngilizce maddeler (API) | CC BY-SA 4.0 | 0,5–3 GB | `lists/` altındaki başlık listeleri; görseller isteğe bağlı |
| Türkçe Vikipedi — tam, nopic (Kiwix ZIM) | CC BY-SA 4.0 | 3,5 GB | `zim` kaynağı; ileride |
| Türkçe Vikipedi — tam, mini | CC BY-SA 4.0 | 1,1 GB | görselsiz, sıkıştırılmış |
| Türkçe Vikipedi — tam, maxi | CC BY-SA 4.0 | 8,3 GB | görsellerle |

Atıf: "Vikipedi, özgür ansiklopedi — CC BY-SA 4.0" + madde revizyon bağlantısı.

## Kitaplar

| Kaynak | Lisans | Yaklaşık boyut | Not |
|---|---|---|---|
| Project Gutenberg (doğrudan EPUB/TXT) | Kamu malı (ABD) | 1–5 MB/kitap | `gutenberg.org/ebooks/<id>.epub` |
| Standard Ebooks (OPDS) | Kamu malı (ABD) | 2–8 MB/kitap | düzenli EPUB |
| Vikikaynak (Wikisource API) | CC BY-SA 4.0 | değişken | Türkçe kamu malı metinler |
| Yerel PDF/EPUB | kullanıcıya bağlı | değişken | `qena library add` ile |

## Haritalar

| Bölge | Kaynak | Lisans | Yaklaşık boyut |
|---|---|---|---|
| Türkiye | Geofabrik PBF / Protomaps PMTiles | ODbL 1.0 | 0,65 GB PBF; z0-15 PMTiles ~1,6 GB |
| Suriye | Geofabrik | ODbL 1.0 | 83 MB |
| Irak | Geofabrik | ODbL 1.0 | 92 MB |
| İran | Geofabrik | ODbL 1.0 | 230 MB |
| Lübnan | Geofabrik | ODbL 1.0 | 53 MB |
| Ürdün | Geofabrik | ODbL 1.0 | 31 MB |
| İsrail-Filistin | Geofabrik | ODbL 1.0 | 121 MB |
| Yunanistan | Geofabrik | ODbL 1.0 | ~0,4 GB |
| Bulgaristan | Geofabrik | ODbL 1.0 | ~0,2 GB |
| Avrupa (tümü) | Geofabrik | ODbL 1.0 | 35,4 GB |
| Dünya — fiziki/idari | Natural Earth | Kamu malı | onlarca MB |

Not: Protomaps basemap, OSM verisinden üretilmiş bir "Produced Work"tür;
ODbL atfı zorunludur. Sokak dışı katmanlar Natural Earth'ten gelir.

## Rehberler

| Kaynak | Lisans | Not |
|---|---|---|
| Qena rehberleri | CC BY-SA 4.0 | Deprem, su/gıda, ilk yardım, enerji/iletişim, tahliye, barınma, belge/para, sağlık, hijyen, hayvanlar |
| Ready.gov / FEMA | Kamu malı (ABD) | İngilizce; çeviri/uyarlama |
| AFAD / Kızılay | Site koşullarına bağlı | Yalnızca atıfla, izin gerekiyorsa indirilmez |

## Web kayıtları

| Kaynak | Lisans | Not |
|---|---|---|
| Kullanıcının verdiği URL'ler | Sitenin kendi lisansı | Kişisel arşiv; `robots.txt`'e saygı, toplu kazıma yok |
| URL listeleri | Kullanıcıya bağlı | Her URL için kaynak ve tarih kaydı |

## Mevcut örnek (12 Kasım 2026)

- 62 Vikipedi maddesi (deprem, tsunami, sel, su, gıda, coğrafya, kurumlar…)
- 20 Gutenberg kitabı (Alice, Pride and Prejudice, Moby Dick, Dracula…)
- 10 Qena rehberi
- 3 harita paketi: Türkiye z0-15 (~1,6 GB), Orta Doğu z0-8 (~5 MB),
  Balkanlar z0-8 (~7 MB)
- 2 web kaydı (example.com, IANA example domains)

Toplam veri dizini ~1,7 GB. Tam liste `qena status` ile görülür.

## Lisans uyumluluğu

- CC BY-SA 4.0 içerik, aynı lisansla paylaşıldığında uyumludur; Qena'da atıf
  ve lisans görünür tutulur.
- ODbL veri tabanı için atıf ve "share alike" yükümlülüğü vardır; türetilmiş
  harita dosyaları aynı lisansla sunulur.
- Kamu malı kitaplar sınırsızdır; yine de kaynak URL'si saklanır.
- Lisansı belirsiz içerik arşive alınmaz; yalnızca bağlantı olarak gösterilir.
