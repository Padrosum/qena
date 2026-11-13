# Project Qena

Kötü günler için yerel bilgi sandığı: internete, buluta ya da dışarıdaki bir
servise ihtiyaç duymadan çalışan; aranabilir, okunabilir ve taşınabilir bir
arşiv.

> Eski Qena 8 Kasım 2026'da iptal edilmişti. Bu depo, aynı adı taşıyan **yeni
> ve bağımsız** projedir; eski içerikle bir bağlantısı yoktur.

## Ne olacak

- **Vikipedi**: seçili maddeler (ve istenirse tam Türkçe Vikipedi arşivi).
- **Kitaplar**: kamu malı / özgür lisanslı kitaplar (Gutenberg, Standard
  Ebooks, Vikikaynak).
- **Haritalar**: Anadolu, Orta Doğu ve Avrupa için çevrimdışı vektör haritalar;
  yalnızca sokak değil, fiziki/idari katmanlar da.
- **Rehberler**: deprem, su/gıda, ilk yardım, enerji/iletişim, tahliye gibi
  kontrol listeleri.
- **Web kayıtları**: tek bir URL'yi ya da listeyi tek dosyalık HTML/WARC olarak
  saklama.
- **Yerel arama**: Türkçe ekleri ve aksan farklarını tolere eden, tek makinede
  çalışan tam metin arama.

## İlkeler

1. **Ağ yalnızca indirme komutunda.** `qena fetch` çalışmadıkça hiçbir bileşen
   dışarıdan veri çekmez. Dizinleyici ve web sunucusu ağa çıkmaz.
2. **Kaynak seçimi kullanıcının.** Her kategori ve kaynak ayrı bir bildirim
   dosyasıdır; istenen kaynak, istenen kategoriden açıkça indirilir.
3. **Tek makine, tek ikili.** Rust ile yazılmış motor + `127.0.0.1` üzerinde
   çalışan yerel web arayüzü. Node, Python ya da ayrı bir veritabanı servisi
   gerekmez.
4. **Lisans ve atıf görünür.** Her içerik parçası kaynak, lisans ve atıf
   bilgisiyle saklanır; arayüzde gösterilir.
5. **Taşınabilir arşiv.** Veri dizini kopyalanabilir; `qena export` ile tek
   pakete dönüştürülebilir.

## Durum

**M1 ve M2'nin çekirdeği çalışıyor** (12 Kasım 2026):

- `qena init` → katalog + veri dizini
- `qena fetch wikipedia-tr-curated` → Vikipedi HTML
- `qena fetch gutenberg-secili` → Gutenberg EPUB/TXT
- `qena fetch rehberler` → 10 kontrol listesi
- `qena fetch protomaps-turkiye` → PMTiles sokak haritası (z0-14)
- `qena index` → Tantivy + Türkçe analiz
- `qena search "deprem çantası"` → sonuçlar
- `qena serve` → arama, okuyucu, **bölümlü kitap okuyucu**, rehberler, harita (bölge + yüksek kontrast stil)
- `qena save <url>` → monolith ile tek dosya web kaydı
- `qena fetch --queue` → web formundan gelen kayıtları işleme
- `qena export` / `qena import` → `.tar.zst` + SHA-256 doğrulamalı taşınabilir paket
- `packaging/install.sh` → `~/.local/bin/qena` + systemd kullanıcı servisi

Ayrıntılı mimari `docs/ARCHITECTURE.md`, kaynak listesi `docs/SOURCES.md`,
yol haritası `docs/ROADMAP.md` içinde.

## Depo düzeni (hedef)

```
qena/
├─ crates/
│  ├─ qena-core/     # model, katalog, lisans, yapılandırma
│  ├─ qena-fetch/    # ağ: vikipedi, kitap, harita, web kaydı
│  ├─ qena-index/    # Tantivy dizini, Türkçe analiz, metin çıkarma
│  ├─ qena-web/      # axum: arama, okuyucu, harita, rehberler
│  └─ qena-cli/      # `qena` komut satırı
├─ sources/          # kaynak bildirimleri (TOML)
├─ lists/            # seçili madde / kitap listeleri
├─ guides/           # özgün kontrol listeleri
├─ assets/           # MapLibre, fontlar, şablonlar
├─ packaging/        # systemd, kurulum, musl derleme
└─ docs/
```

## Lisans

Kod: **GPL-3.0-or-later**. İçerik: kaynağına göre değişir; ayrıntılar
`docs/SOURCES.md` ve arşivdeki atıf kayıtlarında.
