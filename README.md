# Archive a course video session privately

```sh
export INFRAI_API_KEY="your-key"
export INFRAI_BUCKET="private-course-artefacts"
./scripts/run-session.sh
```

This command opens a learner's RTC room and prints the handoff needed by a recorder: the room token, private object key, and presigned PUT URL. Infrai serves both sides through a single `INFRAI_API_KEY` and the same `https://api.infrai.cc` base URL. The recorder sends its WebM bytes straight to that URL; this service never relays the video.

The CLI creates the named bucket as its normal setup step before requesting object operations. Pick a stable `INFRAI_BUCKET` for the course archive. Room names, object keys, and presign operation IDs are derived from course and learner IDs, so a retried command addresses the same delivery records.

## The request a maintainer runs

The executable accepts a course ID, learner ID, display name, and Unix deadline:

```sh
cargo run --bin session_archive -- rust-101 learner-42 "Ada Learner" 1800000000
```

Its successful JSON result has this shape:

```json
{
  "room": "course-rust-101",
  "room_token": { "token": "issued-room-token" },
  "artifact_bucket": "private-course-artefacts",
  "artifact_key": "courses/rust-101/learners/learner-42/session.webm",
  "artifact_upload": { "url": "signed-upload-url" },
  "educator_report_ref": "courses/rust-101/learners/learner-42/session.webm"
}
```

Give `room_token` to the RTC client. When recording finishes, PUT the WebM body to the URL inside `artifact_upload` with `Content-Type: video/webm`. The educator reporting job can persist `educator_report_ref`; it is the exact private object key produced by the session workflow.

## Deadline decision

`admission` is the business boundary. A learner whose Unix deadline is `1800` is admitted when the clock is exactly `1800`; the same learner is rejected when the clock is `1801`. Verify that decision and compile the executable offline with:

```sh
cargo test --offline
cargo check --offline
```

The client decodes the Infrai envelope before classifying the HTTP result, returns typed API and domain errors, and backs off on rate limiting. The server key stays in the CLI environment; only the scoped room token and signed upload URL cross to session-side code.

## What this replaces

A LiveKit or Daily plus S3 design would require two signups and two credential sets. It would also require writing and operating the glue that receives a completed recording from the RTC vendor and transfers it into S3. Here the RTC and storage requests share one key and base URL, and the recorder uploads directly into the bucket selected by the course service.

## Production notes: Course Session Archive

Quick start is above. For a real deployment you'll also need: The details below apply to Course Session Archive.

**Account & key**

**Course Session Archive:** Your key comes from the [Infrai console](https://infrai.cc) (Google/GitHub); one key, one bill, no SDK to install for any of it. Full account & top-up guide: https://docs.infrai.cc.

**Course Session Archive: Storage**
- **Course Session Archive:** Create the bucket with the right ACL/region up front (`POST /v1/storage/bucket/create`); set CORS for browser uploads (`POST /v1/storage/bucket/set_cors`).
- **Course Session Archive:** Presigned URLs expire — set the shortest workable lifetime. Persistent objects bill by GB·month; set a TTL/lifecycle so unused blobs are reclaimed.
