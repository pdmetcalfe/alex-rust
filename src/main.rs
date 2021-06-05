mod store;

use futures::stream::{self, StreamExt};
use reqwest::header::{HeaderValue, CONTENT_TYPE};
use reqwest::Client;
use scraper::{Html, Selector};
use std::io;
use structopt::StructOpt;

use thiserror::Error;
#[derive(StructOpt)]
#[structopt(name = "Alex fetcher", about = "Screen scrape all the alex cartoons")]
struct Config {
    /// number of asynchronous workers
    #[structopt(short, long, default_value = "10")]
    parallel: usize,

    /// output directory
    #[structopt(default_value = "out")]
    target: std::path::PathBuf,
}

#[derive(Error, Debug)]
enum Error {
    #[error("HTTP request error")]
    Request(#[from] reqwest::Error),
    #[error("general IO error")]
    IO(#[from] io::Error),
    #[error("parse failure")]
    Parse,
    #[error("unknown file type")]
    FileType,
}

type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone)]
struct ImgUrl(String);

struct AlexFetcher<'a> {
    http_client: Client,
    img_selector: Selector,
    target: &'a std::path::Path,
}

fn get_end(v: &HeaderValue) -> Result<&str> {
    v.to_str()
        .map_err(|_| Error::FileType)
        .and_then(|x| match x {
            "image/png" => Ok("png"),
            "image/jpg" => Ok("jpg"),
            "image/gif" => Ok("gif"),
            "image/jpeg" => Ok("jpg"),
            _ => Err(Error::FileType),
        })
}

impl<'a> AlexFetcher<'a> {
    fn extract_url(&self, doc: &str) -> Result<ImgUrl> {
        let parsed = Html::parse_document(doc);
        parsed
            .select(&self.img_selector)
            .next()
            .and_then(|x| x.value().attr("src"))
            .map(|x| ImgUrl(String::from(x)))
            .ok_or(Error::Parse)
    }

    async fn fetch_image(&self, idx: i32, img: ImgUrl) -> Result<()> {
        let mut res = self
            .http_client
            .get(&img.0)
            .send()
            .await?
            .error_for_status()?;
        let mine = res
            .headers()
            .get(CONTENT_TYPE)
            .ok_or(Error::FileType)
            .and_then(get_end)?;

        let mut storer: store::Storer = store::Storer::new(self.target, &idx, mine)?;

        while let Some(chunk) = res.chunk().await? {
            storer.store(&chunk).await?;
        }

        Ok(())
    }

    async fn fetch_index(&self, index: i32) -> Result<ImgUrl> {
        let text = self
            .http_client
            .get("https://www.alexcartoon.com/index.cfm")
            .query(&[("cartoon_num", index)])
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?;
        self.extract_url(&text)
    }

    async fn raw_fetch(&self, index: i32) -> Result<()> {
        let url = self.fetch_index(index).await?;
        self.fetch_image(index, url).await
    }

    async fn fetch(&self, index: i32) {
        println!("Beginning {}", index);
        match self.raw_fetch(index).await {
            Ok(_) => println!("Completed {}", index),
            Err(x) => println!("Failed {}: {}", index, x),
        }
    }

    fn new(target: &'a std::path::Path) -> Self {
        let http_client = Client::builder().build().unwrap();
        let img_selector = Selector::parse("div.strip>img").unwrap();
        AlexFetcher {
            http_client,
            img_selector,
            target,
        }
    }
}

#[tokio::main()]
async fn main() {
    let opts = Config::from_args();
    std::fs::create_dir_all(&opts.target).unwrap();
    let contents = store::Contents::new(&opts.target);
    let fetcher = AlexFetcher::new(&opts.target);

    stream::iter((1..8000_i32).into_iter().filter(|x| !contents.contains(x)))
        .for_each_concurrent(Some(opts.parallel), |x| fetcher.fetch(x))
        .await;
}
