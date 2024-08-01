#![allow(deprecated)]

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

// 下载文件
pub async fn download_file(download_url: &str, dest: &Path) -> Result<(), anyhow::Error> {
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

pub fn replace_home(p: &str) -> String {
    if p.starts_with('~') {
        let home = env::home_dir().unwrap();
        return p.replace("~", home.to_str().unwrap());
    }
    p.to_string()
}
