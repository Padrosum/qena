# Kaynak bildirimleri

Her kaynak `sources/<id>.toml` dosyasıdır. `qena source list` bunları okur;
`qena fetch <id>` yalnızca istenen kaynağı indirir.

## Kurallar

- `enabled = false` varsayılandır; kullanıcı açmadan indirme yapılmaz.
- `license` ve `attribution` zorunludur. Boşsa kaynak reddedilir.
- `kind`, indiricinin türünü belirler.
- Kaynağa özel ayarlar `[source.params]` altındadır.
- Aynı `id` iki kez tanımlanamaz.

## Örnek

```toml
[source]
id = "gutenberg-secili"
category = "books"
title = "Gutenberg — seçili kamu malı kitaplar"
kind = "gutenberg"
license = "public-domain"
attribution = "Project Gutenberg"
enabled = false

[source.params]
ids_file = "lists/gutenberg-selected.txt"
format = "epub"
```

## Desteklenen türler

| `kind` | Açıklama |
|---|---|
| `wikipedia_api` | Başlık listesinden HTML + metin |
| `wikisource_api` | Vikikaynak metinleri |
| `gutenberg` | Gutendex üzerinden EPUB/TXT |
| `standard_ebooks` | OPDS kataloğu |
| `protomaps_region` | Bölge PMTiles çıkarımı |
| `natural_earth` | Kamu malı vektör katmanları |
| `guide_pack` | Yerel Markdown rehberleri |
| `url_list` | Satır satır URL listesi |
| `web_save` | Tek URL veya `urls` dizisi |

## Web kaydı örneği

```toml
[source]
id = "web-arsiv"
category = "web"
title = "Web kayıtları"
kind = "url_list"
license = "site-terms"
attribution = "Kişisel arşiv"
enabled = false

[source.params]
urls_file = "lists/web-urls.txt"
```

Kuyruktan işlemek için: `qena fetch --queue`.
