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

#[async_trait]
pub trait FileStore: Send + Sync + Clone {
    async fn build_file(&self, length: u64, metadata: Option<&str>) -> Result<FileInfo<Built>, FileStoreError>;
    async fn create_file(&self, file_info: FileInfo<Built>) -> Result<FileInfo<Created>, FileStoreError>;
    async fn patch_file(&self, file_id: &str, offset: u64, data: &mut [u8]) -> Result<PatchOption, FileStoreError>;   
    async fn delete_file(&self, file_id: &str) -> Result<(), FileStoreError>;
    async fn get_file_info(&self, file_id: &str) -> Result<FileInfo<Created>, FileStoreError>; // return file length and file type
    async fn exists(&self, file_id: &str) -> bool;
}

#[derive(Debug)]
pub enum FileStoreError {
    CreationError(Box<dyn std::error::Error>),
    ReadError(Box<dyn std::error::Error>),
    TerminationError(Box<dyn std::error::Error>),
    Error
}

// NOTE: You can include an Arc<State> for additional logic at the time of the construction of the filestore.
#[derive(Clone)]
pub struct LocalFileStore {
    root_path: String,
    // state: Arc<State>
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
        let file_dir = std::path::Path::new(self.root_path.as_str()).join(file_id);

        let info_path = file_dir.join("info").with_extension("json");

        let file = match File::open(info_path) {
            Ok(file) => file,
            Err(e) => return Err(FileStoreError::ReadError(Box::new(e))),
        };

        let reader = BufReader::new(file);

        serde_json::from_reader(reader)   
            .map_err(|e| FileStoreError::ReadError(Box::new(e)))
    }
}

///
/// This LocalFileStore implementation is for testing purposes only.
/// It is not recommended to use this in production.
/// It's used for testing the tus protocol and the tus server, and uses the filesystem locally.
/// You should spin up your own.
/// 
#[async_trait]
impl FileStore for LocalFileStore {
    async fn build_file(
        &self,
        length: u64,
        metadata: Option<&str>,
    ) -> Result<FileInfo<Built>, FileStoreError> {
        let metadata = match metadata {
            Some(metadata) => match Metadata::try_from(metadata) {
                Ok(m) => m,
                Err(e) => return Err(FileStoreError::CreationError(Box::new(e))),
            },
            None => Metadata::default()
        };

        // note can also use with_raw_id(), which we may use for a custom filestore.
        let file_info = FileInfo::new(length)
            .with_uuid()
            .with_metadata(metadata)
            .build();
        
        Ok(file_info)
    }

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

        // NOTE do we have the filename from the metadata here?
        let file_name = file_dir.join(file_info.name());
        // NOTE this creates a new file; for our bucket filestore we will just create a new file in the bucket.
        // and then name the info.json (it should just be inside)
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
        let mut file: FileInfo<Created>  = self.read_file(file_id)?;

        if *file.offset() != offset {
            return Err(FileStoreError::ReadError(Box::new(
                std::io::Error::from(ErrorKind::InvalidInput),
            )));
        }

        let file_path = format!("{}/{}", self.root_path, file_id);
        println!("When patching file, file_path: {}", file_path);

        let file_dir = Path::new(&self.root_path).join(file_id);

        let mut file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(file_path)
            .map_err(|e| FileStoreError::ReadError(Box::new(e)))?;

        file.seek(SeekFrom::Start(offset))
            .map_err(|e| FileStoreError::ReadError(Box::new(e)))?;

        file.write_all(data)
            .map_err(|e| FileStoreError::ReadError(Box::new(e)))?;

        let file_info_path = file_dir.join("info").with_extension("json");

        let mut file_info = File::options().write(true).open(file_info_path).unwrap();

        serde_json::to_writer(&mut file_info, &file).unwrap();


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
    ) -> Result<FileInfo<Created>, FileStoreError> {
        self.read_file(file_id)
    }
}


#[cfg(test)]
mod tests {
    use base64::Engine;

    use super::*;

    #[derive(Clone, Copy, Debug)]
    enum FileStoreTestState {
        Built,
        Created,
        Patched,
        Completed,
        Terminated,
    }

    // creating separate test dirs because the tests are run in parallel...
    impl FileStoreTestState {
        pub fn name(&self) -> &str {
            match self {
                Self::Built => "_built",
                Self::Created => "_created",
                Self::Patched => "_patched",
                Self::Completed => "_completed",
                Self::Terminated => "_terminated",
            }
        }
    }

    fn cleanup_test_directory(test_state: FileStoreTestState) {
        // todo must be configurable or detected.
        let root_path = format!("/Users/adambrowne/projects/axum-tus/src/root_test_path{}", test_state.name());
        let info_path = format!("{}/{}", root_path, "test_local_file.info.json");

        let _ = std::fs::remove_file(info_path);
        let _ = std::fs::remove_dir_all(root_path);
    }

    async fn build_and_create_test_file(test_state: FileStoreTestState) -> Result<FileInfo<Created>, FileStoreError> {
        let root_path = format!("/Users/adambrowne/projects/axum-tus/src/root_test_path{}", test_state.name());
        let local_file_store = LocalFileStore::new(root_path);

        // 16 megabytes upload length
        let upload_length: u64 = 16361047;
        let metadata_file_string = base64::engine::general_purpose::STANDARD.encode("test_local_file.mov");        
        let metadata_string = format!("filename {},", metadata_file_string);

        let file_info_built = local_file_store.build_file(upload_length, Some(&metadata_string)).await;

        assert!(file_info_built.is_ok());

        // ensure metadata was properly parsed.
        if let Ok(file_info) = file_info_built {
            assert_eq!(file_info.length(), &upload_length);
            
            let metadata = file_info.metadata();
            match metadata {
                Some(metadata) => {
                    match metadata.get_raw("filename") {
                        Ok(raw) => {
                            let filename = String::from_utf8(raw).unwrap();
                            assert_eq!(filename, "test_local_file.mov");
                        },
                        Err(e) => {
                            println!("Error: {:?}", e);
                            panic!();
                        }
                    }
                },
                None => {
                    panic!();
                }
            }
            local_file_store.create_file(file_info).await
        } else {
            Err(FileStoreError::Error)
        }
    }

    async fn patch_byte_offset_of_file(file_store: &LocalFileStore, file_info: &FileInfo<Created>, offset: u64, data: &mut [u8]) -> Result<u64, FileStoreError> {
        let file_id = file_info.id();

        let final_offset: Result<u64, FileStoreError> = match local_file_store.patch_file(&file_id, offset, data).await {
            Ok(result) => {
                let final_offset = match result {
                    PatchOption::Patched(offset) => offset,
                    PatchOption::Completed(file_info) => {
                        // check auto terminate logic here.
    
                        *file_info.length()
                    }
                };
    
                Ok(final_offset)
            },
            Err(e) => {
                println!("Error patching file: {:?}", e);
                Err(FileStoreError::Error)
            }
        };

        final_offset
    }

    #[tokio::test]
    async fn test_local_build_file() {
        cleanup_test_directory(FileStoreTestState::Built);

        let _ = build_and_create_test_file(FileStoreTestState::Built).await;
    }

    #[tokio::test]
    async fn test_patching_file_bytes() {
        let test_state = FileStoreTestState::Patched;
        cleanup_test_directory(test_state);

        let file_info = build_and_create_test_file(test_state.clone()).await.unwrap();

        let root_path = format!("/Users/adambrowne/projects/axum-tus/src/root_test_path{}", test_state.clone().name());
        let local_file_store = LocalFileStore::new(root_path);

        // For this test we're just splitting the file in two.
        let all_file_data = std::fs::read("/Users/adambrowne/projects/axum-tus/src/filesystem/test_local_file.mov").unwrap();
        let midpoint = all_file_data.len() / 2;

        let mut first_half_bytes = all_file_data[..midpoint].to_vec();
        let mut second_half_bytes = all_file_data[midpoint..].to_vec();

        let first_offset = patch_byte_offset_of_file(&local_file_store, &file_info, 0, &mut first_half_bytes).await.unwrap();
        
        let second_offset = patch_byte_offset_of_file(&local_file_store, &file_info, first_offset, &mut second_half_bytes).await.unwrap();

        // proper upload length. should be completed here.
        assert_eq!(second_offset, 16361047);
    }
}