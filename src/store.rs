use std::collections::HashSet;
use walkdir::WalkDir;

pub struct Contents {
    store: HashSet<i32>,
}

impl Contents {
    pub fn new() -> Self {
        Contents {
            store: WalkDir::new("out")
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().is_file())
                .filter_map(|e| -> Option<i32> {
                    e.path().file_stem()?.to_str()?.parse::<i32>().ok()
                })
                .collect(),
        }
    }

    pub fn contains(&self, x: &i32) -> bool {
        self.store.contains(x)
    }
}
