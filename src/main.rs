mod store;

use futures::stream::{self, StreamExt};
use reqwest::header::{HeaderValue, CONTENT_TYPE};
use reqwest::Client;
use scraper::{Html, Selector};
use std::io;

use thiserror::Error;

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

struct AlexFetcher {
    http_client: Client,
    img_selector: Selector,
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

impl AlexFetcher {
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

        let mut storer: store::Storer = store::Storer::new(&idx, mine)?;

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

    async fn fetch(&self, index:i32) {
        println!("Beginning {}", index);
        if let Err(x) = self.raw_fetch(index).await {
            println!("{}", x);
        } else {
            println!("Completed {}", index);
        }
    }

    fn new() -> Self {
        let http_client = Client::builder().build().unwrap();
        let img_selector = Selector::parse("div.strip>img").unwrap();
        AlexFetcher {
            http_client,
            img_selector,
        }
    }
}

#[tokio::main()]
async fn main() {
    let fetcher = AlexFetcher::new();
    let contents = store::Contents::new();

    stream::iter((1..8000_i32).into_iter().filter(|x| !contents.contains(x)))
        .for_each_concurrent(Some(10), |x| fetcher.fetch(x))
        .await;
}
