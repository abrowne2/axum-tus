use axum::async_trait;
use bytes::Bytes;
use crate::TusError;

#[async_trait]
pub trait FileStore {
    async fn create_file(&self, file_id: &str, file_length: u64) -> Result<(), TusError>;
    async fn patch_file(&self, file_id: &str, offset: u64, data: &[u8]) -> Result<(), TusError>;   
    async fn delete_file(&self, file_id: &str) -> Result<(), TusError>;
    async fn get_file_info(&self, file_id: &str) -> Result<(u64, Option<String>), TusError>; // return file length and file type
}