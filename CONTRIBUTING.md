# Contributing to RustIO Admin

Thanks for taking the time to look at the project.

RustIO Admin generates the foundation for operational software — authentication,
sessions, roles, recovery and audit. Because those paths are security-sensitive,
the project favours a narrow, reviewable surface over breadth. Small, well-scoped
changes are much easier to accept than large ones.

## Before you start

For anything beyond a typo or a docs fix, please open an issue first and describe
the problem you are trying to solve. That avoids spending your time on a change
that does not fit the direction described in [`MANIFESTO.md`](./MANIFESTO.md).

## Toolchain

- The workspace targets Rust **1.94**.
- `rustio-admin-cli` keeps a lower compile floor of **1.85** so that
  `cargo install rustio-admin-cli` stays cheap to install. CI enforces this with a
  dedicated `cargo check -p rustio-admin-cli --no-default-features` job.

If you add a dependency to the CLI's lightweight (default) feature set, check that
it does not raise that floor.

## Local checks

CI runs with `RUSTFLAGS: -D warnings`, so warnings fail the build. Run the same
checks locally before opening a pull request:

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo build --workspace --all-targets
cargo test --workspace --all-targets
```

And the CLI compile floor:

```bash
cargo check -p rustio-admin-cli --no-default-features
```

## Workspace layout

```
crates/rustio-admin          the framework
crates/rustio-admin-macros   derive macros
crates/rustio-admin-cli      command-line tool
crates/rustio-admin-assets   bundled static assets
crates/rio-theme             theme crate
```

The projects under `examples/` are **standalone workspaces** with their own
`Cargo.lock`, so they are not built by a workspace-wide `cargo build`. Build them
from inside their own directory when you change something they depend on.

## Pull requests

- Keep the change focused; unrelated refactors make review much harder.
- Add a test when you fix a bug, so the behaviour stays fixed.
- Update [`CHANGELOG.md`](./CHANGELOG.md) for user-visible changes.
- Update the docs in [`docs/`](./docs/) when behaviour or contracts change — the
  repository docs are the source of truth when they disagree with the docs site.

## Security-sensitive changes

Changes touching authentication, session handling, password or token storage,
authorization checks, recovery flows or audit events get a closer review. Please
explain in the pull request what the change means for:

- who is allowed to perform an action,
- how that authority is granted or revoked,
- what happens to sessions that are already active,
- what is written to the audit trail.

If you believe you have found a security vulnerability, please **do not** open a
public issue. Email <abdulwahed.sweden@gmail.com> instead.

## License

By contributing, you agree that your contributions are licensed under the
project's [MIT license](LICENSE).
