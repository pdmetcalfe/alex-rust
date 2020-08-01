use futures::stream::{self, StreamExt};
use reqwest::{Client};
use reqwest::header::{HeaderValue, CONTENT_TYPE};
use scraper::{Selector, Html};
use tokio::fs::File;
use tokio::io::{self, AsyncWriteExt};

use std::io::ErrorKind;

use thiserror::Error;

#[derive(Error, Debug)]
enum Error {
    #[error("HTTP request error")]
    Request(#[from] reqwest::Error),
    #[error("general IO error")]
    IO(#[from] io::Error),
    #[error("parse failure")]
    Parse,
}

#[derive(Debug, Clone)]
struct ImgUrl(String);

struct AlexFetcher {
    http_client: Client,
    img_selector: Selector,
}

fn get_end(v: &HeaderValue) -> &str {
    match v.to_str() {
	Ok("image/png") => &"png",
	Ok("image/jpg") => &"jpg",
	Ok("image/gif") => &"gif",
	Ok("image/jpeg") => &"jpg",
	_            => {
	    &"unknown"
	}
    }
}

impl AlexFetcher {
    fn extract_url(&self, doc: &str) -> Result<ImgUrl, Error> {
	let parsed = Html::parse_document(doc);
	parsed.select(&self.img_selector)
	    .next()
	    .and_then(|x| x.value().attr("src"))
	    .map(|x| ImgUrl(String::from(x)))
	    .ok_or(Error::Parse)
    }

    async fn fetch_image(&self, idx: i32, img: ImgUrl) -> Result<i32, Error> {
	let mut res = self.http_client.get(&img.0)
	    .send()
	    .await?
	    .error_for_status()?;
	let mine = res
	    .headers()
	    .get(CONTENT_TYPE)
	    .map(get_end)
	    .unwrap_or("unknown");
	let s = format!("out/{}.{}", idx, mine);
	
	let mut dest = File::create(s).await?;

	while let Some(chunk) = res.chunk().await? {
	    dest.write_all(&chunk).await?;
	}
	
	Ok(idx)
    }
    
    async fn fetch_index(&self, index: i32) -> Result<ImgUrl, Error> {
	let text = self.http_client
	    .get("https://www.alexcartoon.com/index.cfm")
	    .query(&[("cartoon_num", index)])
	    .send()
            .await?
	    .error_for_status()?
	    .text()
	    .await?;
	self.extract_url(&text)
    }

    async fn fetch(&self, index: i32) -> Result<i32, Error> {
	let url = self.fetch_index(index).await?;
	self.fetch_image(index, url).await
    }

    async fn full_fetch(&self, index:i32) {
	let mut stdout = tokio::io::stdout();
	stdout.write_all(format!("Beginning {}\n", index).as_bytes()).await;
	let res = self.fetch(index).await;
	stdout.write_all(format!("{:?}\n", res).as_bytes()).await;
    }
    
    fn new() -> Self {
	let http_client = Client::builder()
	    .use_rustls_tls()
	    .build()
	    .unwrap();
	let img_selector = Selector::parse("div.strip>img").unwrap();
	AlexFetcher {http_client, img_selector}
    }
}


#[tokio::main()]
async fn main() {
    let fetcher = AlexFetcher::new();

    if let Err(e) = tokio::fs::create_dir("out").await {
	if e.kind() != ErrorKind::AlreadyExists {
	    return
	}
    }
    
    stream::iter(1..8000_i32)
	.for_each_concurrent(Some(100), |x| fetcher.full_fetch(x))
	.await;
}

