use std::fs;
use std::path::PathBuf;

pub struct Config {
    pub language: String,
    pub db_root_path: String,
    pub editor: String, // Новое поле для редактора
}

impl Config {
    pub fn load() -> Self {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        let config_path = PathBuf::from(&home).join(".config/ttytips/config.toml");
        let default_db = PathBuf::from(&home).join(".config/ttytips/db").to_string_lossy().into_owned();

        if config_path.exists() {
            if let Ok(content) = fs::read_to_string(config_path) {
                let mut lang = "ru".to_string();
                let mut path = default_db.clone();
                let mut ed = "nano".to_string(); // Дефолт
                
                for line in content.lines() {
                    if line.starts_with("language =") {
                        lang = line.split('=').nth(1).unwrap_or("ru").trim().replace('"', "");
                    }
                    if line.starts_with("db_root_path =") {
                        path = line.split('=').nth(1).unwrap_or(&default_db).trim().replace('"', "");
                    }
                    if line.starts_with("editor =") {
                        ed = line.split('=').nth(1).unwrap_or("nano").trim().replace('"', "");
                    }
                }
                return Config { language: lang, db_root_path: path, editor: ed };
            }
        }
        
        Config { language: "ru".to_string(), db_root_path: default_db, editor: "nano".to_string() }
    }

    pub fn save(&self) {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        let config_dir = PathBuf::from(&home).join(".config/ttytips");
        let config_path = config_dir.join("config.toml");
        let _ = fs::create_dir_all(&config_dir);

        let content = format!(
            "language = \"{}\"\ndb_root_path = \"{}\"\neditor = \"{}\"\n",
            self.language, self.db_root_path, self.editor
        );
        let _ = fs::write(config_path, content);
    }
}
