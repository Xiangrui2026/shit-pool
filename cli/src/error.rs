use serde::Serialize;

pub(crate) type Result<T> = std::result::Result<T, CliError>;

#[derive(Debug, Serialize)]
pub(crate) struct CliError {
    error: &'static str,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    status: Option<u16>,
    #[serde(skip_serializing)]
    exit_code: u8,
}

impl CliError {
    pub(crate) fn usage(message: impl Into<String>) -> Self {
        Self::new("invalid_input", message, 2)
    }

    pub(crate) fn auth(message: &'static str) -> Self {
        Self::new("authentication_required", message, 3)
    }

    pub(crate) fn io(message: impl Into<String>) -> Self {
        Self::new("local_io", message, 6)
    }

    pub(crate) fn output(_: std::io::Error) -> Self {
        Self::io("Could not write stdout.")
    }

    pub(crate) fn invalid_response(message: &'static str) -> Self {
        Self::new("invalid_response", message, 5)
    }

    pub(crate) fn transport(error: reqwest::Error) -> Self {
        if error.is_timeout() {
            Self::new("timeout", "The request timed out.", 5)
        } else if error.is_decode() {
            Self::new(
                "invalid_response",
                "The API response did not match its JSON contract.",
                5,
            )
        } else {
            Self::new(
                "request_failed",
                "Could not complete the request to SHIT-Pool.",
                5,
            )
        }
    }

    pub(crate) fn response(status: u16, code: Option<&str>) -> Self {
        let (kind, exit_code) = match status {
            400 | 413 | 415 | 422 => ("invalid_input", 2),
            401 | 403 => ("authentication_required", 3),
            404 => ("not_found", 4),
            _ => ("api_error", 5),
        };
        let code = code.filter(|value| {
            matches!(
                *value,
                "invalid_category"
                    | "invalid_source"
                    | "invalid_original_number"
                    | "invalid_stem"
                    | "invalid_answer"
                    | "invalid_solution_title"
                    | "invalid_solution_body"
                    | "invalid_notes"
                    | "invalid_tags"
                    | "invalid_id"
                    | "invalid_query"
                    | "invalid_page"
                    | "empty_patch"
                    | "question_not_found"
                    | "solution_not_found"
                    | "unauthorized"
                    | "database_not_configured"
                    | "database_unavailable"
                    | "database_error"
                    | "unsupported_image"
                    | "invalid_image"
                    | "image_too_large"
                    | "unsupported_image_reference"
                    | "missing_question_image"
                    | "archive_too_large"
                    | "image_storage_error"
            )
        });
        let mut error = Self::new(
            kind,
            code.unwrap_or("The API rejected the request."),
            exit_code,
        );
        error.status = Some(status);
        error
    }

    pub(crate) fn exit_code(&self) -> u8 {
        self.exit_code
    }

    fn new(error: &'static str, message: impl Into<String>, exit_code: u8) -> Self {
        Self {
            error,
            message: message.into(),
            status: None,
            exit_code,
        }
    }
}
