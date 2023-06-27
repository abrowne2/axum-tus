use async_trait::async_trait;
use bytes::Bytes;
use std::{
    error::Error,
    fs::{self, File},
    io::{BufReader, ErrorKind, Seek, SeekFrom, Write},
    path::Path,
};

use super::{
    file_info::{Built, Completed, Created, FileInfo, Terminated, self},
    metadata::Metadata,
};

pub enum PatchOption {
    Patched(u64),
    Completed(FileInfo<Completed>),
}

// todo do we need build_file? it may do the setup by pulling the metadata from the file itself (this includes the tus protocol stuff)
#[async_trait]
pub trait FileStore: Send + Sync {
    async fn create_file(&self, file_info: FileInfo<Built>) -> Result<FileInfo<Created>, FileStoreError>;
    async fn patch_file(&self, file_id: &str, offset: u64, data: &mut [u8]) -> Result<PatchOption, FileStoreError>;   
    async fn delete_file(&self, file_id: &str) -> Result<(), FileStoreError>;
    async fn get_file_info(&self, file_id: &str) -> Result<(u64, Option<String>), FileStoreError>; // return file length and file type
    async fn exists(&self, file_id: &str) -> bool;
}

#[derive(Debug)]
pub enum FileStoreError {
    CreationError(Box<dyn std::error::Error>),
    ReadError(Box<dyn std::error::Error>),
    TerminationError(Box<dyn std::error::Error>),
    Error
}

pub struct LocalFileStore {
    root_path: String
}

impl LocalFileStore {
    pub fn new(root_path: String) -> Self {
        Self {
            root_path
        }
    }

    fn read_file<State>(
        &self,
        file_id: &str,
    ) -> Result<FileInfo<State>, FileStoreError> {
        let file_path = format!("{}/{}", self.root_path, file_id);
        println!("When reading file, file_path: {}", file_path);

        let file = std::fs::File::open(file_path).map_err(|e| FileStoreError::ReadError(Box::new(e)))?;

        let metadata = file.metadata().map_err(|e| FileStoreError::ReadError(Box::new(e)))?;
        
        let reader = BufReader::new(file);

        serde_json::from_reader(reader)
            .map_err(|e| FileStoreError::ReadError(Box::new(e)))
    }
}

#[async_trait]
impl FileStore for LocalFileStore {
    async fn exists(
        &self,
        file_id: &str,
    ) -> bool {
        let file_path = format!("{}/{}", self.root_path, file_id);
        println!("When checking file existence, file_path: {}", file_path);

        let file = std::fs::File::open(file_path);
        if file.is_ok() {
            true
        } else {
            false
        }
    }

    async fn create_file(
        &self,
        file_info: FileInfo<Built>,
    ) -> Result<FileInfo<Created>, FileStoreError> {
        let file_dir = Path::new(&self.root_path);
        if !file_dir.exists() {
            fs::create_dir_all(file_dir).map_err(|e| FileStoreError::CreationError(Box::new(e)))?;
        }

        let file_name = file_dir.join("file");

        if let Err(e) = match File::options()
            .read(true)
            .write(true)
            .create_new(true)
            .open(&file_name)
        {
            Ok(file) => file.set_len(*file_info.length()).map_err(|e| e.into()),
            Err(e) => Err(e.into()),
        } {
            return Err(FileStoreError::CreationError(e));
        };

        let Some(file_name) = file_name.as_path().to_str() else {
            return Err(FileStoreError::CreationError(Box::new(
                std::io::Error::from(ErrorKind::InvalidInput), // ErrorKind::InvalidFilename
            )))
        };

        let file_info = file_info.mark_as_created(file_name);
        
        if let Err(e) = match File::options()
            .read(true)
            .write(true)
            .create_new(true)
            .open(file_dir.join("info").with_extension("json"))
        {
            Ok(info) => {
                serde_json::to_writer(info, &file_info).map_err(|e| e.into())
            }
            Err(e) => Err(e.into()),
        } {
            return Err(FileStoreError::CreationError(e));
        };

        Ok(file_info)

    }

    async fn patch_file(
        &self,
        file_id: &str,
        offset: u64,
        data: &mut [u8],
    ) -> Result<PatchOption, FileStoreError> {
        let file_path = format!("{}/{}", self.root_path, file_id);
        println!("When patching file, file_path: {}", file_path);

        let mut file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(file_path)
            .map_err(|e| FileStoreError::ReadError(Box::new(e)))?;

        file.seek(SeekFrom::Start(offset))
            .map_err(|e| FileStoreError::ReadError(Box::new(e)))?;

        file.write_all(data)
            .map_err(|e| FileStoreError::ReadError(Box::new(e)))?;

        let metadata = file.metadata().map_err(|e| FileStoreError::ReadError(Box::new(e)))?;

        // also, remember to check for the last part so if it's the last one, we can start the merge process

        Ok(PatchOption::Patched(metadata.len()))
    }

    async fn delete_file(
        &self,
        file_id: &str,
    ) -> Result<(), FileStoreError> {
        let file_path = format!("{}/{}", self.root_path, file_id);
        println!("When deleting file, file_path: {}", file_path);

        let file = std::fs::remove_file(file_path).map_err(|e| FileStoreError::TerminationError(Box::new(e)))?;

        Ok(())
    }        

    async fn get_file_info(
        &self,
        file_id: &str
    ) -> Result<(u64, Option<String>), FileStoreError> {
        let file_path = format!("{}/{}", self.root_path, file_id);
        println!("When getting file info, file_path: {}", file_path);

        let file = std::fs::File::open(file_path).map_err(|e| FileStoreError::ReadError(Box::new(e)))?;

        let metadata = file.metadata().map_err(|e| FileStoreError::ReadError(Box::new(e)))?;


        Ok((metadata.len(), None))
    }
}