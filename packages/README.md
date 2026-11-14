# Hazır veri paketleri

`packages/*.toml` dosyaları, birden çok kaynağı tek komutta indirmek için
paket tanımlarıdır. Paketler `qena package` komutuyla listelenir.

```bash
qena package list
qena package list --long
qena package list --category haritalar
qena package list --installed
qena package show cekirdek
qena package fetch hizli-baslangic
qena package fetch haritalar --dry-run
qena package list --json
```

`list` varsayılan olarak kimlik, kategori, boyut, kaynak sayısı ve durum
gösterir. Durum `hazır`, `kısmi` veya `yok` olur. `--long` açıklama, lisans,
etiketler ve kaynakları da yazar; `--json` betikler içindir.

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

| Kimlik | Kategori | İçerik | Boyut |
|---|---|---|---|
| `minimal` | baslangic | 5 Vikipedi, 1 kitap, rehberler | ~2 MB |
| `hizli-baslangic` | baslangic | 10 Vikipedi, 3 kitap, rehberler, Orta Doğu haritası | ~20 MB |
| `wikipedia-tr-20` | wikipedia | 20 Vikipedi maddesi | ~3 MB |
| `wikipedia-tr` | wikipedia | 62 Vikipedi maddesi | ~11 MB |
| `kitaplar-5` | kitaplar | 5 Gutenberg kitabı | ~10 MB |
| `kitaplar` | kitaplar | 20 Gutenberg kitabı | ~39 MB |
| `rehberler` | rehberler | 10 kontrol listesi | ~20 KB |
| `haritalar-orta-dogu` | haritalar | Orta Doğu z0-8 | ~5 MB |
| `haritalar-balkanlar` | haritalar | Balkanlar z0-8 | ~7 MB |
| `haritalar-anadolu` | haritalar | Türkiye z15 | ~1,6 GB |
| `haritalar` | haritalar | Türkiye + Orta Doğu + Balkanlar | ~1,6 GB |
| `cekirdek` | arsiv | 62 Vikipedi, 20 kitap, rehberler, üç harita | ~1,7 GB |
