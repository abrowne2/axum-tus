pub mod creation;
pub mod upload_handler;
pub mod file_info_handler;
pub mod info;

use serde::{Deserialize, Serialize};

// Generic AuthClaims trait for JWT claims in our requests
pub trait AuthClaims {
    fn get_user_id(&self) -> &str;
    fn get_expiration(&self) -> usize;
    fn get_subject(&self) -> &str;
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExampleJwtClaims {
    pub sub: String,
    // #[serde(deserialize_with = "deserialize_string_from_number")]
    pub user_id: String,
    pub exp: usize,
}

impl AuthClaims for ExampleJwtClaims {
    fn get_user_id(&self) -> &str {
        &self.user_id
    }

    fn get_expiration(&self) -> usize {
        self.exp
    }

    fn get_subject(&self) -> &str {
        &self.sub
    }
}