use core::iter::{Extend, FromIterator};
use std::collections::{hash_map, HashMap};
use std::io::Result;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use chrono::NaiveDate;

use serde::{Deserialize, Serialize};

#[derive(Default, Serialize, Deserialize)]
pub struct Contents(HashMap<i32, NaiveDate>);

impl Extend<(i32, chrono::NaiveDate)> for Contents {
    fn extend<T: IntoIterator<Item = (i32, NaiveDate)>>(&mut self, iter: T) {
        self.0.extend(iter);
    }
}

impl IntoIterator for Contents {
    type IntoIter = hash_map::IntoIter<i32, NaiveDate>;
    type Item = (i32, NaiveDate);
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl FromIterator<(i32, NaiveDate)> for Contents {
    fn from_iter<T>(iter: T) -> Self
    where
        T: IntoIterator<Item = (i32, NaiveDate)>,
    {
        Contents(iter.into_iter().collect())
    }
}

impl Contents {
    pub fn contains(&self, x: &i32) -> bool {
        self.0.contains_key(x)
    }
}

pub struct Storer {
    dst: File,
}

impl<I: Into<File>> From<I> for Storer {
    fn from(dst: I) -> Self {
        Storer { dst: dst.into() }
    }
}

impl Storer {
    pub fn new(
        out_dir: &std::path::Path,
        target: chrono::NaiveDate,
        extension: &str,
    ) -> Result<Self> {
        let s = out_dir.join(format!("{}", target.format("%Y")));
        std::fs::create_dir_all(&s)?;
        let s = s.join(format!("{}.{}", target.format("%Y-%m-%d"), extension));

        Ok(Storer::from(std::fs::File::create(&s)?))
    }

    pub async fn store(&mut self, payload: &[u8]) -> Result<()> {
        self.dst.write_all(payload).await
    }
}
