use crate::api_result::{ApiPayload, ApiResponse, ApiResponseExt, ApiResult};
use crate::authorization::Authorization;
use crate::error::Result;
use crate::utils::JsonStream;
use futures::prelude::*;
use reqwest::header::AUTHORIZATION;
use reqwest::{Client, IntoUrl, Method, Proxy, Url};
use serde::de::DeserializeOwned;
use std::sync::Arc;

#[derive(Debug)]
pub struct TwitterApi<A> {
    pub client: Client,
    pub base_url: Url,
    pub auth: Arc<A>,
}

impl<A> TwitterApi<A>
where
    A: Authorization,
{
    pub fn new(auth: A) -> Self {
        let mut build = Client::builder().pool_max_idle_per_host(0);
        if cfg!(debug_assertions) {
            build = build
                .proxy(Proxy::http("http://127.0.0.1:1087").unwrap())
                .proxy(Proxy::https("http://127.0.0.1:1087").unwrap());
        }
        Self {
            client: build.build().unwrap(),
            base_url: Url::parse("https://api.twitter.com/2/").unwrap(),
            auth: Arc::new(auth),
        }
    }

    pub fn auth(&self) -> &A {
        &self.auth
    }

    pub(crate) fn url(&self, url: impl AsRef<str>) -> Result<Url> {
        Ok(self.base_url.join(url.as_ref())?)
    }

    pub(crate) fn request(&self, method: Method, url: impl IntoUrl) -> reqwest::RequestBuilder {
        println!("{} {}", method.as_str(), url.as_str());
        self.client.request(method, url)
    }

    pub(crate) async fn send<T: DeserializeOwned, M: DeserializeOwned>(
        &self,
        req: reqwest::RequestBuilder,
    ) -> ApiResult<A, T, M> {
        let mut req = req.build()?;
        let authorization = self.auth.header(&req).await?;
        let _ = req.headers_mut().insert(AUTHORIZATION, authorization);
        let url = req.url().clone();
        let response = self
            .client
            .execute(req)
            .await?
            .api_error_for_status()
            .await?
            .json()
            .await?;
        Ok(ApiResponse::new(self, url, response))
    }

    pub(crate) async fn stream<T: DeserializeOwned, M: DeserializeOwned>(
        &self,
        req: reqwest::RequestBuilder,
    ) -> Result<impl Stream<Item = Result<ApiPayload<T, M>>>> {
        let mut req = req.build()?;
        let authorization = self.auth.header(&req).await?;
        let _ = req.headers_mut().insert(AUTHORIZATION, authorization);
        Ok(JsonStream::new(
            self.client
                .execute(req)
                .await?
                .api_error_for_status()
                .await?
                .bytes_stream(),
        ))
    }
}

impl<A> Clone for TwitterApi<A> {
    fn clone(&self) -> Self {
        Self {
            client: self.client.clone(),
            base_url: self.base_url.clone(),
            auth: self.auth.clone(),
        }
    }
}
