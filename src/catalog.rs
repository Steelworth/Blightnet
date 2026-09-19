use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
pub struct Layer {
    pub id: String,
    pub name: String,
    pub category: String,
    #[serde(default)]
    pub mood: String,
    #[serde(default)]
    pub file: String,
    #[serde(default)]
    pub files: Vec<String>,
    #[serde(default)]
    pub world: String,
}

impl Layer {
    pub fn audio_file(&self) -> Option<&str> {
        if !self.file.is_empty() {
            Some(self.file.as_str())
        } else {
            self.files.first().map(String::as_str)
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
pub struct Scene {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub blurb: String,
    #[serde(default)]
    pub mood: String,
    #[serde(default)]
    pub layers: HashMap<String, f32>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
pub struct Setting {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub indoor: bool,
}

#[derive(Debug, Deserialize)]
struct File {
    layers: Vec<Layer>,
    #[serde(rename = "blightLayers")]
    blight_layers: Vec<Layer>,
    scenes: Vec<Scene>,
    #[serde(rename = "blightScenes")]
    blight_scenes: Vec<Scene>,
    settings: Vec<Setting>,
    #[serde(rename = "blightSettings")]
    blight_settings: Vec<Setting>,
}

#[derive(Debug, Clone)]
pub struct Catalog {
    pub hearth_layers: Vec<Layer>,
    pub blight_layers: Vec<Layer>,
    pub hearth_scenes: Vec<Scene>,
    pub blight_scenes: Vec<Scene>,
    pub hearth_settings: Vec<Setting>,
    pub blight_settings: Vec<Setting>,
}

impl Catalog {
    pub fn load(root: &Path) -> Result<Self, String> {
        let path = root.join("data/mixer-catalog.json");
        let raw = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
        let f: File = serde_json::from_str(&raw).map_err(|e| e.to_string())?;
        Ok(Self {
            hearth_layers: f.layers,
            blight_layers: f.blight_layers,
            hearth_scenes: f.scenes,
            blight_scenes: f.blight_scenes,
            hearth_settings: f.settings,
            blight_settings: f.blight_settings,
        })
    }

    pub fn layers(&self, blight: bool) -> &[Layer] {
        if blight {
            &self.blight_layers
        } else {
            &self.hearth_layers
        }
    }

    pub fn scenes(&self, blight: bool) -> &[Scene] {
        if blight {
            &self.blight_scenes
        } else {
            &self.hearth_scenes
        }
    }

    pub fn settings(&self, blight: bool) -> &[Setting] {
        if blight {
            &self.blight_settings
        } else {
            &self.hearth_settings
        }
    }

    pub fn layer(&self, blight: bool, id: &str) -> Option<&Layer> {
        self.layers(blight).iter().find(|l| l.id == id)
    }
}

pub fn load_list(root: &Path, name: &str) -> Vec<serde_json::Value> {
    let path: PathBuf = root.join("data").join(name);
    std::fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    }

    #[test]
    fn catalog_loads_both_worlds() {
        let c = Catalog::load(&root()).expect("catalog");
        assert!(c.hearth_layers.len() > 50);
        assert!(c.blight_layers.len() > 400);
        assert!(c.hearth_scenes.len() > 10);
        assert!(c.blight_scenes.len() > 10);
        assert!(c.hearth_settings.len() >= 10);
        assert!(c.blight_settings.len() >= 10);
        assert!(c.layer(false, "tavern_jig").is_some());
        let music = c
            .layers(true)
            .iter()
            .filter(|l| l.category == "music")
            .count();
        assert!(music > 400);
        for mood in ["rebellious", "melancholic", "relaxing", "brutal"] {
            assert!(
                c.layers(true)
                    .iter()
                    .any(|l| l.category == "music" && l.mood == mood),
                "{mood}"
            );
        }
    }

    #[test]
    fn every_layer_has_an_audio_file_on_disk() {
        let root = root();
        let c = Catalog::load(&root).unwrap();
        let mut missing = Vec::new();
        for blight in [false, true] {
            for l in c.layers(blight) {
                if let Some(f) = l.audio_file() {
                    if !root.join(f).is_file() {
                        missing.push(format!("{} {}", l.id, f));
                    }
                } else {
                    missing.push(format!("{} (no file)", l.id));
                }
            }
        }
        assert!(missing.is_empty(), "missing audio: {missing:?}");
    }

    #[test]
    fn place_paintings_exist_or_day_fallback() {
        let root = root();
        let c = Catalog::load(&root).unwrap();
        let mut missing = Vec::new();
        for (blight, dir) in [(false, "assets/places"), (true, "assets/places-blight")] {
            for s in c.settings(blight) {
                let day = root.join(dir).join(format!("{}-day.jpg", s.id));
                if !day.is_file() {
                    missing.push(format!("{dir}/{}-day.jpg", s.id));
                }
            }
        }
        assert!(missing.is_empty(), "missing paintings: {missing:?}");
    }

    #[test]
    fn json_catalogs_parse_and_have_names() {
        let root = root();
        for file in [
            "bestiary.json",
            "srd-npcs.json",
            "gods.json",
            "datashard.json",
            "npcs.json",
            "srd-kit.json",
            "red-kit.json",
            "gangs.json",
            "corps.json",
            "lore.json",
        ] {
            let rows = load_list(&root, file);
            assert!(!rows.is_empty(), "{file} empty");
            assert!(
                rows.iter().all(|r| r.get("name").and_then(|x| x.as_str()).is_some()),
                "{file} missing name"
            );
        }
    }

    #[test]
    fn audio_file_falls_back_to_files_array() {
        let l = Layer {
            id: "x".into(),
            name: "x".into(),
            category: "music".into(),
            mood: String::new(),
            file: String::new(),
            files: vec!["audio/a.ogg".into()],
            world: String::new(),
        };
        assert_eq!(l.audio_file(), Some("audio/a.ogg"));
        let l = Layer {
            file: "audio/b.ogg".into(),
            files: vec!["audio/a.ogg".into()],
            ..l
        };
        assert_eq!(l.audio_file(), Some("audio/b.ogg"));
    }
}
