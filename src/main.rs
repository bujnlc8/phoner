use std::{process::exit, time};

use clap::{CommandFactory, Parser};
use colored::Colorize;
use phoner::{util::wait_blink, PhoneData};

#[derive(Parser)]
#[command(version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    /// 更新手机号码库
    #[clap(long, conflicts_with_all = vec!["db_info"])]
    update: bool,

    /// 更新手机号码库链接，默认 https://raw.githubusercontent.com/ls0f/phone/master/phone/phone.dat
    #[clap(long)]
    update_url: Option<String>,

    /// 查看手机号码库信息
    #[clap(long, conflicts_with_all = vec!["update", "update_url"])]
    db_info: bool,

    /// 手机号码
    phone: Option<String>,
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let cli = Cli::parse();
    if cli.phone.is_some() && cli.phone.clone().unwrap().len() != 11 {
        eprintln!("{}", "手机号码格式错误".red());
        exit(1);
    }
    let mut client = PhoneData::new(None);
    if cli.update {
        let start = time::Instant::now();
        let wait = wait_blink("正在下载...", 3);
        client.download_file(cli.update_url).await?;
        wait.sender.send(true).unwrap();
        wait.handle.await?;
        println!(
            "{} {}",
            "下载完成 ✅".green(),
            format!("{}ms elapsed.", start.elapsed().as_millis()).bright_black()
        );
        return Ok(());
    }
    if cli.db_info {
        client.print_db_info().await?;
        return Ok(());
    }
    if cli.phone.is_none() {
        Cli::command().print_help().unwrap();
        exit(0);
    }
    let wait = wait_blink("正在查询，请稍候🔎...", 3);
    let record = client.query(&cli.phone.unwrap(), true).await;
    wait.sender.send(true).unwrap();
    wait.handle.await?;
    match record {
        Err(e) => {
            eprintln!("{}", e.to_string().red());
            exit(1)
        }
        Ok(data) => data.display(),
    }
    Ok(())
}
