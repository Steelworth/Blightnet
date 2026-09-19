use rand::Rng;
use std::path::Path;

#[derive(Clone)]
pub struct Names {
    pub male: Vec<String>,
    pub female: Vec<String>,
    pub last: Vec<String>,
}

impl Names {
    pub fn load(root: &Path) -> Self {
        let raw = std::fs::read_to_string(root.join("data/names.json")).ok();
        let v: serde_json::Value = raw
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or(serde_json::json!({}));
        let take = |k: &str| {
            v.get(k)
                .and_then(|x| x.as_array())
                .map(|a| {
                    a.iter()
                        .filter_map(|x| x.as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default()
        };
        Self {
            male: take("male"),
            female: take("female"),
            last: take("last"),
        }
    }

    fn pick(rows: &[String]) -> String {
        if rows.is_empty() {
            return String::new();
        }
        rows[rand::thread_rng().gen_range(0..rows.len())].clone()
    }

    pub fn first(&self, female: bool) -> String {
        Self::pick(if female { &self.female } else { &self.male })
    }

    pub fn last(&self) -> String {
        Self::pick(&self.last)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    }

    #[test]
    fn lists_are_500_unique() {
        let n = Names::load(&root());
        assert_eq!(n.male.len(), 500);
        assert_eq!(n.female.len(), 500);
        assert_eq!(n.last.len(), 500);
        assert_eq!(n.male.iter().collect::<std::collections::HashSet<_>>().len(), 500);
        assert_eq!(n.female.iter().collect::<std::collections::HashSet<_>>().len(), 500);
        assert_eq!(n.last.iter().collect::<std::collections::HashSet<_>>().len(), 500);
    }

    #[test]
    fn shuffle_first_last_male_and_female() {
        let n = Names::load(&root());
        for _ in 0..40 {
            let f = n.first(true);
            let m = n.first(false);
            let l = n.last();
            assert!(n.female.iter().any(|x| x == &f), "{f}");
            assert!(n.male.iter().any(|x| x == &m), "{m}");
            assert!(n.last.iter().any(|x| x == &l), "{l}");
        }
    }

    #[test]
    fn empty_lists_do_not_panic() {
        let n = Names {
            male: vec![],
            female: vec![],
            last: vec![],
        };
        assert!(n.first(true).is_empty());
        assert!(n.first(false).is_empty());
        assert!(n.last().is_empty());
    }
}
