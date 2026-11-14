# Yedekleme

Qena arşivini tek dosyada yedeklemek için `scripts/backup.sh` kullanılır.
Betik, `qena export` ile `.tar.zst` paketi üretir, yanına SHA-256 yazar ve
varsayılan olarak son 5 paketi tutar.

```bash
cd ~/Projeler/qena
./scripts/backup.sh /mnt/yedek/qena
```

Farklı depo/ikili yolları:

```bash
QENA_DIR=/opt/qena QENA_BACKUP_KEEP=10 ./scripts/backup.sh /mnt/yedek/qena
```

## Zamanlama

Kullanıcı cron örneği:

```cron
0 3 * * * cd /home/KULLANICI/Projeler/qena && ./scripts/backup.sh /mnt/yedek/qena >> /home/KULLANICI/.local/share/qena-backup.log 2>&1
```

systemd timer kullanacaksanız `qena.service` gibi bir `qena-backup.service` +
`qena-backup.timer` çifti oluşturun; `ExecStart` alanında betiği tam yolla
çağırın.

## Geri yükleme

```bash
qena import /mnt/yedek/qena/qena-YYYYMMDD-HHMMSS.tar.zst --into ./qena-geri
cd qena-geri
qena --config qena.toml serve
```

`qena import` dosyaları `MANIFEST.json` içindeki SHA-256 ile doğrular. Bozuk
paket reddedilir.

## Notlar

- Tutarlı yedek için önce `qena serve` sürecini durdurun.
- Harita paketleri büyüktür (Türkiye z0-15 birkaç GB); yedek alanını buna göre
  planlayın.
- `data/` ve `*.tar.zst` git’e girmez; yedekler ayrı bir diskte tutulmalıdır.
