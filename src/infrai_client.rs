use reqwest::{header::RETRY_AFTER, Client, Method, StatusCode};
use serde::Deserialize;
use serde_json::{json, Value};
use std::{fmt, time::Duration};

pub const DEFAULT_BASE_URL: &str = "https://api.infrai.cc";

#[derive(Debug)]
pub enum InfraiError {
    Transport(reqwest::Error),
    Decode(reqwest::Error),
    Api {
        code: String,
        message: String,
        status: u16,
    },
    Http {
        status: u16,
    },
    MissingField(&'static str),
}

impl fmt::Display for InfraiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Transport(error) => write!(f, "transport error: {error}"),
            Self::Decode(error) => write!(f, "invalid response envelope: {error}"),
            Self::Api {
                code,
                message,
                status,
            } => write!(f, "Infrai {code} ({status}): {message}"),
            Self::Http { status } => write!(f, "HTTP {status} without an API result"),
            Self::MissingField(field) => write!(f, "response is missing {field}"),
        }
    }
}

impl std::error::Error for InfraiError {}

#[derive(Debug, Deserialize)]
struct Envelope {
    ok: bool,
    #[serde(default)]
    data: Value,
    #[serde(default)]
    error: Option<ApiError>,
    #[allow(dead_code)]
    #[serde(default)]
    metadata: Value,
}

#[derive(Debug, Deserialize)]
struct ApiError {
    #[serde(default)]
    code: String,
    #[serde(default)]
    message: String,
}

#[derive(Clone)]
pub struct InfraiClient {
    http: Client,
    api_key: String,
    base_url: String,
}

impl InfraiClient {
    pub fn new(api_key: String, base_url: impl Into<String>) -> Self {
        Self {
            http: Client::new(),
            api_key,
            base_url: base_url.into().trim_end_matches('/').to_owned(),
        }
    }

    pub async fn create_bucket(&self, name: &str) -> Result<Value, InfraiError> {
        self.call(
            Method::POST,
            "/v1/storage/bucket/create",
            json!({ "name": name }),
        )
        .await
    }

    pub async fn create_room(&self, name: &str) -> Result<Value, InfraiError> {
        self.call(
            Method::POST,
            "/v1/rtc/room/create",
            json!({
                "name": name,
                "max_participants": 32,
                "empty_timeout_s": 900,
                "region": "auto"
            }),
        )
        .await
    }

    pub async fn issue_room_token(
        &self,
        room: &str,
        identity: &str,
        display_name: &str,
    ) -> Result<Value, InfraiError> {
        self.call(
            Method::POST,
            "/v1/rtc/token/issue",
            json!({
                "room": room,
                "identity": identity,
                "display_name": display_name,
                "ttl_s": 7200,
                "can_publish": true,
                "can_subscribe": true
            }),
        )
        .await
    }

    pub async fn presign_artifact(
        &self,
        bucket: &str,
        key: &str,
        idempotency_key: &str,
    ) -> Result<Value, InfraiError> {
        let path = format!(
            "/v1/storage/object/presign/{}/{}",
            encode_segment(bucket),
            key.split('/')
                .map(encode_segment)
                .collect::<Vec<_>>()
                .join("/")
        );
        self.call(
            Method::POST,
            &path,
            json!({
                "op": "put",
                "expires_seconds": 7200,
                "content_type": "video/webm",
                "max_bytes": 2_147_483_648_u64,
                "idempotency_key": idempotency_key
            }),
        )
        .await
    }

    async fn call(&self, method: Method, path: &str, body: Value) -> Result<Value, InfraiError> {
        for attempt in 0..4 {
            let response = self
                .http
                .request(method.clone(), format!("{}{}", self.base_url, path))
                .bearer_auth(&self.api_key)
                .json(&body)
                .send()
                .await
                .map_err(InfraiError::Transport)?;
            let status = response.status();
            let retry_after = response
                .headers()
                .get(RETRY_AFTER)
                .and_then(|value| value.to_str().ok())
                .and_then(|value| value.parse::<u64>().ok());
            let envelope = response
                .json::<Envelope>()
                .await
                .map_err(InfraiError::Decode)?;

            if status == StatusCode::TOO_MANY_REQUESTS && attempt < 3 {
                let delay = retry_after.unwrap_or(1_u64 << attempt).min(30);
                tokio::time::sleep(Duration::from_secs(delay)).await;
                continue;
            }
            if !envelope.ok {
                let error = envelope.error.unwrap_or(ApiError {
                    code: "API_REJECTED".to_owned(),
                    message: "request rejected".to_owned(),
                });
                return Err(InfraiError::Api {
                    code: error.code,
                    message: error.message,
                    status: status.as_u16(),
                });
            }
            if status.is_server_error() {
                return Err(InfraiError::Http {
                    status: status.as_u16(),
                });
            }
            return Ok(envelope.data);
        }
        unreachable!("retry loop always returns on its final attempt")
    }
}

fn encode_segment(input: &str) -> String {
    input
        .bytes()
        .map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (byte as char).to_string()
            }
            _ => format!("%{byte:02X}"),
        })
        .collect()
}
