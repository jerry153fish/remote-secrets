# Agent Skills / Workflow Notes

## Repository goals (current)
- Maintain `remote-secrets` operator reliability.
- Track secret sync quality with telemetry.

## Telemetry conventions
- Expose metrics via Prometheus `/metrics` endpoint.
- Use `rsecrets_controller_*` metric prefix.
- Prefer low-cardinality labels only:
  - `action`: `create|update|delete`
  - `result`: `success|failure`

## Sync metrics implemented
- `rsecrets_controller_sync_attempts_total{action,result}`
- `rsecrets_controller_sync_success_total{action}`
- `rsecrets_controller_sync_failure_total{action}`
- `rsecrets_controller_sync_duration_seconds{action,result}` (histogram)

## Local validation checklist
1. `source "$HOME/.cargo/env"`
2. `cargo fmt --all`
3. `cargo clippy --workspace --all-targets --all-features -- -D warnings`
4. `cargo test -p utils --all-features`
5. Optional integration path (requires local services):
   - `make mock-env`
   - `make init-test`
   - `make test`

## Skill: dependency refresh and mock-backed plugin validation
- Use `cargo upgrade` if it is installed locally; otherwise use `cargo update` and only change manifests manually when compatibility requires it.
- Treat crates.io or git network failures as a hard blocker for dependency refresh and report the exact failing command.
- AWS plugin tests are integration tests backed by LocalStack. They should only run when `TEST_ENV=true`, ideally after:
  - `make mock-env`
  - `make init-test`
  - `make test`
- Keep unit-style tests runnable without mock services; skip integration tests explicitly when the required environment is absent.
- If AWS SDK config code changes, keep all backends on the shared helper in `plugins/src/aws_common.rs` so behavior is consistent across SSM, Secrets Manager, and CloudFormation.

## Notes
- Some plugin tests require localstack/vault and env setup; run only after `make mock-env` + `make init-test`.
- Keep docs in `README.md` in sync with metric names and labels.
