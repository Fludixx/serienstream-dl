use crate::serienstream::Host;
use std::fs::File;

pub mod vidoza;
pub mod vivo;

pub struct Downloader {
    url: String,
    video_url: String,
    file_name: String,
}

impl Downloader {
    pub fn get_name(&self) -> String {
        String::from(self.file_name.replace(&self.get_extension(), ""))
    }

    pub fn get_file_name(&self) -> String {
        self.file_name.clone()
    }

    pub fn get_url(&self) -> String {
        self.url.clone()
    }

    pub fn get_extension(&self) -> String {
        String::from(self.file_name.split(".").last().unwrap())
    }

    pub fn download_to_file(&self, file: &mut File) {
        let mut video = reqwest::get(&self.video_url).expect("Failed to open video");
        video.copy_to(file);
    }
}
