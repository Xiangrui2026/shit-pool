use std::{error::Error, sync::Arc, time::Duration};

use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, State},
    http::StatusCode,
    routing::post,
};
use lettre::{
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
    message::{Mailbox, header::ContentType},
    transport::smtp::authentication::Credentials,
};
use serde::Deserialize;
use tokio::{sync::Semaphore, time::timeout};

use super::error::ApiError;
use crate::config::optional_var;

const MAX_MESSAGE_LENGTH: usize = 3000;
const DELIVERY_TIMEOUT: Duration = Duration::from_secs(12);

struct Mailer {
    transport: AsyncSmtpTransport<Tokio1Executor>,
    sender: Mailbox,
    recipient: Mailbox,
    slots: Semaphore,
}

impl Mailer {
    fn from_env() -> Result<Option<Self>, Box<dyn Error>> {
        let Some(host) = optional_var("SHIT_POOL_SMTP_HOST")? else {
            return Ok(None);
        };
        let user = required("SHIT_POOL_SMTP_USER")?;
        let password = required("SHIT_POOL_SMTP_PASSWORD")?;
        let sender = user.parse().map_err(|_| "Invalid SHIT_POOL_SMTP_USER")?;
        let recipient = required("SHIT_POOL_FEEDBACK_TO")?
            .parse()
            .map_err(|_| "Invalid SHIT_POOL_FEEDBACK_TO")?;
        let port = optional_var("SHIT_POOL_SMTP_PORT")?
            .as_deref()
            .unwrap_or("465")
            .parse::<u16>()
            .map_err(|_| "Invalid SHIT_POOL_SMTP_PORT")?;
        let transport = AsyncSmtpTransport::<Tokio1Executor>::relay(&host)?
            .port(port)
            .credentials(Credentials::new(user, password))
            .timeout(Some(DELIVERY_TIMEOUT))
            .build();
        Ok(Some(Self {
            transport,
            sender,
            recipient,
            slots: Semaphore::new(2),
        }))
    }
}

fn required(key: &str) -> Result<String, Box<dyn Error>> {
    optional_var(key)?.ok_or_else(|| format!("Missing {key} for feedback delivery").into())
}

pub(crate) fn routes() -> Result<Router, Box<dyn Error>> {
    Ok(with_mailer(Mailer::from_env()?))
}

fn with_mailer(mailer: Option<Mailer>) -> Router {
    Router::new()
        .route("/api/feedback", post(submit))
        .layer(DefaultBodyLimit::max(24 * 1024))
        .with_state(mailer.map(Arc::new))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Feedback {
    message: String,
}

async fn submit(
    State(mailer): State<Option<Arc<Mailer>>>,
    Json(feedback): Json<Feedback>,
) -> Result<StatusCode, ApiError> {
    let message = feedback.message.trim();
    if message.is_empty() || message.chars().count() > MAX_MESSAGE_LENGTH || message.contains('\0')
    {
        return Err(ApiError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "invalid_feedback",
        ));
    }
    let mailer = mailer
        .ok_or_else(|| ApiError::new(StatusCode::SERVICE_UNAVAILABLE, "feedback_not_configured"))?;
    let _permit = mailer
        .slots
        .try_acquire()
        .map_err(|_| ApiError::new(StatusCode::TOO_MANY_REQUESTS, "feedback_busy"))?;
    let email = Message::builder()
        .from(mailer.sender.clone())
        .to(mailer.recipient.clone())
        .subject("SHIT-Pool 石库网站反馈")
        .header(ContentType::TEXT_PLAIN)
        .body(message.to_owned())
        .map_err(|_| ApiError::new(StatusCode::UNPROCESSABLE_ENTITY, "invalid_feedback"))?;
    match timeout(DELIVERY_TIMEOUT, mailer.transport.send(email)).await {
        Ok(Ok(_)) => Ok(StatusCode::NO_CONTENT),
        Ok(Err(_)) => {
            eprintln!("Feedback SMTP delivery failed");
            Err(ApiError::new(
                StatusCode::BAD_GATEWAY,
                "feedback_delivery_failed",
            ))
        }
        Err(_) => Err(ApiError::new(
            StatusCode::GATEWAY_TIMEOUT,
            "feedback_delivery_timeout",
        )),
    }
}

#[cfg(test)]
mod tests {
    use axum::{body::Body, http::Request};
    use tokio::{
        io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
        net::TcpListener,
    };
    use tower::ServiceExt;

    use super::*;

    #[tokio::test]
    async fn feedback_validates_without_an_admin_key() {
        let app = with_mailer(None);
        for (body, status) in [
            (r#"{"message":"  "}"#, StatusCode::UNPROCESSABLE_ENTITY),
            (
                r#"{"message":"Valid feedback"}"#,
                StatusCode::SERVICE_UNAVAILABLE,
            ),
            (
                r#"{"message":"hi","to":"other@example.org"}"#,
                StatusCode::UNPROCESSABLE_ENTITY,
            ),
        ] {
            let response = app
                .clone()
                .oneshot(
                    Request::post("/api/feedback")
                        .header("Content-Type", "application/json")
                        .body(Body::from(body))
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(response.status(), status);
        }
    }

    #[tokio::test]
    async fn feedback_is_delivered_as_plain_text_to_the_configured_recipient() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let transport = AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous("127.0.0.1")
            .port(listener.local_addr().unwrap().port())
            .build();
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let (read, mut write) = stream.split();
            let mut lines = BufReader::new(read).lines();

            write.write_all(b"220 test ESMTP\r\n").await.unwrap();
            assert!(
                lines
                    .next_line()
                    .await
                    .unwrap()
                    .unwrap()
                    .starts_with("EHLO ")
            );
            write.write_all(b"250 test\r\n").await.unwrap();
            assert_eq!(
                lines.next_line().await.unwrap().unwrap(),
                "MAIL FROM:<feedback@example.org>"
            );
            write.write_all(b"250 OK\r\n").await.unwrap();
            assert_eq!(
                lines.next_line().await.unwrap().unwrap(),
                "RCPT TO:<owner@example.org>"
            );
            write.write_all(b"250 OK\r\n").await.unwrap();
            assert_eq!(lines.next_line().await.unwrap().unwrap(), "DATA");
            write.write_all(b"354 End with a dot\r\n").await.unwrap();
            let mut email = Vec::new();
            loop {
                let line = lines.next_line().await.unwrap().unwrap();
                if line == "." {
                    break;
                }
                email.push(line);
            }
            write.write_all(b"250 Accepted\r\n").await.unwrap();
            assert_eq!(lines.next_line().await.unwrap().unwrap(), "QUIT");
            write.write_all(b"221 Bye\r\n").await.unwrap();
            assert!(
                email
                    .iter()
                    .any(|line| line.starts_with("Content-Type: text/plain"))
            );
            assert_eq!(
                email.last().map(String::as_str),
                Some("Feedback for question 42")
            );
        });
        let response = with_mailer(Some(Mailer {
            transport,
            sender: "feedback@example.org".parse().unwrap(),
            recipient: "owner@example.org".parse().unwrap(),
            slots: Semaphore::new(2),
        }))
        .oneshot(
            Request::post("/api/feedback")
                .header("Content-Type", "application/json")
                .body(Body::from(r#"{"message":"Feedback for question 42"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
        assert_eq!(response.status(), StatusCode::NO_CONTENT);
        timeout(Duration::from_secs(2), server)
            .await
            .unwrap()
            .unwrap();
    }
}
