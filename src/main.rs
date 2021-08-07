mod store;

use futures::stream::{FuturesUnordered, StreamExt};
use reqwest::header::{HeaderValue, CONTENT_TYPE};
use reqwest::Client;
use scraper::{Html, Selector};
use std::io;
use std::path::Path;
use structopt::StructOpt;
use thiserror::Error;
use rand::seq::SliceRandom;
use store::{Contents, Storer};

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
    date_selector: Selector,
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
    fn extract_url(&self, doc: &str) -> Result<(ImgUrl, chrono::NaiveDate)> {
        let parsed = Html::parse_document(doc);
        let url = parsed
            .select(&self.img_selector)
            .next()
            .and_then(|x| x.value().attr("src"))
            .map(|x| ImgUrl(String::from(x)))
            .ok_or(Error::Parse)?;
        let date = parsed
            .select(&self.date_selector)
            .next()
            .and_then(|x| {
                let t: String = x.text().collect();
                chrono::NaiveDate::parse_from_str(&t, "%A, %-d %B %Y").ok()
            })
            .ok_or(Error::Parse)?;
        Ok((url, date))
    }

    async fn fetch_image(&self, date: chrono::NaiveDate, img: ImgUrl) -> Result<()> {
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

        let mut storer: Storer = Storer::new(self.target, date, mine)?;

        while let Some(chunk) = res.chunk().await? {
            storer.store(&chunk).await?;
        }

        Ok(())
    }

    async fn fetch_index(&self, index: i32) -> Result<(ImgUrl, chrono::NaiveDate)> {
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

    async fn raw_fetch(&self, index: i32) -> Result<chrono::NaiveDate> {
        let (url, date) = self.fetch_index(index).await?;
        self.fetch_image(date, url).await?;
        Ok(date)
    }

    async fn fetch(&self, index: i32) -> Option<i32> {
        println!("Beginning {}", index);
        match self.raw_fetch(index).await {
            Ok(date) => {
                println!("Completed {} ({})", index, date);
                Some(index)
            }
            Err(x) => {
                println!("Failed {}: {}", index, x);
                None
            }
        }
    }

    fn new(target: &'a std::path::Path) -> Self {
        let http_client = Client::builder().build().unwrap();
        let img_selector = Selector::parse("div.strip>img").unwrap();
        let date_selector = Selector::parse("div.date>h2").unwrap();
        AlexFetcher {
            http_client,
            img_selector,
            date_selector,
            target,
        }
    }
}

fn read_contents<P: AsRef<Path>>(path: P) -> Result<Contents> {
    let src = std::fs::OpenOptions::new()
        .read(true)
        .write(false)
        .open(path);
    match src {
        Err(_) => Ok(Default::default()),
        Ok(handle) => {
            let buf = std::io::BufReader::new(handle);
            serde_json::from_reader(buf).map_err(|_| Error::FileType)
        }
    }
}

fn store_contents<P: AsRef<Path>>(contents: &Contents, path: P) -> Result<()> {
    let dst = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(path)?;
    let writer = std::io::BufWriter::new(dst);
    serde_json::to_writer(writer, contents).unwrap();
    Ok(())
}

#[tokio::main()]
async fn main() {
    let opts = Config::from_args();
    std::fs::create_dir_all(&opts.target).unwrap();
    let out_file = opts.target.join("index.txt");
    let mut contents = read_contents(&out_file).unwrap();
    let fetcher = AlexFetcher::new(&opts.target);
    let mut to_do = {
        let mut src = (1..8000_i32)
        .into_iter()
        .filter(|x| !contents.contains(x))
        .map(|x| fetcher.fetch(x))
        .collect::<Vec<_>>();
        let mut rng = rand::thread_rng();
        src.shuffle(&mut rng);
        src.into_iter()
    };
    let mut workers: FuturesUnordered<_> = to_do.by_ref().take(opts.parallel).collect();
    while let Some(item) = workers.next().await {
        contents.extend(item);
        workers.extend(to_do.next());
    }
    store_contents(&contents, &out_file).unwrap();
}
