use anyhow::Result;
use directories::ProjectDirs;
use std::path::PathBuf;

#[derive(Default)]
pub struct Config {
    config_dir: PathBuf,
}

impl Config {
    pub fn new() -> Result<Self> {
        let proj_dirs = ProjectDirs::from("com", "yandex", "downloader")
            .ok_or_else(|| anyhow::anyhow!("Не удалось определить директорию конфигурации"))?;
        let config_dir = proj_dirs.config_dir().to_path_buf();
        std::fs::create_dir_all(&config_dir)?;
        Ok(Config { config_dir })
    }

    pub fn get_config_file_path(&self) -> PathBuf {
        self.config_dir.join("profiles.json")
    }

    pub fn get_cookies_file_path(&self) -> PathBuf {
        self.config_dir.join("cookies.json")
    }
}