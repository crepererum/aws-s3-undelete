use anyhow::{Context, Error, Result};
use aws_sdk_s3::Client;
use clap::Parser;
use futures_concurrency::prelude::*;
use tokio::{
    fs::File,
    io::{AsyncBufReadExt, BufReader},
};

#[derive(Debug, Parser, Clone)]
struct Config {
    #[clap(long)]
    input_file: String,

    #[clap(long)]
    bucket: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cfg = Config::parse();

    let shared_config = aws_config::load_from_env().await;
    let client = Client::new(&shared_config);

    let file = File::open(&cfg.input_file)
        .await
        .context("Failed to open file")?;
    let reader = BufReader::new(file);
    let lines = reader.lines();
    futures::stream::unfold(lines, |mut lines| async move {
        lines
            .next_line()
            .await
            .context("read line")
            .transpose()
            .map(|res| (res, lines))
    })
    .co()
    .try_for_each::<_, _, Error>(|line_res| {
        let client = &client;
        let cfg = &cfg;
        async move {
            let line = line_res?;
            match process_line(&line, cfg, client).await {
                Ok(()) => {
                    println!("done: {line}")
                }
                Err(e) => {
                    println!("cannot process line: {e}");
                }
            }
            Result::Ok(())
        }
    })
    .await?;

    Ok(())
}

async fn process_line(line: &str, cfg: &Config, client: &Client) -> Result<()> {
    let versions = client
        .list_object_versions()
        .bucket(&cfg.bucket)
        .prefix(line)
        .send()
        .await
        .context("get object versions")?;
    let Some(delete_markers) = versions.delete_markers else {
        return Ok(());
    };
    let delete_markers = delete_markers
        .into_iter()
        .filter(|marker| marker.is_latest().unwrap_or_default());

    for marker in delete_markers {
        let Some(key) = marker.key else {
            continue;
        };
        let Some(version_id) = marker.version_id else {
            continue;
        };
        client
            .delete_object()
            .bucket(&cfg.bucket)
            .key(key)
            .version_id(version_id)
            .send()
            .await
            .context("cannot delete marker")?;
    }

    Ok(())
}
