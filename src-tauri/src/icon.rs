//! インフラ層: `.app` バンドルからアイコン画像 (PNG) を取り出す。
//! `Info.plist` の `CFBundleIconFile` が指す `.icns` から最大サイズの画像を使う。

use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};

/// `Info.plist` を読み、アイコン (.icns) のパスを返す。
pub fn find_icon_path(app_path: &Path) -> Result<PathBuf, String> {
    let plist_path = app_path.join("Contents/Info.plist");
    let info = plist::Value::from_file(&plist_path)
        .map_err(|e| format!("Cannot read Info.plist: {e}"))?;
    let name = info
        .as_dictionary()
        .and_then(|d| d.get("CFBundleIconFile"))
        .and_then(|v| v.as_string())
        .ok_or_else(|| "CFBundleIconFile not found".to_string())?;
    let file_name = if name.ends_with(".icns") { name.to_string() } else { format!("{name}.icns") };
    let icon_path = app_path.join("Contents/Resources").join(file_name);
    if icon_path.is_file() { Ok(icon_path) } else { Err(format!("Icon file not found: {}", icon_path.display())) }
}

/// PNG バイト列を `<img src>` にそのまま使える data URL にする。
pub fn to_data_url(png: &[u8]) -> String {
    use base64::{engine::general_purpose::STANDARD, Engine};
    format!("data:image/png;base64,{}", STANDARD.encode(png))
}

/// アプリのアイコンを、最大サイズの PNG バイト列として返す。
pub fn extract_icon_png(app_path: &Path) -> Result<Vec<u8>, String> {
    let icon_path = find_icon_path(app_path)?;
    let file = File::open(&icon_path).map_err(|e| e.to_string())?;
    let family = icns::IconFamily::read(BufReader::new(file)).map_err(|e| e.to_string())?;
    let largest = family
        .available_icons()
        .into_iter()
        .max_by_key(|t| t.pixel_width())
        .ok_or_else(|| "No icon images in icns".to_string())?;
    let image = family.get_icon_with_type(largest).map_err(|e| e.to_string())?;
    let mut png = Vec::new();
    image.write_png(&mut png).map_err(|e| e.to_string())?;
    Ok(png)
}


#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::Path;

    const PNG_MAGIC: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];

    fn plist_with(icon_file: Option<&str>) -> String {
        let key = icon_file
            .map(|f| format!("<key>CFBundleIconFile</key><string>{f}</string>"))
            .unwrap_or_default();
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>{key}</dict></plist>"#
        )
    }

    /// 指定サイズの単色 RGBA 画像を持つ .icns のバイト列を作る。
    fn icns_bytes(sizes: &[u32]) -> Vec<u8> {
        let mut family = icns::IconFamily::new();
        for &s in sizes {
            let data = vec![255u8; (s * s * 4) as usize];
            let image = icns::Image::from_data(icns::PixelFormat::RGBA, s, s, data).unwrap();
            family.add_icon(&image).unwrap();
        }
        let mut out = Vec::new();
        family.write(&mut out).unwrap();
        out
    }

    fn make_bundle(root: &Path, plist: Option<String>, icns: Option<(&str, Vec<u8>)>) -> std::path::PathBuf {
        let app = root.join("Foo.app");
        fs::create_dir_all(app.join("Contents/Resources")).unwrap();
        if let Some(p) = plist { fs::write(app.join("Contents/Info.plist"), p).unwrap(); }
        if let Some((name, bytes)) = icns { fs::write(app.join("Contents/Resources").join(name), bytes).unwrap(); }
        app
    }

    #[test]
    fn encodes_png_bytes_as_data_url() {
        assert_eq!(to_data_url(&[1, 2, 3]), "data:image/png;base64,AQID");
        assert_eq!(to_data_url(&[]), "data:image/png;base64,");
    }

    #[test]
    fn finds_icon_when_plist_omits_extension() {
        let dir = tempfile::tempdir().unwrap();
        let app = make_bundle(dir.path(), Some(plist_with(Some("AppIcon"))), Some(("AppIcon.icns", icns_bytes(&[32]))));
        assert_eq!(find_icon_path(&app).unwrap(), app.join("Contents/Resources/AppIcon.icns"));
    }

    #[test]
    fn finds_icon_when_plist_includes_extension() {
        let dir = tempfile::tempdir().unwrap();
        let app = make_bundle(dir.path(), Some(plist_with(Some("AppIcon.icns"))), Some(("AppIcon.icns", icns_bytes(&[32]))));
        assert_eq!(find_icon_path(&app).unwrap(), app.join("Contents/Resources/AppIcon.icns"));
    }

    #[test]
    fn errors_when_info_plist_is_missing() {
        let dir = tempfile::tempdir().unwrap();
        let app = make_bundle(dir.path(), None, None);
        assert!(find_icon_path(&app).unwrap_err().contains("Info.plist"));
    }

    #[test]
    fn errors_when_icon_key_is_missing() {
        let dir = tempfile::tempdir().unwrap();
        let app = make_bundle(dir.path(), Some(plist_with(None)), None);
        assert_eq!(find_icon_path(&app).unwrap_err(), "CFBundleIconFile not found");
    }

    #[test]
    fn errors_when_icon_file_is_missing() {
        let dir = tempfile::tempdir().unwrap();
        let app = make_bundle(dir.path(), Some(plist_with(Some("Nope"))), None);
        assert!(find_icon_path(&app).unwrap_err().contains("Icon file not found"));
    }

    #[test]
    fn extracts_png_of_the_largest_size() {
        let dir = tempfile::tempdir().unwrap();
        let app = make_bundle(dir.path(), Some(plist_with(Some("AppIcon"))), Some(("AppIcon.icns", icns_bytes(&[16, 64]))));
        let png = extract_icon_png(&app).unwrap();
        assert_eq!(&png[..8], &PNG_MAGIC);
        // PNG の IHDR: 幅は先頭から 16〜20 バイト目
        assert_eq!(u32::from_be_bytes(png[16..20].try_into().unwrap()), 64);
    }

    #[test]
    fn errors_on_corrupt_icns() {
        let dir = tempfile::tempdir().unwrap();
        let app = make_bundle(dir.path(), Some(plist_with(Some("AppIcon"))), Some(("AppIcon.icns", b"garbage".to_vec())));
        assert!(extract_icon_png(&app).is_err());
    }

    #[test]
    fn errors_on_icns_without_images() {
        let dir = tempfile::tempdir().unwrap();
        let app = make_bundle(dir.path(), Some(plist_with(Some("AppIcon"))), Some(("AppIcon.icns", icns_bytes(&[]))));
        assert_eq!(extract_icon_png(&app).unwrap_err(), "No icon images in icns");
    }
}
