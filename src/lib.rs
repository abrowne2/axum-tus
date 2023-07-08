mod filesystem;
mod tus_service;
mod request_handlers;

pub use filesystem::file_store::FileStore;

use request_handlers::creation::*;
use request_handlers::file_info_handler::*;
use request_handlers::upload_handler::*;
use request_handlers::info::*;

// TUS Headers for its protocol
use http::header::HeaderMap;
use http::header::HeaderValue;
use std::str::FromStr;
use axum::routing::*;

pub fn setup_tus_routes(router: &mut axum::Router, file_store: FileStore) -> axum::Router {
    router
        .route("/", post(creation_handler).options(info_handler))
        .route("/:id", head(file_info_handler).patch(upload_handler))
        .layer(TusLayer {
            file_store
        });

    router
}

// TUS Headers for its protocol
#[derive(Debug)]
pub enum AxumTusHeaders {
    MaxSize,
    Extensions,
    Version,
    Resumable,
    UploadLength,
    UploadOffset,
    UploadMetadata
}

pub enum TusExtensions {
    Creation,
    Concatenation, //TODO implement concatenation extension (non-contiguous and parallel uploads.)
    Termination,
    CreationWithUpload,
    Checksum,
}

impl TusExtensions {
    pub fn name(&self) -> String {
        match self {
            Self::Creation => "creation".to_string(),
            _ => todo!("Not yet implemented"),
        }
    }
}

impl AxumTusHeaders {
    pub fn name(&self) -> &'static str {
        match self {
            Self::MaxSize => "Tus-Max-Size",
            Self::Extensions => "Tus-Extension",
            Self::Version => "Tus-Version",
            Self::Resumable => "Tus-Resumable",
            Self::UploadLength => "Upload-Length",
            Self::UploadOffset => "Upload-Offset",
            Self::UploadMetadata => "Upload-Metadata"
        }
    }
}

#[derive(Debug, Default)]
pub struct TusHeaderMap {
    max_size: Option<u64>,
    extensions: Option<Vec<String>>,
    version: Option<Vec<String>>,
    resumable: Option<String>,
    upload_length: Option<u64>,
    upload_metadata: Option<String>,
    upload_offset: Option<u64>
}

impl TusHeaderMap {    
    // TODO for now, defaults to 1.0.0 for resumable / version support.
    pub fn with_tus_version() -> Self {
        Self {
            resumable: Some("1.0.0".to_string()),
            version: Some(vec!["1.0.0".to_string()]),
            extensions: Some(vec![TusExtensions::Creation.name()]), 
            max_size: Some(300_000_000_000), //TODO Have configurable max size (300 gb default)
            ..Default::default()
        }
    }

    pub fn from_headers(headers: &HeaderMap) -> TusHeaderMap {
        let mut tus_header_map = TusHeaderMap::default();
    
        if let Some(maxsize) = headers.get(AxumTusHeaders::MaxSize.name()) {
            let maxsize = u64::from_str(maxsize.to_str().unwrap_or("0")).unwrap_or_default();
            tus_header_map.max_size = Some(maxsize);
        }
    
        if let Some(extensions) = headers.get(AxumTusHeaders::Extensions.name()) {
            let extensions = extensions
                .to_str()
                .unwrap_or("")
                .split(',')
                .map(String::from)
                .collect();
            tus_header_map.extensions = Some(extensions);
        }
    
        if let Some(version) = headers.get(AxumTusHeaders::Version.name()) {
            let version = version
                .to_str()
                .unwrap_or("")
                .split(',')
                .map(String::from)
                .collect();
            tus_header_map.version = Some(version);
        }
    
        if let Some(resumable) = headers.get(AxumTusHeaders::Resumable.name()) {
            let resumable = resumable.to_str().unwrap_or("").to_string();
            tus_header_map.resumable = Some(resumable);
        }
    
        if let Some(upload_length) = headers.get(AxumTusHeaders::UploadLength.name()) {
            let upload_length = u64::from_str(upload_length.to_str().unwrap_or("0")).unwrap_or_default();
            tus_header_map.upload_length = Some(upload_length);
        }
    
        if let Some(upload_offset) = headers.get(AxumTusHeaders::UploadOffset.name()) {
            let upload_offset = u64::from_str(upload_offset.to_str().unwrap_or("0")).unwrap_or_default();
            tus_header_map.upload_offset = Some(upload_offset);
        }
    
        if let Some(upload_metadata) = headers.get(AxumTusHeaders::UploadMetadata.name()) {
            let upload_metadata = upload_metadata.to_str().unwrap_or("").to_string();
            tus_header_map.upload_metadata = Some(upload_metadata);
        }
    
        tus_header_map
    }
    

    pub fn apply(&self, headers: &mut HeaderMap) {
        if let Some(max_size) = &self.max_size {
            headers.insert(AxumTusHeaders::MaxSize.name(), HeaderValue::from_str(&max_size.to_string()).unwrap());
        }

        if let Some(extensions) = &self.extensions {
            let extensions = extensions.join(",");
            headers.insert(AxumTusHeaders::Extensions.name(), HeaderValue::from_str(&extensions).unwrap());
        }

        if let Some(version) = &self.version {
            let version = version.join(",");
            headers.insert(AxumTusHeaders::Version.name(), HeaderValue::from_str(&version).unwrap());
        }

        if let Some(resumable) = &self.resumable {
            headers.insert(AxumTusHeaders::Resumable.name(), HeaderValue::from_str(&resumable).unwrap());
        }

        if let Some(upload_length) = &self.upload_length {
            headers.insert(AxumTusHeaders::UploadLength.name(), HeaderValue::from_str(&upload_length.to_string()).unwrap());
        }

        if let Some(upload_metadata) = &self.upload_metadata {
            headers.insert(AxumTusHeaders::UploadMetadata.name(), HeaderValue::from_str(&upload_metadata).unwrap());
        }

        if let Some(upload_offset) = &self.upload_offset {
            headers.insert(AxumTusHeaders::UploadOffset.name(), HeaderValue::from_str(&upload_offset.to_string()).unwrap());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_setup_routes() {
        // if running tests, make sure that your root path resolves to the local src directory.
        let local_file_store = filesystem::file_store::LocalFileStore::new("/Users/adambrowne/projects/axum-tus/src/root_test_path".to_string());

        let local_file_store = Arc::new(local_file_store);

        let mut axum_router = axum::Router::new();

        axum_router = setup_tus_routes(axum_router, local_file_store);
    }
}