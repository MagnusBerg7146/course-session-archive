use course_session_archive::{
    course_session::{open_course_session, CourseDelivery},
    infrai_client::{InfraiClient, DEFAULT_BASE_URL},
};
use std::{env, error::Error, io};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 5 {
        eprintln!("usage: session-archive <course-id> <learner-id> <learner-name> <deadline-unix>");
        std::process::exit(2);
    }

    let api_key = env::var("INFRAI_API_KEY")
        .map_err(|_| io::Error::new(io::ErrorKind::NotFound, "INFRAI_API_KEY is required"))?;
    let bucket =
        env::var("INFRAI_BUCKET").unwrap_or_else(|_| "private-course-artefacts".to_owned());
    let deadline_unix = args[4].parse::<u64>()?;
    let client = InfraiClient::new(api_key, DEFAULT_BASE_URL);
    let handoff = open_course_session(
        &client,
        &bucket,
        CourseDelivery {
            course_id: args[1].clone(),
            learner_id: args[2].clone(),
            learner_name: args[3].clone(),
            deadline_unix,
        },
    )
    .await?;

    println!("{}", serde_json::to_string_pretty(&handoff)?);
    Ok(())
}
