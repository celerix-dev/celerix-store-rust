use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use crate::{Result, Error};
use log::warn;

#[allow(unused_imports)]
use crate::engine::MemStore;

/// Handles disk I/O for the [`MemStore`].
/// 
/// Persistence uses an atomic "write-then-rename" strategy to ensure data integrity.
/// Each persona is stored in its own `.json` file.
pub struct Persistence {
    data_dir: PathBuf,
}

impl Persistence {
    /// Initializes a new `Persistence` handler in the specified directory.
    /// 
    /// If the directory does not exist, it will be created.
    pub fn new<P: AsRef<Path>>(dir: P) -> Result<Self> {
        let dir = dir.as_ref().to_path_buf();
        if !dir.exists() {
            fs::create_dir_all(&dir)?;
        }
        Ok(Self { data_dir: dir })
    }

    /// Writes a single persona's data to a JSON file atomically.
    /// 
    /// This method writes to a temporary file first and then renames it to the
    /// final destination, preventing file corruption during power failures.
    pub fn save_persona(&self, persona_id: &str, data: &HashMap<String, HashMap<String, serde_json::Value>>) -> Result<()> {
        let file_path = self.data_dir.join(format!("{}.json", persona_id));
        let temp_path = file_path.with_extension("json.tmp");

        let bytes = serde_json::to_vec_pretty(data)?;
        
        fs::write(&temp_path, bytes)?;
        fs::rename(&temp_path, &file_path)?;

        Ok(())
    }

    /// Loads all persona data found in the data directory.
    /// 
    /// Scans for all `.json` files in the `data_dir` and parses them into the
    /// store's internal data structure.
    pub fn load_all(&self) -> Result<HashMap<String, HashMap<String, HashMap<String, serde_json::Value>>>> {
        let mut all_data = HashMap::new();

        if !self.data_dir.exists() {
            return Ok(all_data);
        }

        for entry in fs::read_dir(&self.data_dir)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                let persona_id = path.file_stem()
                    .and_then(|s| s.to_str())
                    .ok_or_else(|| Error::Internal("Invalid filename".to_string()))?
                    .to_string();

                let content = match fs::read(&path) {
                    Ok(c) => c,
                    Err(e) => {
                        warn!("Could not read persona file {:?}: {}", path, e);
                        continue;
                    }
                };

                let persona_data: HashMap<String, HashMap<String, serde_json::Value>> = match serde_json::from_slice(&content) {
                    Ok(d) => d,
                    Err(e) => {
                        warn!("Could not unmarshal persona data from {:?}: {}", path, e);
                        continue;
                    }
                };

                all_data.insert(persona_id, persona_data);
            }
        }

        Ok(all_data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use serde_json::json;

    #[test]
    fn test_save_and_load_all() {
        let dir = tempdir().unwrap();
        let persistence = Persistence::new(dir.path()).unwrap();

        let mut data = HashMap::new();
        let mut app_data = HashMap::new();
        app_data.insert("key1".to_string(), json!("value1"));
        data.insert("app1".to_string(), app_data);

        persistence.save_persona("p1", &data).unwrap();

        let loaded = persistence.load_all().unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded.get("p1").unwrap().get("app1").unwrap().get("key1").unwrap(), &json!("value1"));
    }

    #[test]
    fn test_atomic_rename() {
        let dir = tempdir().unwrap();
        let persistence = Persistence::new(dir.path()).unwrap();

        let mut data = HashMap::new();
        let mut app_data = HashMap::new();
        app_data.insert("key1".to_string(), json!("value1"));
        data.insert("app1".to_string(), app_data);

        persistence.save_persona("p1", &data).unwrap();

        let file_path = dir.path().join("p1.json");
        assert!(file_path.exists());
        
        let temp_path = dir.path().join("p1.json.tmp");
        assert!(!temp_path.exists());
    }

    #[test]
    fn test_go_compatibility() {
        // Mock the Go test data structure
        let go_json = r#"{
  "test_app": {
    "key_0": 0,
    "key_1": "string_val"
  }
}"#;
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("go_persona.json");
        fs::write(&file_path, go_json).unwrap();

        let persistence = Persistence::new(dir.path()).unwrap();
        let loaded = persistence.load_all().unwrap();
        
        let persona = loaded.get("go_persona").unwrap();
        let app = persona.get("test_app").unwrap();
        assert_eq!(app.get("key_0").unwrap(), &json!(0));
        assert_eq!(app.get("key_1").unwrap(), &json!("string_val"));
    }
}
