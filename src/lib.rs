mod file_store;
mod tus_service;
// mod errors;

pub use file_store::FileStore;
// pub use tus_service::Tus;
// pub use errors::TusError;

// TUS Headers for its protocol
use http::header::HeaderMap;
use http::header::HeaderValue;
use std::str::FromStr;

// TUS Headers for its protocol
#[derive(Debug)]
pub enum AxumTusHeaders {
    MaxSize(u64),
    Extensions(Vec<String>),
    Version(Vec<String>),
    Resumable(String),
}

impl AxumTusHeaders {
    pub fn from_headers(headers: &HeaderMap) -> Result<Vec<AxumTusHeaders>, Box<dyn std::error::Error>> {
        let mut tus_headers = Vec::new();

        if let Some(max_size) = headers.get("Tus-Max-Size") {
            let max_size = u64::from_str(max_size.to_str()?)?;
            tus_headers.push(AxumTusHeaders::MaxSize(max_size));
        }

        if let Some(extensions) = headers.get("Tus-Extension") {
            let extensions = extensions.to_str()?
                .split(',')
                .map(String::from)
                .collect();
            tus_headers.push(AxumTusHeaders::Extensions(extensions));
        }

        if let Some(version) = headers.get("Tus-Version") {
            let version = version.to_str()?
                .split(',')
                .map(String::from)
                .collect();
            tus_headers.push(AxumTusHeaders::Version(version));
        }

        if let Some(resumable) = headers.get("Tus-Resumable") {
            let resumable = resumable.to_str()?.to_string();
            tus_headers.push(AxumTusHeaders::Resumable(resumable));
        }

        Ok(tus_headers)
    }

    pub fn apply(&self, headers: &mut HeaderMap) {
        match self {
            AxumTusHeaders::MaxSize(size) => {
                headers.insert("Tus-Max-Size", HeaderValue::from_str(&size.to_string()).unwrap());
            }
            AxumTusHeaders::Extensions(ext) => {
                headers.insert("Tus-Extension", HeaderValue::from_str(&ext.join(",")).unwrap());
            }
            AxumTusHeaders::Version(ver) => {
                headers.insert("Tus-Version", HeaderValue::from_str(&ver.join(",")).unwrap());
            }
            AxumTusHeaders::Resumable(ver) => {
                headers.insert("Tus-Resumable", HeaderValue::from_str(ver).unwrap());
            }
        }
    }
}
