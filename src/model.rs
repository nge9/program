use base64::{engine::general_purpose::STANDARD, Engine};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::{Cursor, Write},
    path::Path,
};
use url::Url;

#[derive(Clone, Serialize, Deserialize)]
pub struct Platform {
    pub id: String,
    pub name: String,
    pub url: String,
    pub icon: String,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub primary: String,
    pub secondary: String,
    #[serde(rename = "fontSize")]
    pub font_size: u8,
    pub shortcut: String,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            primary: "#10131b".into(),
            secondary: "#b4a0ff".into(),
            font_size: 14,
            shortcut: "F11".into(),
        }
    }
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Config {
    pub version: u8,
    pub apps: Vec<Platform>,
    #[serde(default)]
    pub settings: Settings,
}
impl Default for Config {
    fn default() -> Self {
        let sites = [
            ("WhatsApp", "https://web.whatsapp.com"),
            ("Discord", "https://discord.com/app"),
            ("Gmail", "https://mail.google.com"),
            ("Instagram", "https://www.instagram.com"),
            ("YouTube", "https://www.youtube.com"),
            ("Telegram", "https://web.telegram.org"),
            ("X", "https://x.com"),
            ("TikTok", "https://www.tiktok.com"),
        ];
        Self {
            version: 1,
            apps: sites
                .into_iter()
                .map(|(n, u)| Platform::new(n, u, None).unwrap())
                .collect(),
            settings: Settings::default(),
        }
    }
}
pub fn website(raw: &str) -> Result<String, String> {
    let raw = raw.trim();
    if raw.is_empty()
        || raw.len() > 2048
        || raw.chars().any(|c| c.is_whitespace() || c.is_control())
    {
        return Err("Enter a valid website address.".into());
    }
    let text = if raw.contains("://") {
        raw.to_owned()
    } else {
        format!("https://{raw}")
    };
    let url = Url::parse(&text).map_err(|_| "Enter a valid website address.")?;
    if !matches!(url.scheme(), "http" | "https")
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return Err("Use an HTTP or HTTPS address without embedded credentials.".into());
    }
    Ok(url.to_string())
}
impl Platform {
    pub fn new(name: &str, address: &str, picture: Option<String>) -> Result<Self, String> {
        let name = name.trim();
        if name.is_empty() || name.chars().count() > 80 {
            return Err("Choose a name between 1 and 80 characters.".into());
        }
        let url = website(address)?;
        let host = Url::parse(&url).unwrap().host_str().unwrap().to_owned();
        let icon = picture
            .unwrap_or_else(|| format!("https://www.google.com/s2/favicons?domain={host}&sz=128"));
        if !icon.starts_with("https://www.google.com/s2/favicons?")
            && !icon.starts_with("data:image/png;base64,")
        {
            return Err("Choose a supported app picture.".into());
        }
        Ok(Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.into(),
            url,
            icon,
        })
    }
}
impl Settings {
    pub fn validate(&self) -> Result<(), String> {
        for c in [&self.primary, &self.secondary] {
            if c.len() != 7 || !c.starts_with('#') || !c[1..].chars().all(|x| x.is_ascii_hexdigit())
            {
                return Err("Choose valid colors.".into());
            }
        }
        if !(12..=20).contains(&self.font_size) {
            return Err("Text size must be between 12 and 20.".into());
        }
        if self.shortcut.is_empty()
            || self.shortcut.len() > 32
            || ["Ctrl+H", "Ctrl+W", "Ctrl+Tab", "Ctrl+Shift+Tab"].contains(&self.shortcut.as_str())
        {
            return Err("Choose another fullscreen shortcut.".into());
        }
        Ok(())
    }
}
impl Config {
    pub fn load(path: &Path) -> Result<Self, String> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let text = fs::read_to_string(path).map_err(|e| e.to_string())?;
        let cfg: Self = serde_json::from_str(&text)
            .map_err(|_| "Your workspace file could not be read. It has not been overwritten.")?;
        cfg.settings.validate()?;
        let mut ids = std::collections::HashSet::new();
        for a in &cfg.apps {
            website(&a.url)?;
            if a.name.trim().is_empty() || !ids.insert(a.id.clone()) {
                return Err("Invalid platform records. Your file has not been overwritten.".into());
            }
        }
        Ok(cfg)
    }
    pub fn save(&self, path: &Path) -> Result<(), String> {
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
        let bytes = serde_json::to_vec_pretty(self).map_err(|e| e.to_string())?;
        atomicwrites::AtomicFile::new(path, atomicwrites::AllowOverwrite)
            .write(|f| f.write_all(&bytes))
            .map_err(|e| e.to_string())
    }
}
pub fn picture(path: &Path) -> Result<String, String> {
    let bytes = fs::read(path).map_err(|e| e.to_string())?;
    if bytes.len() > 10 * 1024 * 1024 {
        return Err("Choose an image smaller than 10 MB.".into());
    }
    let img = bounded_image(&bytes)?.thumbnail(128, 128).to_rgba8();
    let mut canvas = image::RgbaImage::new(128, 128);
    image::imageops::overlay(
        &mut canvas,
        &img,
        ((128 - img.width()) / 2).into(),
        ((128 - img.height()) / 2).into(),
    );
    let mut out = Cursor::new(Vec::new());
    image::DynamicImage::ImageRgba8(canvas)
        .write_to(&mut out, image::ImageFormat::Png)
        .map_err(|e| e.to_string())?;
    Ok(format!(
        "data:image/png;base64,{}",
        STANDARD.encode(out.into_inner())
    ))
}
fn bounded_image(bytes: &[u8]) -> Result<image::DynamicImage, String> {
    let mut reader = image::ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|e| e.to_string())?;
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(8192);
    limits.max_image_height = Some(8192);
    limits.max_alloc = Some(80 * 1024 * 1024);
    reader.limits(limits);
    let image = reader
        .decode()
        .map_err(|_| "Choose a valid image up to 8192 pixels wide and high.")?;
    if image.width() as u64 * image.height() as u64 > 16_000_000 {
        return Err("Choose an image up to 16 megapixels.".into());
    }
    Ok(image)
}
pub fn qr(path: &Path) -> Result<String, String> {
    let bytes = fs::read(path).map_err(|e| e.to_string())?;
    if bytes.len() > 10 * 1024 * 1024 {
        return Err("Choose a QR image smaller than 10 MB.".into());
    }
    let mut img = rqrr::PreparedImage::prepare(bounded_image(&bytes)?.to_luma8());
    let mut urls = Vec::new();
    for grid in img.detect_grids() {
        if let Ok((_, text)) = grid.decode() {
            let text = text.trim();
            if text.starts_with("https://")
                || text.starts_with("http://")
                || text.starts_with("www.")
            {
                if let Ok(url) = website(text) {
                    if !urls.contains(&url) {
                        urls.push(url);
                    }
                }
            }
        }
    }
    match urls.len() {
        1 => Ok(urls.remove(0)),
        0 => Err("No website QR code found. Sign-in and Wi-Fi codes are not website links.".into()),
        _ => Err("Crop the image to one website QR code.".into()),
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn address_validation() {
        assert_eq!(website("example.org").unwrap(), "https://example.org/");
        for bad in [
            "file:///secret",
            "javascript:hello",
            "https://user:pass@example.org",
            "a b",
        ] {
            assert!(website(bad).is_err());
        }
    }
    #[test]
    fn defaults_and_settings() {
        let c = Config::default();
        assert_eq!(c.apps.len(), 8);
        assert!(c.settings.validate().is_ok());
        let mut s = c.settings;
        s.font_size = 25;
        assert!(s.validate().is_err());
    }
    #[test]
    fn empty_workspace_roundtrip() {
        let mut c = Config::default();
        c.apps.clear();
        let text = serde_json::to_string(&c).unwrap();
        let back: Config = serde_json::from_str(&text).unwrap();
        assert!(back.apps.is_empty());
    }
}
