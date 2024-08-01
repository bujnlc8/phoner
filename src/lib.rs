pub mod util;

use std::{fmt::Debug, os::unix::fs::MetadataExt, path::PathBuf};

use anyhow::anyhow;
use colored::Colorize;
use tokio::{
    fs::{self, File},
    io::{AsyncReadExt, AsyncSeekExt},
};
use util::{download_file, replace_home, u8_i32};

#[derive(Debug)]
pub struct PhoneData {
    pub data_path: PathBuf,
    pub version: String,
    pub start_index: u32,
    pub record: Vec<u8>,
    pub index: Vec<u8>,
    pub size: u64,
}

// 索引数据
#[derive(Debug)]
pub struct IndexData {
    pub phone_prefix: i32,
    pub offset: i32,
    pub isp: u8,
}

#[derive(Debug)]
pub struct RecordData {
    pub province: String,
    pub city: String,
    pub zip_code: String,
    pub area_code: String,
    pub isp: String,
}

impl RecordData {
    pub fn display(&self) {
        if self.province != self.city {
            println!(
                "{} {}{} {}",
                format!("[{}]", self.area_code).magenta(),
                self.province.green(),
                self.city.green(),
                self.isp.red().bold()
            );
        } else {
            println!(
                "{} {} {}",
                format!("[{}]", self.area_code).magenta(),
                self.province.green(),
                self.isp.red().bold()
            );
        }
    }
}

// 运营商
enum Isp {
    Cmcc = 1,
    Cucc = 2,
    Ctcc = 3,
    CtccV = 4,
    CuccV = 5,
    CmccV = 6,
    Cbcc = 7,
    CbccV = 8,
}

impl Isp {
    fn from_u8(i: u8) -> Option<Isp> {
        match i {
            1 => Some(Isp::Cmcc),
            2 => Some(Isp::Cucc),
            3 => Some(Isp::Ctcc),
            4 => Some(Isp::CtccV),
            5 => Some(Isp::CuccV),
            6 => Some(Isp::CmccV),
            7 => Some(Isp::Cbcc),
            8 => Some(Isp::CbccV),
            _ => None,
        }
    }

    fn get_name(&self) -> String {
        match self {
            Isp::Cmcc => "中国移动".to_string(),
            Isp::Cucc => "中国联通".to_string(),
            Isp::Ctcc => "中国电信".to_string(),
            Isp::CtccV => "中国电信虚拟运营商".to_string(),
            Isp::CuccV => "中国联通虚拟运营商".to_string(),
            Isp::CmccV => "中国移动虚拟运营商".to_string(),
            Isp::Cbcc => "中国广电".to_string(),
            Isp::CbccV => "中国广电虚拟运营商".to_string(),
        }
    }
}

const PHONE_DATA_URL: &str = "https://raw.githubusercontent.com/ls0f/phone/master/phone/phone.dat";

impl PhoneData {
    pub fn new(data_path: Option<&str>) -> Self {
        let data_path = PathBuf::from(replace_home(
            data_path.unwrap_or(Self::get_data_path().as_str()),
        ));
        Self {
            data_path,
            version: String::new(),
            start_index: 0,
            record: vec![],
            index: vec![],
            size: 0,
        }
    }

    pub fn get_data_path() -> String {
        replace_home("~/.cache/phoner/phone.dat")
    }

    pub async fn download_file(&self, download_url: Option<String>) -> Result<(), anyhow::Error> {
        if !self.data_path.parent().unwrap().exists() {
            fs::create_dir_all(self.data_path.parent().unwrap()).await?;
        }
        download_file(
            download_url.unwrap_or(PHONE_DATA_URL.to_string()).as_str(),
            &PathBuf::from(Self::get_data_path()),
        )
        .await?;
        Ok(())
    }

    async fn init(&mut self) -> Result<(), anyhow::Error> {
        if !self.data_path.exists() {
            self.download_file(None).await?;
        }
        let mut file = File::open(self.data_path.clone()).await?;
        // 读取头部8个字节 版本号, 第一个索引的偏移
        let mut version_bytes: [u8; 4] = [0; 4];
        file.read_exact(&mut version_bytes).await?;
        let version = String::from_utf8_lossy(&version_bytes);
        self.version = version.to_string();
        file.seek(std::io::SeekFrom::Start(4)).await?;
        // 小端字节序
        let start_index = file.read_u32_le().await?;
        self.start_index = start_index;
        // 读取记录区[8:start_index]
        file.seek(std::io::SeekFrom::Start(8)).await?;
        let mut record = vec![0u8; (start_index - 8) as usize];
        file.read_exact(&mut record).await?;
        self.record = record;
        // 读取索引区
        let mut index = vec![];
        file.read_to_end(&mut index).await?;
        self.index = index;
        let metadata = file.metadata().await.unwrap();
        self.size = metadata.size();
        Ok(())
    }

    fn search_index(&self, phone: &str) -> Result<Option<IndexData>, anyhow::Error> {
        if self.index.is_empty() {
            return Err(anyhow!("数据未初始化"));
        }
        let phone_prefix: i32 = phone[..7].parse()?;
        // 采用二分法查找index
        let mut start = 0;
        let mut end = self.index.len() / 9 - 1;
        let mut position = None;
        loop {
            let mid = (start + end) / 2;
            // 前4位为手机号
            let prefix = &self.index[mid * 9..(mid * 9 + 4)];
            let prefix = u8_i32(prefix);
            if prefix > phone_prefix {
                if mid == 0 {
                    break;
                }
                end = mid - 1;
            } else if prefix == phone_prefix {
                position = Some(mid);
                break;
            } else {
                start = mid + 1;
            }
            if start > end {
                break;
            }
        }
        if let Some(pos) = position {
            let data = &self.index[pos * 9..(pos + 1) * 9];
            return Ok(Some(IndexData {
                phone_prefix: u8_i32(&data[..4]),
                offset: u8_i32(&data[4..8]),
                isp: u8_i32(&data[8..]) as u8,
            }));
        }
        Ok(None)
    }

    pub async fn query(&mut self, phone: &str, init: bool) -> Result<RecordData, anyhow::Error> {
        if init {
            self.init().await?;
        }
        let index = self.search_index(phone).unwrap();
        if index.is_none() {
            return Err(anyhow!("未找到数据"));
        }
        let index = index.unwrap();
        let mut record = self.read_recrod(index.offset).unwrap();
        if let Some(isp) = Isp::from_u8(index.isp) {
            record.isp = isp.get_name();
        }
        Ok(record)
    }

    fn read_recrod(&self, offset: i32) -> Result<RecordData, anyhow::Error> {
        let res: Vec<u8> = self
            .record
            .iter()
            .skip((offset - 8) as usize)
            .take_while(|x| **x != 0)
            .copied()
            .collect();
        let data = String::from_utf8(res).unwrap();
        let data_split: Vec<&str> = data.split('|').collect();
        if data_split.len() != 4 {
            return Err(anyhow!("数据格式错误: {}", data));
        }
        Ok(RecordData {
            province: data_split[0].to_string(),
            city: data_split[1].to_string(),
            zip_code: data_split[2].to_string(),
            area_code: data_split[3].to_string(),
            isp: String::new(),
        })
    }

    pub async fn print_db_info(&mut self) -> Result<(), anyhow::Error> {
        if !self.data_path.exists() {
            return Err(anyhow!("手机号码库不存在"));
        }
        self.init().await?;
        println!("{:<7}: {}", "version".green().bold(), self.version,);
        println!("{:<7}: {}", "lines".green().bold(), self.index.len() / 9);
        println!(
            "{:<7}: {} bytes ({:.4}MB)",
            "size".green().bold(),
            self.size,
            self.size as f64 / 1024.0 / 1024.0,
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::PhoneData;

    #[tokio::test]
    async fn test_query() -> Result<(), anyhow::Error> {
        let mut phone_data = PhoneData::new(Some("./phone.dat"));
        let record = phone_data.query("13456789012", true).await?;
        assert_eq!(record.province, "浙江");
        assert_eq!(record.city, "杭州");
        assert_eq!(record.isp, "中国移动");
        assert_eq!(record.zip_code, "310000");
        assert_eq!(record.area_code, "0571");
        Ok(())
    }
}
