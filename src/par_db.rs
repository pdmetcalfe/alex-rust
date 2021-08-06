use futures::Future;
use tokio::sync::oneshot;
use tokio::sync::mpsc;

pub use crate::db::{IndexEntry, Error, Database};


#[derive(Debug)]
enum Commands {
    Store(IndexEntry, oneshot::Sender<Result<(), Error>>),
    ContainsDate(chrono::NaiveDate, oneshot::Sender<Result<Option<i32>, Error>>),
    ContainsIndex(i32, oneshot::Sender<Result<Option<chrono::NaiveDate>, Error>>),
}

use Commands::*;

pub struct ParDbServer {
    database: Database,
    channel: mpsc::Receiver<Commands>,
}

#[derive(Clone)]
pub struct ParDbClient {
    channel: mpsc::Sender<Commands>
}

impl ParDbClient {
    pub async fn cartoon_index(&self, date: chrono::NaiveDate) -> Result<Option<i32>, Error> {
        let (tx, rx) = oneshot::channel();
        self.channel.send(Commands::ContainsDate(date, tx)).await.unwrap();
        rx.await.unwrap()
    }

    pub async fn cartoon_date(&self, index: i32) -> Result<Option<chrono::NaiveDate>, Error> {
        let (tx, rx) = oneshot::channel();
        self.channel.send(Commands::ContainsIndex(index, tx)).await.unwrap();
        rx.await.unwrap()
    }

    pub async fn add_cartoon(&self, cartoon: IndexEntry) -> Result<(), Error> {
        let (tx, rx) = oneshot::channel();
        self.channel.send(Commands::Store(cartoon, tx)).await.unwrap();
        rx.await.unwrap()
    }
}

pub fn new(database: Database) -> (ParDbClient, ParDbServer) {
    let (tx, rx) = mpsc::channel(32);
    (ParDbClient {channel: tx}, ParDbServer {database, channel:rx})
}

impl ParDbServer {
    fn mainloop(mut self) -> () {
        while let Some(item) = self.channel.blocking_recv() {
            match item {
                ContainsDate(date, tx) => {
                    tx.send(self.database.cartoon_index(&date)).unwrap();
                },
                ContainsIndex(index, tx) => {
                    tx.send(self.database.cartoon_date(index)).unwrap();
                },
                Store(entry, tx) => {   
                    tx.send(self.database.add_cartoon(&entry)).unwrap(); 
                }
            }
        }
    }
    pub fn run(self) -> impl Future<Output=Result<(), tokio::task::JoinError>> {
        tokio::task::spawn_blocking(|| {self.mainloop()} )
    } 
}

