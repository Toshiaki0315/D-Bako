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

/// アイコンファイルが変わったかを見分ける目印 (パス・inode・サイズ・更新時刻)。
/// 画像を取り出すより十分に軽いので、画面側はこれが変わったときだけ取り出し直す。
pub fn icon_stamp(app_path: &Path) -> Result<String, String> {
    use std::os::unix::fs::MetadataExt;
    let icon_path = find_icon_path(app_path)?;
    let meta = std::fs::metadata(&icon_path).map_err(|e| e.to_string())?;
    Ok(format!("{}|{}|{}|{}.{}", icon_path.display(), meta.ino(), meta.len(), meta.mtime(), meta.mtime_nsec()))
}

/// PNG バイト列を `<img src>` にそのまま使える data URL にする。
pub fn to_data_url(png: &[u8]) -> String {
    use base64::{engine::general_purpose::STANDARD, Engine};
    format!("data:image/png;base64,{}", STANDARD.encode(png))
}

/// 余白を持つアイコンの、絵の部分が占める割合 (他のアイコンと見た目をそろえる目安)。
const ICON_CONTENT_RATIO: f32 = 0.8;
/// 角丸の半径 (縮小前の画像の一辺に対する割合)。
const ICON_CORNER_RATIO: f32 = 0.18;

/// 透明な余白のない全面アイコンを、余白ありのアイコンと同じ見た目にそろえる。
/// 四隅が不透明なら、中央へ縮小して角丸に切り抜く。余白があるものはそのまま返す。
pub fn normalize_margin(rgba: &[u8], size: u32) -> Vec<u8> {
    let n = size as usize;
    if n == 0 || rgba.len() != n * n * 4 {
        return rgba.to_vec();
    }
    let last = n - 1;
    if [(0, 0), (last, 0), (0, last), (last, last)].iter().any(|&(x, y)| rgba[(y * n + x) * 4 + 3] < 200) {
        return rgba.to_vec();
    }
    let side = n as f32 * ICON_CONTENT_RATIO;
    let offset = (n as f32 - side) / 2.0;
    let radius = n as f32 * ICON_CORNER_RATIO * ICON_CONTENT_RATIO;
    let mut out = vec![0u8; rgba.len()];
    for y in 0..n {
        for x in 0..n {
            let (px, py) = (x as f32 + 0.5 - offset, y as f32 + 0.5 - offset);
            if px < 0.0 || py < 0.0 || px >= side || py >= side {
                continue;
            }
            let (cx, cy) = (px.clamp(radius, side - radius), py.clamp(radius, side - radius));
            if (px - cx).powi(2) + (py - cy).powi(2) > radius * radius {
                continue;
            }
            let (sx, sy) = (px / side * n as f32 - 0.5, py / side * n as f32 - 0.5);
            let (x0, y0) = (sx.floor().clamp(0.0, last as f32) as usize, sy.floor().clamp(0.0, last as f32) as usize);
            let (x1, y1) = ((x0 + 1).min(last), (y0 + 1).min(last));
            let (fx, fy) = ((sx - x0 as f32).clamp(0.0, 1.0), (sy - y0 as f32).clamp(0.0, 1.0));
            for c in 0..4 {
                let at = |xx: usize, yy: usize| rgba[(yy * n + xx) * 4 + c] as f32;
                let top = at(x0, y0) * (1.0 - fx) + at(x1, y0) * fx;
                let bottom = at(x0, y1) * (1.0 - fx) + at(x1, y1) * fx;
                out[(y * n + x) * 4 + c] = (top * (1.0 - fy) + bottom * fy).round() as u8;
            }
        }
    }
    out
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
    let image = image.convert_to(icns::PixelFormat::RGBA);
    let image = if image.width() == image.height() {
        let data = normalize_margin(image.data(), image.width());
        icns::Image::from_data(icns::PixelFormat::RGBA, image.width(), image.height(), data).map_err(|e| e.to_string())?
    } else {
        image
    };
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


    fn alpha_at(rgba: &[u8], size: u32, x: u32, y: u32) -> u8 {
        rgba[((y * size + x) * 4 + 3) as usize]
    }

    #[test]
    fn leaves_an_icon_with_a_transparent_margin_alone() {
        let size = 40u32;
        let mut rgba = vec![0u8; (size * size * 4) as usize];
        for y in 4..36 {
            for x in 4..36 {
                let i = ((y * size + x) * 4) as usize;
                rgba[i..i + 4].copy_from_slice(&[10, 20, 30, 255]);
            }
        }
        assert_eq!(normalize_margin(&rgba, size), rgba);
    }

    #[test]
    fn shrinks_and_rounds_a_full_bleed_icon() {
        let size = 100u32;
        let rgba = vec![255u8; (size * size * 4) as usize];
        let out = normalize_margin(&rgba, size);
        assert_eq!(out.len(), rgba.len());
        assert_eq!(alpha_at(&out, size, 0, 0), 0);
        assert_eq!(alpha_at(&out, size, 99, 99), 0);
        assert_eq!(alpha_at(&out, size, 50, 3), 0);
        assert_eq!(alpha_at(&out, size, 50, 50), 255);
        assert_eq!(alpha_at(&out, size, 11, 11), 0);
    }

    #[test]
    fn normalizing_ignores_a_buffer_of_the_wrong_length() {
        assert_eq!(normalize_margin(&[1, 2, 3], 10), vec![1, 2, 3]);
        assert_eq!(normalize_margin(&[], 0), Vec::<u8>::new());
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
    fn stamp_stays_the_same_while_the_icon_file_is_unchanged() {
        let dir = tempfile::tempdir().unwrap();
        let app = make_bundle(dir.path(), Some(plist_with(Some("AppIcon"))), Some(("AppIcon.icns", icns_bytes(&[16]))));
        assert_eq!(icon_stamp(&app).unwrap(), icon_stamp(&app).unwrap());
    }

    #[test]
    fn stamp_changes_when_the_app_is_updated_with_a_new_icon_file() {
        let dir = tempfile::tempdir().unwrap();
        let app = make_bundle(dir.path(), Some(plist_with(Some("AppIcon"))), Some(("AppIcon.icns", icns_bytes(&[16]))));
        let before = icon_stamp(&app).unwrap();
        // アップデートはファイルを置き換える (別の inode・内容・時刻になる)
        let icns = app.join("Contents/Resources/AppIcon.icns");
        fs::remove_file(&icns).unwrap();
        fs::write(&icns, icns_bytes(&[16, 32])).unwrap();
        assert_ne!(icon_stamp(&app).unwrap(), before);
    }

    #[test]
    fn stamp_changes_when_the_plist_points_to_another_icon_file() {
        let dir = tempfile::tempdir().unwrap();
        let app = make_bundle(dir.path(), Some(plist_with(Some("AppIcon"))), Some(("AppIcon.icns", icns_bytes(&[16]))));
        let before = icon_stamp(&app).unwrap();
        fs::write(app.join("Contents/Resources/NewIcon.icns"), icns_bytes(&[16])).unwrap();
        fs::write(app.join("Contents/Info.plist"), plist_with(Some("NewIcon"))).unwrap();
        assert_ne!(icon_stamp(&app).unwrap(), before);
    }

    #[test]
    fn stamp_errors_when_the_icon_file_is_missing() {
        let dir = tempfile::tempdir().unwrap();
        let app = make_bundle(dir.path(), Some(plist_with(Some("Nope"))), None);
        assert!(icon_stamp(&app).unwrap_err().contains("Icon file not found"));
    }

    #[test]
    fn errors_on_icns_without_images() {
        let dir = tempfile::tempdir().unwrap();
        let app = make_bundle(dir.path(), Some(plist_with(Some("AppIcon"))), Some(("AppIcon.icns", icns_bytes(&[]))));
        assert_eq!(extract_icon_png(&app).unwrap_err(), "No icon images in icns");
    }
}
