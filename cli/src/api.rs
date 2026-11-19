use std::{env, time::Duration};

use reqwest::{
    Method, Url,
    blocking::{Client, RequestBuilder},
    header::{CONTENT_TYPE, HeaderValue},
    redirect::Policy,
};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::Value;

use crate::{
    args::{Search, TagMatch},
    error::{CliError, Result},
    payload::{NewQuestion, NewSolution, QuestionPatch, SolutionPatch},
};

pub(crate) struct Api {
    base: Url,
    client: Client,
}

#[derive(Deserialize, Serialize)]
pub(crate) struct Question {
    id: i32,
    category: String,
    source: String,
    #[serde(default)]
    original_number: Option<i32>,
    #[serde(default)]
    paper_source: Option<String>,
    pub stem: String,
    pub answer: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub solutions: Option<Vec<Solution>>,
    notes: String,
    tags: String,
}

#[derive(Deserialize, Serialize)]
pub(crate) struct Solution {
    id: i32,
    pub title: String,
    pub body: String,
    pub is_default: bool,
}

#[derive(Deserialize, Serialize)]
pub(crate) struct SolutionId {
    question_id: i32,
    id: i32,
}

#[derive(Deserialize, Serialize)]
pub(crate) struct SearchPage {
    items: Vec<Question>,
    page: u32,
    page_size: u32,
    total: u64,
}

#[derive(Deserialize, Serialize)]
pub(crate) struct QuestionId {
    id: i32,
}

#[derive(Deserialize, Serialize)]
pub(crate) struct UploadedImage {
    name: String,
    path: String,
    tex: String,
}

impl Api {
    pub(crate) fn new(base: &str) -> Result<Self> {
        let mut base =
            Url::parse(base).map_err(|_| CliError::usage("Provide a valid HTTP(S) API URL."))?;
        if !matches!(base.scheme(), "http" | "https")
            || base.host().is_none()
            || !base.username().is_empty()
            || base.password().is_some()
            || base.query().is_some()
            || base.fragment().is_some()
        {
            return Err(CliError::usage(
                "Use an HTTP(S) API URL without credentials, query or fragment.",
            ));
        }
        if !base.path().ends_with('/') {
            base.path_segments_mut()
                .expect("HTTP URL has path segments")
                .push("");
        }

        let mut builder = Client::builder()
            .timeout(Duration::from_secs(20))
            .connect_timeout(Duration::from_secs(5))
            .redirect(Policy::none())
            .retry(reqwest::retry::never())
            .user_agent(concat!("shit-pool/", env!("CARGO_PKG_VERSION")));
        if is_loopback(&base) {
            builder = builder.no_proxy();
        }
        let client = builder.build().map_err(CliError::transport)?;
        Ok(Self { base, client })
    }

    pub(crate) fn authorization(&self) -> Result<HeaderValue> {
        if self.base.scheme() != "https" && !is_loopback(&self.base) {
            return Err(CliError::usage(
                "Authenticated requests require HTTPS, except on loopback.",
            ));
        }
        let key = env::var("SHIT_POOL_API_KEY")
            .map_err(|_| CliError::auth("Set SHIT_POOL_API_KEY for write operations."))?;
        if key.is_empty() || !key.bytes().all(|byte| byte.is_ascii_graphic()) {
            return Err(CliError::auth(
                "SHIT_POOL_API_KEY must be non-empty and contain no whitespace.",
            ));
        }
        let mut value = HeaderValue::from_str(&format!("Bearer {key}"))
            .map_err(|_| CliError::auth("SHIT_POOL_API_KEY is not a valid header value."))?;
        value.set_sensitive(true);
        Ok(value)
    }

    pub(crate) fn search(&self, filters: &Search) -> Result<SearchPage> {
        let mut query = vec![("page", filters.page.to_string())];
        if let Some(keyword) = &filters.query {
            query.push(("q", keyword.clone()));
        }
        if let Some(category) = filters.category_id {
            query.push(("category_id", category.to_string()));
        }
        if !filters.tags.is_empty() {
            if filters
                .tags
                .iter()
                .any(|tag| tag.trim().is_empty() || tag.contains([',', '，']))
            {
                return Err(CliError::usage(
                    "Pass each non-empty tag separately with --tag.",
                ));
            }
            query.push(("tags", filters.tags.join(",")));
            query.push((
                "tag_match",
                match filters.tag_match {
                    TagMatch::Any => "any",
                    TagMatch::All => "all",
                }
                .into(),
            ));
        }
        self.send(self.request(Method::GET, "api/questions")?.query(&query))
    }

    pub(crate) fn get(&self, id: i32) -> Result<Question> {
        self.send(self.request(Method::GET, &format!("api/questions/{id}"))?)
    }

    pub(crate) fn upload_image(&self, data: Vec<u8>, auth: HeaderValue) -> Result<UploadedImage> {
        self.send(
            self.request(Method::POST, "api/images")?
                .header(reqwest::header::AUTHORIZATION, auth)
                .header(CONTENT_TYPE, "application/octet-stream")
                .body(data),
        )
    }

    pub(crate) fn export(&self, id: i32) -> Result<Vec<u8>> {
        use std::io::Read;

        const MAX_ZIP_BYTES: u64 = 33 * 1024 * 1024;
        let response =
            self.response(self.request(Method::GET, &format!("api/questions/{id}/export"))?)?;
        if response
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            != Some("application/zip")
            || response
                .content_length()
                .is_some_and(|size| size > MAX_ZIP_BYTES)
        {
            return Err(CliError::invalid_response(
                "Expected a ZIP archive up to 33 MiB.",
            ));
        }
        let mut archive = Vec::new();
        response
            .take(MAX_ZIP_BYTES + 1)
            .read_to_end(&mut archive)
            .map_err(|_| CliError::invalid_response("Could not read the ZIP response."))?;
        if archive.len() as u64 > MAX_ZIP_BYTES || !archive.starts_with(b"PK\x03\x04") {
            return Err(CliError::invalid_response(
                "Expected a ZIP archive up to 33 MiB.",
            ));
        }
        Ok(archive)
    }

    pub(crate) fn categories(&self) -> Result<Value> {
        self.send(self.request(Method::GET, "api/categories")?)
    }

    pub(crate) fn tags(&self) -> Result<Value> {
        self.send(self.request(Method::GET, "api/tags")?)
    }

    pub(crate) fn health(&self) -> Result<Value> {
        self.send(self.request(Method::GET, "api/health")?)
    }

    pub(crate) fn add(&self, question: &NewQuestion, auth: HeaderValue) -> Result<QuestionId> {
        self.send(
            self.request(Method::POST, "api/questions")?
                .header(reqwest::header::AUTHORIZATION, auth)
                .json(question),
        )
    }

    pub(crate) fn update(
        &self,
        id: i32,
        patch: &QuestionPatch,
        auth: HeaderValue,
    ) -> Result<QuestionId> {
        self.send(
            self.request(Method::PATCH, &format!("api/questions/{id}"))?
                .header(reqwest::header::AUTHORIZATION, auth)
                .json(patch),
        )
    }

    pub(crate) fn delete(&self, id: i32, auth: HeaderValue) -> Result<QuestionId> {
        self.send(
            self.request(Method::DELETE, &format!("api/questions/{id}"))?
                .header(reqwest::header::AUTHORIZATION, auth),
        )
    }

    pub(crate) fn add_solution(
        &self,
        question_id: i32,
        solution: &NewSolution,
        auth: HeaderValue,
    ) -> Result<SolutionId> {
        self.send(
            self.request(
                Method::POST,
                &format!("api/questions/{question_id}/solutions"),
            )?
            .header(reqwest::header::AUTHORIZATION, auth)
            .json(solution),
        )
    }

    pub(crate) fn update_solution(
        &self,
        question_id: i32,
        id: i32,
        patch: &SolutionPatch,
        auth: HeaderValue,
    ) -> Result<SolutionId> {
        self.send(
            self.request(
                Method::PATCH,
                &format!("api/questions/{question_id}/solutions/{id}"),
            )?
            .header(reqwest::header::AUTHORIZATION, auth)
            .json(patch),
        )
    }

    pub(crate) fn delete_solution(
        &self,
        question_id: i32,
        id: i32,
        auth: HeaderValue,
    ) -> Result<SolutionId> {
        self.send(
            self.request(
                Method::DELETE,
                &format!("api/questions/{question_id}/solutions/{id}"),
            )?
            .header(reqwest::header::AUTHORIZATION, auth),
        )
    }

    fn request(&self, method: Method, path: &str) -> Result<RequestBuilder> {
        let url = self
            .base
            .join(path)
            .map_err(|_| CliError::usage("Invalid API URL."))?;
        Ok(self.client.request(method, url))
    }

    fn send<T: DeserializeOwned>(&self, request: RequestBuilder) -> Result<T> {
        self.response(request)?.json().map_err(CliError::transport)
    }

    fn response(&self, request: RequestBuilder) -> Result<reqwest::blocking::Response> {
        let response = request.send().map_err(CliError::transport)?;
        let status = response.status();
        if !status.is_success() {
            #[derive(Deserialize)]
            struct ErrorPayload {
                error: String,
            }
            let body = response.json::<ErrorPayload>().ok();
            return Err(CliError::response(
                status.as_u16(),
                body.as_ref().map(|body| body.error.as_str()),
            ));
        }
        Ok(response)
    }
}

fn is_loopback(base: &Url) -> bool {
    match base.host() {
        Some(url::Host::Ipv4(address)) => address.is_loopback(),
        Some(url::Host::Ipv6(address)) => address.is_loopback(),
        Some(url::Host::Domain("localhost")) => true,
        _ => false,
    }
}
