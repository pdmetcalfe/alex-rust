use rusqlite::{params, OptionalExtension, Result};
use std::path::Path;

pub type Error = rusqlite::Error;

#[derive(Clone, Copy, Debug)]
pub struct IndexEntry {
    index: i32,
    date: chrono::NaiveDate,
}

impl IndexEntry {
    pub fn new(index: i32, date: chrono::NaiveDate) -> Self {
        IndexEntry { index, date }
    }
}

#[derive(Debug)]
pub struct Database {
    connection: rusqlite::Connection,
}

impl Database {
    pub fn new<P>(path: P) -> Result<Self>
    where
        P: AsRef<Path>,
    {
        let connection = rusqlite::Connection::open(path)?;
        connection.execute(
            "
                create table if not exists cartoons (
                    ix INTEGER primary key,
                    date DATE not null
                )
            ",
            [],
        )?;
        Ok(Database { connection })
    }

    pub fn cartoon_index(&self, date: &chrono::NaiveDate) -> Result<Option<i32>> {
        self.connection
            .query_row(
                "
            select ix from cartoons where date = (?)
        ",
                params![date],
                |row| row.get(0),
            )
            .optional()
    }

    pub fn cartoon_date(&self, index: i32) -> Result<Option<chrono::NaiveDate>> {
        self.connection
            .query_row("select date from cartoons where ix = (?)",
            params![index],
            |row| row.get(0))
            .optional()
    }

    pub fn add_cartoon(&self, cartoon: &IndexEntry) -> Result<()> {
        self.connection.execute(
            "
        insert into cartoons (ix, date) values (?, ?)
        ",
            params![cartoon.index, cartoon.date],
        )?;
        Ok(())
    }
}
