# Repository Guidelines

## Project Structure & Module Organization
- `main/` hosts the controller binary; `main/src/manager.rs` wires the reconciliation loop.
- `plugins/` contains backend adapters (AWS SSM, Secrets Manager, Vault, Pulumi); extend `plugins/src/lib.rs` when adding new sources.
- `crd/` holds the CRD generator; run `cargo run --bin crd` to refresh `config/crd.yaml`.
- `k8s/` and `config/` provide Kubernetes manifests; `config/default/` is shipped defaults, `config/local/` is for dev clusters.
- `e2e/` bundles docker-compose services and fixture manifests; `utils/` contains shared helpers reused across crates.

## Build, Test, and Development Commands
- `cargo build --workspace` compiles all crates.
- `make controller` runs the operator locally with your current kubeconfig.
- `make manifest` / `make manifest-clean` apply or tear down Kustomize bundles.
- `make mock-env` spins up LocalStack, Vault, and other dependencies for integration tests.
- `make upgrade-rust` keeps the toolchain on the stable channel.

## Coding Style & Naming Conventions
Rust 2021, 4-space indentation, and snake_case modules are the norm. Always run `cargo fmt --all` before committing and follow up with `cargo clippy --all-targets --all-features -D warnings` to keep lint debt out. Public APIs should live in the crate root’s `lib.rs`; keep backend-specific logic scoped inside `plugins/src/<backend>.rs`.

## Testing Guidelines
Use `make test` (wraps `cargo test --all-targets -- --nocapture`) to execute unit and integration suites with `TEST_ENV=true`. Seed mocked cloud services with `make init-test` after `make mock-env`. Place new fixtures in `e2e/` and keep test modules adjacent to the code they cover (`mod tests` inside the same file). Target high coverage on reconciliation paths and serializers, especially any new `Plugin` implementations.

## Commit & Pull Request Guidelines
Commits follow Conventional Commit prefixes (e.g. `fix:`, `feat:`, `chore(deps):`) as seen in history. Keep changes focused and include why the change is needed in the body. PRs must outline the problem, the solution, manual or automated test evidence, and reference related issues. Add screenshots or manifest diffs when Kubernetes behavior changes.

## Security & Configuration Tips
Never commit real secrets; use placeholders in YAML and document the required keys in `README.md`. When testing cloud backends, rely on LocalStack endpoints wired in `e2e/services.yaml` and keep AWS credentials in local environment variables or `kind`-scoped secrets. Review RBAC manifests in `config/` whenever introducing new controller capabilities.
