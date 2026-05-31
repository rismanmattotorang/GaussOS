//! Utility functions

/// API client for communicating with the GaussTwin server
pub mod api {
    use anyhow::Result;
    use serde::{de::DeserializeOwned, Serialize};

    /// API client
    pub struct ApiClient {
        base_url: String,
        client: reqwest::Client,
    }

    impl ApiClient {
        /// Create a new API client
        pub fn new(base_url: &str) -> Self {
            Self {
                base_url: base_url.to_string(),
                client: reqwest::Client::new(),
            }
        }

        /// Make a GET request
        pub async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
            let url = format!("{}{}", self.base_url, path);
            let response = self.client.get(&url).send().await?;
            let data = response.json().await?;
            Ok(data)
        }

        /// Make a POST request
        pub async fn post<T: Serialize, R: DeserializeOwned>(&self, path: &str, body: &T) -> Result<R> {
            let url = format!("{}{}", self.base_url, path);
            let response = self.client.post(&url).json(body).send().await?;
            let data = response.json().await?;
            Ok(data)
        }

        /// Make a PUT request
        pub async fn put<T: Serialize, R: DeserializeOwned>(&self, path: &str, body: &T) -> Result<R> {
            let url = format!("{}{}", self.base_url, path);
            let response = self.client.put(&url).json(body).send().await?;
            let data = response.json().await?;
            Ok(data)
        }

        /// Make a DELETE request
        pub async fn delete(&self, path: &str) -> Result<()> {
            let url = format!("{}{}", self.base_url, path);
            self.client.delete(&url).send().await?;
            Ok(())
        }
    }
}
