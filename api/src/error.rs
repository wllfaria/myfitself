use axum::Json;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use derive_more::{Display, Error, From};
use serde::Serialize;

use crate::services::clerk::ClerkError;

#[derive(Debug, Error, Display, From)]
pub enum AppError {
    #[display("Something went wrong: {_0}")]
    #[error(ignore)]
    ServerError(String),

    #[from]
    Unauthorized(ClerkError),
}

#[derive(Serialize)]
struct ErrorBody {
    ok: bool,
    message: String,
    status: u16,
    #[serde(rename = "statusText")]
    status_text: String,
}

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        let code = self.error_code();

        let message = Json(ErrorBody {
            ok: false,
            message: self.to_string(),
            status: code.as_u16(),
            status_text: code
                .canonical_reason()
                .expect("canonical reason must be defined")
                .to_string(),
        });

        (code, message).into_response()
    }
}

impl AppError {
    fn error_code(&self) -> StatusCode {
        match self {
            AppError::ServerError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::Unauthorized(_) => StatusCode::UNAUTHORIZED,
        }
    }
}

impl From<sqlx::Error> for AppError {
    fn from(err: sqlx::Error) -> Self {
        AppError::ServerError(err.to_string())
    }
}
