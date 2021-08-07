use core::iter::{Extend, FromIterator};
use std::collections::HashSet;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;

use serde::{Deserialize, Serialize};

#[derive(Default, Serialize, Deserialize)]
pub struct Contents(HashSet<i32>);

impl Extend<i32> for Contents {
    fn extend<T: IntoIterator<Item = i32>>(&mut self, iter: T) {
        self.0.extend(iter);
    }
}

impl FromIterator<i32> for Contents {
    fn from_iter<T>(iter: T) -> Self
    where
        T: IntoIterator<Item = i32>,
    {
        Contents(iter.into_iter().collect())
    }
}

impl Contents {
    pub fn contains(&self, x: &i32) -> bool {
        self.0.contains(x)
    }
}

pub struct Storer {
    dst: File,
}

impl Storer {
    pub fn new(
        out_dir: &std::path::Path,
        target: chrono::NaiveDate,
        extension: &str,
    ) -> Result<Self, std::io::Error> {
        let s = out_dir.join(format!("{}", target.format("%Y")));
        std::fs::create_dir_all(&s)?;
        let s = s.join(format!("{}.{}", target.format("%Y-%m-%d"), extension));

        Ok(Storer {
            dst: File::from_std(std::fs::File::create(&s)?),
        })
    }

    pub async fn store(&mut self, payload: &[u8]) -> Result<(), std::io::Error> {
        self.dst.write_all(payload).await
    }
}
