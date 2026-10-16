use font_converter_engine::{Existing, Mode};
use serde::{Deserialize, Serialize};
use std::{fs, io::Write, path::PathBuf};
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub version: u32,
    pub language: String,
    pub theme: String,
    pub jobs: usize,
    #[serde(default)]
    pub mode: Mode,
    #[serde(default)]
    pub existing: Existing,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            version: 1,
            language: if sys_locale::get_locale()
                .unwrap_or_default()
                .to_lowercase()
                .starts_with("zh")
            {
                "zh-CN"
            } else {
                "en"
            }
            .into(),
            theme: "system".into(),
            jobs: font_converter_engine::default_jobs(),
            mode: Mode::Auto,
            existing: Existing::Skip,
        }
    }
}
impl Settings {
    pub fn path() -> Option<PathBuf> {
        directories::ProjectDirs::from("io.github", "mdfox-chaoszone", "fontconverter")
            .map(|d| d.config_dir().join("settings.json"))
    }
    pub fn load() -> (Self, Option<String>) {
        let Some(path) = Self::path() else {
            return (
                Self::default(),
                Some("Unable to locate user configuration directory".into()),
            );
        };
        Self::read(&path)
    }
    fn read(path: &std::path::Path) -> (Self, Option<String>) {
        if !path.exists() {
            return (Self::default(), None);
        }
        match fs::read(path)
            .ok()
            .and_then(|data| serde_json::from_slice::<Self>(&data).ok())
        {
            Some(value)
                if value.version == 1
                    && ["zh-CN", "en"].contains(&value.language.as_str())
                    && ["system", "light", "dark"].contains(&value.theme.as_str())
                    && (1..=32).contains(&value.jobs) =>
            {
                (value, None)
            }
            _ => (
                Self::default(),
                Some("Settings could not be read; defaults restored".into()),
            ),
        }
    }
    pub fn save(&self) -> Result<(), String> {
        let path = Self::path().ok_or("Unable to locate user configuration directory")?;
        self.write(&path)
    }
    fn write(&self, path: &std::path::Path) -> Result<(), String> {
        let parent = path.parent().unwrap();
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        let mut file = tempfile::NamedTempFile::new_in(parent).map_err(|e| e.to_string())?;
        file.write_all(&serde_json::to_vec_pretty(self).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
        file.as_file().sync_all().map_err(|e| e.to_string())?;
        file.persist(path).map_err(|e| e.to_string())?;
        Ok(())
    }
    pub fn zh(&self) -> bool {
        self.language == "zh-CN"
    }
}
pub fn tr(zh: bool, en: &'static str, cn: &'static str) -> &'static str {
    if zh { cn } else { en }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn corrupt_and_future_settings_recover_without_destroying_evidence() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("settings.json");
        fs::write(&path, b"{broken").unwrap();
        let (settings, notice) = Settings::read(&path);
        assert!(notice.is_some());
        assert!((1..=32).contains(&settings.jobs));
        assert_eq!(fs::read(&path).unwrap(), b"{broken");
        let future = Settings {
            version: 99,
            ..Settings::default()
        };
        future.write(&path).unwrap();
        assert!(Settings::read(&path).1.is_some());
        fs::write(
            &path,
            br#"{"version":1,"language":"en","theme":"light","jobs":4,"columns":[1,2,3,4,5]}"#,
        )
        .unwrap();
        let (legacy, notice) = Settings::read(&path);
        assert!(
            notice.is_none(),
            "settings from the previous layout still load"
        );
        assert_eq!((legacy.jobs, legacy.mode), (4, Mode::Auto));
        let valid = Settings {
            jobs: 32,
            mode: Mode::Decode,
            language: "en".into(),
            theme: "dark".into(),
            ..Settings::default()
        };
        valid.write(&path).unwrap();
        let (restored, notice) = Settings::read(&path);
        assert!(notice.is_none());
        assert_eq!(restored.jobs, 32);
        assert_eq!(restored.theme, "dark");
        assert_eq!(restored.mode, Mode::Decode);
        assert_eq!(fs::read_dir(temp.path()).unwrap().count(), 1);
    }
}
