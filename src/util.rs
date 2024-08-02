#![allow(deprecated)]
#![allow(unused_imports)]

use colored::Colorize;

use std::{
    env,
    io::{self, Write},
    path::Path,
    time::Duration,
};

use tokio::{
    fs, io as t_io,
    sync::oneshot::{self, Sender},
    task::JoinHandle,
    time::sleep,
};

// 小端字节序slice转i32
pub fn u8_i32(data: &[u8]) -> i32 {
    let mut res = 0;
    for (i, v) in data.iter().enumerate() {
        res += (*v as i32) << (8 * i);
    }
    res
}

pub fn clear_current_line() {
    // 使用 ANSI 转义序列清除行并将光标移到行首
    print!("\r\x1B[2K");
    io::stdout().flush().unwrap();
}

#[derive(Debug)]
pub struct WaitBlinker {
    pub sender: Sender<bool>,
    pub handle: JoinHandle<()>,
}

pub fn wait_blink(msg: &str, blink_char_num: usize) -> WaitBlinker {
    let msg = msg.to_string();
    let (tx, mut rx) = oneshot::channel::<bool>();
    let handle = tokio::spawn(async move {
        loop {
            print!("{}", format!("\r{}", msg).green());
            io::stdout().flush().unwrap();
            sleep(Duration::from_millis(120)).await;
            print!(
                "{}",
                format!(
                    "\r{}{}",
                    msg.chars()
                        .take(msg.chars().count() - blink_char_num)
                        .collect::<String>(),
                    " ".repeat(blink_char_num),
                )
                .green()
            );
            io::stdout().flush().unwrap();
            sleep(Duration::from_millis(50)).await;
            if rx.try_recv().is_ok() {
                clear_current_line();
                break;
            }
        }
    });
    WaitBlinker { sender: tx, handle }
}

// 静默下载
pub async fn download_file_silent(download_url: &str, dest: &Path) -> Result<(), anyhow::Error> {
    let response = reqwest::get(download_url).await?;
    let dest_dir = dest.parent().unwrap();
    if !dest_dir.exists() {
        fs::create_dir_all(dest_dir).await?;
    }
    t_io::copy(
        &mut response.bytes().await?.as_ref(),
        &mut fs::File::create(dest).await?,
    )
    .await?;
    Ok(())
}

// 下载文件
#[cfg(not(feature = "download-progress"))]
pub async fn download_file(download_url: &str, dest: &Path) -> Result<(), anyhow::Error> {
    download_file_silent(download_url, dest).await
}

pub fn replace_home(p: &str) -> String {
    if p.starts_with('~') {
        let home = env::home_dir().unwrap();
        return p.replace("~", home.to_str().unwrap());
    }
    p.to_string()
}

#[cfg(feature = "download-progress")]
pub async fn download_file(download_url: &str, dest: &Path) -> Result<(), anyhow::Error> {
    use std::time;

    use indicatif::{ProgressBar, ProgressStyle};
    use tokio::io::AsyncWriteExt;
    let start = time::Instant::now();
    let mut response = reqwest::Client::new().get(download_url).send().await?;
    let total_size = response.content_length().unwrap();
    let pb = ProgressBar::new(total_size);
    pb.set_style(ProgressStyle::default_bar().template("{spinner:.green} [{elapsed_precise}] [{bar:50.cyan/blue}] {bytes}/{total_bytes} ({eta})").unwrap().progress_chars("#>-"));
    pb.clone().with_elapsed(start.elapsed());
    let dest_dir = dest.parent().unwrap();
    if !dest_dir.exists() {
        fs::create_dir_all(dest_dir).await?;
    }
    let mut file = fs::File::create(dest).await?;
    while let Some(chunk) = response.chunk().await? {
        pb.inc(chunk.len() as u64);
        file.write(&chunk).await?;
    }
    pb.finish();
    Ok(())
}

#[cfg(test)]
mod tests {

    use std::path::Path;

    use super::download_file;

    #[tokio::test]
    async fn download_file_with_progress_test() -> Result<(), anyhow::Error> {
        download_file(
            "https://raw.githubusercontent.com/ls0f/phone/master/phone/phone.dat",
            Path::new("./phone.dat").as_ref(),
        )
        .await?;
        Ok(())
    }
}
