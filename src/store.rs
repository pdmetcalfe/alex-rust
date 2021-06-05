use std::collections::HashSet;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use walkdir::WalkDir;

pub struct Contents {
    store: HashSet<i32>,
}

impl Contents {
    pub fn new(target: &std::path::Path) -> Self {
        Contents {
            store: WalkDir::new(target)
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

pub struct Storer {
    dst: File,
}

impl Storer {
    pub fn new(
        out_dir: &std::path::Path,
        target: &i32,
        extension: &str,
    ) -> Result<Self, std::io::Error> {
        let s = out_dir.join(format!("{:04}", 100 * (target / 100)));
        std::fs::create_dir_all(&s)?;
        let s = s.join(format!("{:04}.{}", target, extension));

        Ok(Storer {
            dst: File::from_std(std::fs::File::create(&s)?),
        })
    }

    pub async fn store(&mut self, payload: &[u8]) -> Result<(), std::io::Error> {
        self.dst.write_all(payload).await
    }
}
