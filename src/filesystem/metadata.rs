use base64::Engine as _;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, error::Error, fmt::Display};

/// A struct representing the metadata associated with an uploaded file.
#[derive(Default, Debug, Serialize, Deserialize)]
pub struct Metadata(HashMap<String, String>);

/// An error type representing errors that can occur while dealing with metadata.
#[derive(Debug, PartialEq)]
pub enum MetadataError {
    InvalidKey,
    DecodeError(String),
    InvalidMetadataFormat,
}

impl Error for MetadataError {}

impl Display for MetadataError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl Metadata {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn get_raw(&self, key: &str) -> Result<Vec<u8>, MetadataError> {
        let value = match self.0.get(key) {
            Some(v) => v,
            None => return Err(MetadataError::InvalidKey),
        };

        match base64::engine::general_purpose::STANDARD.decode(value) {
            Ok(decoded) => Ok(decoded),
            Err(e) => Err(MetadataError::DecodeError(e.to_string())),
        }
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }
}

impl TryFrom<&str> for Metadata {
    type Error = MetadataError;

    /// Attempts to parse a metadata string into a [`Metadata`] instance.
    ///
    /// The given string should follow the tus [`Upload-Metadata`](https://tus.io/protocols/resumable-upload.html#upload-metadata) definition .
    /// ```
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        if value.is_empty() {
            return Err(MetadataError::InvalidMetadataFormat);
        }

        let mut metadata = Metadata::new();

        for pair in value.split(',') {
            let pair = pair.trim();

            if pair.is_empty() {
                continue;
            }

            let parts: Vec<&str> = pair.split(' ').map(|v| v.trim()).collect();

            if parts.is_empty() || parts.len() > 2 {
                return Err(MetadataError::InvalidMetadataFormat);
            }

            if parts[0].is_empty() {
                return Err(MetadataError::InvalidKey);
            }

            if let (Some(key), value) = (parts.get(0), parts.get(1)) {
                let value = match value {
                    Some(v) => v.to_string(),
                    None => String::default(),
                };

                metadata.0.insert(key.to_string(), value);
            }
        }

        Ok(metadata)
    }
}