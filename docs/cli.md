# CLI

## Install

```bash
cargo install rustio-admin-cli
```

## Commands

### `rustio-admin <command>`

| Command                                | What it does                                                |
|----------------------------------------|-------------------------------------------------------------|
| `new <name>`                           | Scaffold a new project (friendly alias for `startproject`)  |
| `startproject <name>`                  | Scaffold a new project at `./<name>`                        |
| `startapp <name>`                      | Add a model + migration to the current project              |
| `migrate apply`                        | Apply pending SQL migrations                                |
| `migrate status`                       | List applied and pending migrations                         |
| `user create`                          | Create a user in the auth tables                            |
| `group create <name>`                  | Create a group                                              |
| `perm grant-user`                      | Grant a permission to a user                                |
| `audit tail`                           | Tail the audit trail (`rustio_admin_actions`)               |
| `doctor`                               | Diagnose the local environment (DB, auth, MFA, secret key)  |
| `docs`                                 | Print where the framework's documentation lives             |
| `theme list`                           | List built-in `AdminTheme` palette presets                  |
| `override <template>`                  | Copy an embedded admin template to `./templates/`           |
| `reload`                               | Watch the source tree and re-run `cargo run` on change      |
| `test-init`                            | Generate a starter integration test at `tests/smoke.rs`     |
| `builder new <name>`                   | Bootstrap a Builder-managed project                         |
| `add model <Name>`                     | Record an `add_model` event in the Builder log              |
| `add field <Model> <name> <type>`      | Record an `add_field` event in the Builder log              |
| `plan`                                 | Show the diff `commit` would apply (read-only)              |
| `commit`                               | Apply the Builder plan atomically                           |
| `help`                                 | Show available commands                                     |

## Examples

```bash
# Scaffold a new project and run it (requires a local PostgreSQL)
rustio-admin new mysite
cd mysite
createdb mysite_dev
rustio-admin migrate apply
rustio-admin user create --email admin@mysite.local --role administrator
cargo run
# Sign in at http://127.0.0.1:8000/admin
```

```bash
# Add a model to the project, then re-apply migrations
rustio-admin startapp patient --field name:str --field date_of_birth:timestamp
rustio-admin migrate apply
```

## Flags

- `-h`, `--help`        Show help for any command
- `-V`, `--version`     Print version
- `--quiet`             Suppress progress spinners and status feedback
- `--no-progress`       Alias for `--quiet`
