use sha2::{Digest, Sha256};

/// Dosya adı ve URL parçası için Türkçe karakterleri sadeleştirir.
pub fn slugify(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        let mapped = match ch {
            'İ' | 'I' | 'ı' | 'i' => 'i',
            'Ş' | 'ş' => 's',
            'Ğ' | 'ğ' => 'g',
            'Ü' | 'ü' => 'u',
            'Ö' | 'ö' => 'o',
            'Ç' | 'ç' => 'c',
            'Â' | 'â' => 'a',
            'Î' | 'î' => 'i',
            'Û' | 'û' => 'u',
            other => other,
        };
        if mapped.is_ascii_alphanumeric() {
            out.push(mapped.to_ascii_lowercase());
        } else if mapped.is_alphanumeric() {
            for lower in mapped.to_lowercase() {
                out.push(lower);
            }
        } else if !out.ends_with('-') {
            out.push('-');
        }
    }
    let trimmed = out.trim_matches('-');
    let mut result: String = trimmed.chars().take(80).collect();
    if result.is_empty() {
        result.push_str("doc");
    }
    result
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    let digest = Sha256::digest(bytes);
    let mut out = String::with_capacity(64);
    for byte in digest {
        write!(&mut out, "{byte:02x}").expect("bellek yazımı");
    }
    out
}

pub fn now_rfc3339() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugify_turkish() {
        assert_eq!(slugify("İstanbul'da Deprem!"), "istanbul-da-deprem");
        assert_eq!(slugify("Şanlıurfa — su & gıda"), "sanliurfa-su-gida");
    }

    #[test]
    fn slugify_empty() {
        assert_eq!(slugify("!!!"), "doc");
    }

    #[test]
    fn sha256_known_value() {
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }
}
