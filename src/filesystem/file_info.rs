use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::filesystem::metadata::Metadata;
use std::{
    io::{Error, ErrorKind, Result},
    marker::PhantomData,
};


#[derive(Default, Debug)]
pub struct Building;

#[derive(Default, Debug)]
pub struct Built;

#[derive(Default, Debug)]
pub struct Created;

#[derive(Default, Debug)]
pub struct Completed;

#[derive(Default, Debug)]
pub struct Terminated;

/// A struct representing a file and its metadata during various stages of processing.
///
/// The struct has four possible states: [`Built`], [`Created`], [`Completed`] and [`Terminated`].
/// - [`Built`] - The file instances has been built and is ready to create information on disk.
/// - [`Created`] - The file information has been saved on disk.
/// - [`Completed`] - The file has been fully processed and is ready to be used.
/// - [`Terminated`] - The file has been terminated and is no longer saved on disk.
#[derive(Default, Debug, Serialize, Deserialize)]
pub struct FileInfo<State = Building> {
    id: String,
    file_name: String,
    length: u64,
    offset: u64,
    metadata: Option<Metadata>,

    #[serde(skip)]
    state: PhantomData<State>,
}

impl<State> FileInfo<State> {
    pub fn id(&self) -> &str {
        &self.id
    }
    
    pub fn length(&self) -> &u64 {
        &self.length
    }

    pub fn metadata(&self) -> &Option<Metadata> {
        &self.metadata
    }

    pub fn name(&self) -> &String {
        &self.file_name
    }

    // for use with the Upload-Metadata header
    pub fn metadata_str(&self) -> String {
        if let Some(metadata) = &self.metadata {
            serde_json::to_string(metadata).unwrap()
        } else {
            String::default()
        }
    }

    // for use for applying the header.
    pub fn length_str(&self) -> String {
        self.length.to_string()
    }
}

impl FileInfo<Building> {
    pub(super) fn new(length: u64) -> Self {
        Self {
            length,
            ..Default::default()
        }
    }

    pub(super) fn with_uuid(self) -> Self {
        self.with_raw_id(Uuid::new_v4().simple().to_string())
    }

    pub(super) fn with_raw_id(mut self, id: String) -> Self {
        self.id = id;
        self
    }

    pub(super) fn with_metadata(mut self, metadata: Metadata) -> Self {
        self.metadata = Some(metadata);
        // Attempt to get the filename from the passed in metadata.
        if let Some(file_name) = self.metadata.as_ref().unwrap().try_file_name() {
            self.file_name = file_name;
        }
        
        self
    }

    pub(super) fn build(self) -> FileInfo<Built> {
        FileInfo::<Built> {
            state: std::marker::PhantomData,
            id: self.id,
            length: self.length,
            offset: self.offset,
            metadata: self.metadata,
            file_name: self.file_name,
        }
    }
}

impl FileInfo<Built> {
    pub(super) fn mark_as_created(self, file_name: &str) -> FileInfo<Created> {
        FileInfo::<Created> {
            file_name: file_name.to_string(),
            state: std::marker::PhantomData,
            id: self.id,
            length: self.length,
            offset: self.offset,
            metadata: self.metadata,
        }
    }
}

impl FileInfo<Created> {
    pub fn offset(&self) -> &u64 {
        &self.offset
    }

    pub(super) fn set_offset(&mut self, offset: u64) -> Result<()> {
        if offset > self.length {
            return Err(Error::from(ErrorKind::OutOfMemory));
        }

        self.offset = offset;

        Ok(())
    }

    pub(crate) fn check_completion(self) -> Option<FileInfo<Completed>> {
        if self.offset != self.length {
            return None;
        }

        Some(FileInfo::<Completed> {
            state: std::marker::PhantomData,
            id: self.id,
            length: self.length,
            offset: self.offset,
            metadata: self.metadata,
            file_name: self.file_name,
        })
    }
}

impl FileInfo<Completed> {
    pub fn file_name(&self) -> &String {
        &self.file_name
    }
}

impl FileInfo<Terminated> {
    pub fn offset(&self) -> &u64 {
        &self.offset
    }

    pub fn file_name(&self) -> &String {
        &self.file_name
    }
}