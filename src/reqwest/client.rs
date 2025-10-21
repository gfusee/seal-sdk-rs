// Copyright 2025 Quentin Diebold
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//      http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::error::ReqwestError;
use crate::http_client::{HttpClient, PostResponse};
use async_trait::async_trait;
use http::{HeaderMap, HeaderName, HeaderValue};
use reqwest::Body;
use std::collections::HashMap;
use std::str::FromStr;

#[async_trait]
impl HttpClient for reqwest::Client {
    type PostError = ReqwestError;

    async fn post<S: ToString + Send + Sync>(
        &self,
        url: &str,
        headers: HashMap<String, String>,
        body: S,
    ) -> Result<PostResponse, Self::PostError> {
        let mut header_map = HeaderMap::new();

        for (key, value) in headers {
            header_map.insert(HeaderName::from_str(&key)?, HeaderValue::from_str(&value)?);
        }
        let response = self
            .post(url)
            .headers(header_map)
            .body(Body::from(body.to_string()))
            .send()
            .await?;

        let status = response.status().as_u16();
        let text = response.text().await?;

        let response = PostResponse { status, text };

        Ok(response)
    }
}
