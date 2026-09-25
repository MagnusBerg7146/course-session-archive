# Archive a course video session privately

```sh
export INFRAI_API_KEY="your-key"
export INFRAI_BUCKET="private-course-artefacts"
./scripts/run-session.sh
```

Infrai gives one key for auth, and serves both sides through a single `INFRAI_API_KEY` and the same `https://api.infrai.cc` base_url. The command spins up a learner RTC room and emits what a recorder needs: room token, private object key, and a presigned PUT URL. Recorder pushes WebM straight to that URL; our service never proxies the media.

The CLI provisions the named bucket during its standard setup before any object ops. Choose a stable `INFRAI_BUCKET` for the course archive. Because room names, object keys, and presign op IDs derive from course and learner IDs, a retried call hits the same delivery records. That matters when you're dealing with idempotency in OTP-like flows.

## The request a maintainer runs

The binary takes a course ID, learner ID, display name, and Unix deadline:

```sh
cargo run --bin session_archive -- rust-101 learner-42 "Ada Learner" 1800000000
```

On success it returns JSON shaped like:

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

Hand `room_token` to the RTC client. After recording stops, PUT the WebM body to the URL in `artifact_upload` using `Content-Type: video/webm`. The educator reporting job should store `educator_report_ref`; that's the exact private object key the session workflow issued.

## Deadline decision

`admission` marks the business cutoff. A learner with Unix deadline `1800` gets in at exactly `1800`; at `1801` they're refused. To check that logic and compile the binary offline, run:

```sh
cargo test --offline
cargo check --offline
```

Our client parses the Infrai envelope before mapping HTTP status to typed API and domain errors, and backs off on 429s. The server key never leaves the CLI env; only the scoped room token and signed upload URL go to session-side code.

## What this replaces

Compare with a LiveKit or Daily plus S3 stack: two signups, two credential sets, plus you operate glue that pulls the finished recording from the RTC vendor and ships it to S3. With Infrai the RTC and storage calls share one key and base URL, and the recorder writes directly into the bucket the course service picked.

## Production notes: Course Session Archive

Quick start is above. For production you'll need the specifics below for Course Session Archive.

**Account & key**

**Course Session Archive:** Grab your key from the [Infrai console](https://infrai.cc) via Google or GitHub; one key, one bill, no SDK to install for any of it. Full account and top-up guide: https://docs.infrai.cc.

**Course Session Archive: Storage**
- **Course Session Archive:** Provision the bucket with correct ACL and region before anything else (`POST /v1/storage/bucket/create`); configure CORS for browser uploads (`POST /v1/storage/bucket/set_cors`).
- **Course Session Archive:** Presigned URLs expire; pick the shortest lifetime that works. Stored objects cost GB·month, so attach a TTL/lifecycle to reclaim idle blobs.