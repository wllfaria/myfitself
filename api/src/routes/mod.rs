use axum::http::StatusCode;
use serde::Serialize;

pub mod auth;

#[derive(Serialize)]
pub struct HttpResponse<B> {
    ok: bool,
    status: u16,
    #[serde(rename = "statusText")]
    status_text: String,
    body: B,
}

impl<B> From<B> for HttpResponse<B> {
    fn from(body: B) -> Self {
        Self {
            ok: true,
            status: StatusCode::OK.as_u16(),
            status_text: StatusCode::OK.canonical_reason().unwrap().to_string(),
            body,
        }
    }
}
