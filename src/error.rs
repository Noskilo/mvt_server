use std::{
    fmt::{self, Display},
    str::FromStr,
};

use actix_web::{error::ResponseError, http::StatusCode, HttpResponse};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub enum TransectErrorCode {
    DBError,
    InvalidInput,
}

#[derive(Debug, Serialize)]
pub struct TransectError {
    pub title: Option<String>,
    pub detail: Option<String>,
    pub code: Option<TransectErrorCode>,
}

impl Display for TransectError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl ResponseError for TransectError {
    fn status_code(&self) -> actix_web::http::StatusCode {
        match self.code {
            Some(TransectErrorCode::DBError) => StatusCode::INTERNAL_SERVER_ERROR,
            Some(TransectErrorCode::InvalidInput) => StatusCode::BAD_REQUEST,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> actix_web::HttpResponse<actix_web::body::BoxBody> {
        HttpResponse::build(self.status_code()).json(self)
    }
}

pub trait ParsableRequestParam<T: FromStr> {
    fn parsable(self, parameter_name: &str) -> Result<T, TransectError>;
}

impl<T: FromStr> ParsableRequestParam<T> for Option<&str> {
    fn parsable(self, parameter_name: &str) -> Result<T, TransectError> {
        let string_result = match self {
            Some(value) => Ok(value),
            None => Err(TransectError {
                title: Some("Missing Value".to_string()),
                detail: Some("A required input parameter is missing.".to_string()),
                code: Some(TransectErrorCode::InvalidInput),
            }),
        };

        match string_result {
            Ok(string_value) => string_value.parse().map_err(|_| TransectError {
                title: Some("Invalid Value Type".to_string()),
                detail: Some(format!(
                    "The value for '{parameter_name}' is of the incorrect type."
                )),
                code: Some(TransectErrorCode::InvalidInput),
            }),
            Err(transect_error) => Err(transect_error),
        }
    }
}
