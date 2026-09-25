use crate::infrai_client::{InfraiClient, InfraiError};
use serde::Serialize;
use serde_json::Value;
use std::{
    fmt,
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Debug)]
pub enum SessionError {
    DeadlinePassed { deadline_unix: u64, now_unix: u64 },
    Api(InfraiError),
}

impl fmt::Display for SessionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DeadlinePassed {
                deadline_unix,
                now_unix,
            } => {
                write!(
                    f,
                    "learner deadline {deadline_unix} has passed at {now_unix}"
                )
            }
            Self::Api(error) => error.fmt(f),
        }
    }
}

impl std::error::Error for SessionError {}

impl From<InfraiError> for SessionError {
    fn from(value: InfraiError) -> Self {
        Self::Api(value)
    }
}

#[derive(Debug)]
pub struct CourseDelivery {
    pub course_id: String,
    pub learner_id: String,
    pub learner_name: String,
    pub deadline_unix: u64,
}

#[derive(Debug, Serialize)]
pub struct SessionHandoff {
    pub room: String,
    pub room_token: Value,
    pub artifact_bucket: String,
    pub artifact_key: String,
    pub artifact_upload: Value,
    pub educator_report_ref: String,
}

pub fn admission(deadline_unix: u64, now_unix: u64) -> Result<(), SessionError> {
    if now_unix > deadline_unix {
        Err(SessionError::DeadlinePassed {
            deadline_unix,
            now_unix,
        })
    } else {
        Ok(())
    }
}

pub async fn open_course_session(
    client: &InfraiClient,
    bucket: &str,
    delivery: CourseDelivery,
) -> Result<SessionHandoff, SessionError> {
    let now_unix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    admission(delivery.deadline_unix, now_unix)?;

    let room = format!("course-{}", safe_id(&delivery.course_id));
    let artifact_key = format!(
        "courses/{}/learners/{}/session.webm",
        safe_id(&delivery.course_id),
        safe_id(&delivery.learner_id)
    );
    let operation_id = format!(
        "{}-{}-session-webm",
        safe_id(&delivery.course_id),
        safe_id(&delivery.learner_id)
    );

    client.create_bucket(bucket).await?;
    client.create_room(&room).await?;
    let room_token = client
        .issue_room_token(&room, &delivery.learner_id, &delivery.learner_name)
        .await?;
    let artifact_upload = client
        .presign_artifact(bucket, &artifact_key, &operation_id)
        .await?;

    Ok(SessionHandoff {
        educator_report_ref: artifact_key.clone(),
        room,
        room_token,
        artifact_bucket: bucket.to_owned(),
        artifact_key,
        artifact_upload,
    })
}

fn safe_id(input: &str) -> String {
    input
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' {
                ch
            } else {
                '-'
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn learner_is_admitted_at_deadline_but_rejected_after_it() {
        assert!(admission(1_800, 1_800).is_ok());
        assert!(matches!(
            admission(1_800, 1_801),
            Err(SessionError::DeadlinePassed {
                deadline_unix: 1_800,
                now_unix: 1_801
            })
        ));
    }
}
