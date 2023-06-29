mod filesystem;
mod tus_service;
mod request_handlers;
// mod errors;

pub use filesystem::file_store::FileStore;
// pub use tus_service::Tus;
// pub use errors::TusError;

// TUS Headers for its protocol
use http::header::HeaderMap;
use http::header::HeaderValue;
use std::str::FromStr;

// TUS Headers for its protocol
#[derive(Debug)]
pub enum AxumTusHeaders {
    MaxSize,
    Extensions,
    Version,
    Resumable,
    UploadLength,
    UploadMetadata
}

impl AxumTusHeaders {
    pub fn name(&self) -> &'static str {
        match self {
            Self::MaxSize => "Tus-Max-Size",
            Self::Extensions => "Tus-Extension",
            Self::Version => "Tus-Version",
            Self::Resumable => "Tus-Resumable",
            Self::UploadLength => "Upload-Length",
            Self::UploadMetadata => "Upload-Metadata"
        }
    }
}

pub struct TusHeaderMap {
    max_size: Option<u64>,
    extensions: Option<Vec<String>>,
    version: Option<Vec<String>>,
    resumable: Option<String>,
    upload_length: Option<u64>,
    upload_metadata: Option<String>
}

impl Default for TusHeaderMap {
    fn default() -> Self {
        Self {
            max_size: None,
            extensions: None,
            version: None,
            resumable: None,
            upload_length: None,
            upload_metadata: None
        }
    }
}

impl TusHeaderMap {    
    pub fn from_headers(headers: &HeaderMap) -> Result<TusHeaderMap, Box<dyn std::error::Error>> {
        let mut tus_header_map = TusHeaderMap::default();

        if let Some(maxsize) = headers.get(AxumTusHeaders::MaxSize.name()) {
            let maxsize = u64::from_str(maxsize.to_str()?)?;
            tus_header_map.max_size = Some(maxsize);
        }

        if let Some(extensions) = headers.get(AxumTusHeaders::Extensions.name()) {
            let extensions = extensions.to_str()?
                .split(',')
                .map(String::from)
                .collect();
            tus_header_map.extensions = Some(extensions);
        }

        if let Some(version) = headers.get(AxumTusHeaders::Version.name()) {
            let version = version.to_str()?
                .split(',')
                .map(String::from)
                .collect();
            tus_header_map.version = Some(version);
        }

        if let Some(resumable) = headers.get(AxumTusHeaders::Resumable.name()) {
            let resumable = resumable.to_str()?.to_string();
            tus_header_map.resumable = Some(resumable);
        }

        if let Some(upload_length) = headers.get(AxumTusHeaders::UploadLength.name()) {
            let upload_length = u64::from_str(upload_length.to_str()?)?;
            tus_header_map.upload_length = Some(upload_length);
        }

        if let Some(upload_metadata) = headers.get(AxumTusHeaders::UploadMetadata.name()) {
            let upload_metadata = upload_metadata.to_str()?.to_string();
            tus_header_map.upload_metadata = Some(upload_metadata);
        }

        Ok(tus_header_map)
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
    }
}