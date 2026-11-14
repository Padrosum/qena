# Hazır veri paketleri

`packages/*.toml` dosyaları, birden çok kaynağı tek komutta indirmek için
paket tanımlarıdır. Paketler `qena package` komutuyla listelenir.

```bash
qena package list
qena package show cekirdek
qena package fetch hizli-baslangic
qena package fetch haritalar --dry-run
qena package list --json
```

## Biçim

```toml
[package]
id = "ornek-paket"
title = "Örnek paket"
description = "Kısa açıklama."
license = "kaynak bazlı"
estimated_size = "~100 MB"
sources = ["wikipedia-tr-curated", "rehberler"]

[package.limits]
wikipedia-tr-curated = 20
```

- `sources`: `sources/*.toml` içindeki kaynak kimlikleri.
- `limits`: kaynak başına en fazla öğe; yoksa kaynağın kendi listesi kullanılır.
- `estimated_size`: kullanıcıya gösterilen yaklaşık boyut; bağlayıcı değildir.
- Aynı `id` iki kez kullanılamaz.
- Paket, kaynak tanımlarının yerine geçmez; yalnızca bir indirme planıdır.

## Mevcut paketler

| Kimlik | İçerik | Boyut |
|---|---|---|
| `hizli-baslangic` | 10 Vikipedi, 3 kitap, rehberler, Orta Doğu haritası | ~20 MB |
| `cekirdek` | 62 Vikipedi, 20 kitap, rehberler, üç harita | ~1,7 GB |
| `wikipedia-tr` | 62 Vikipedi maddesi | ~11 MB |
| `kitaplar` | 20 Gutenberg kitabı | ~39 MB |
| `rehberler` | 10 kontrol listesi | ~20 KB |
| `haritalar` | Türkiye + Orta Doğu + Balkanlar | ~1,6 GB |
| `haritalar-anadolu` | Türkiye z15 | ~1,6 GB |
