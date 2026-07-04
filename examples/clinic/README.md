# Clinic — the canonical RustIO Admin project

This is the reference shape every RustIO Admin project keeps for its
whole life: **one workspace, capabilities as crates**. A 5-model project
and a 500-model project have the *same* layout — that consistency is the
point. (For the *why*, read [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md).)

## What each folder is

```text
clinic-admin/
├── Cargo.toml          the workspace (lists the crates)
├── .env.example        copy to .env, set DATABASE_URL, then run
├── crates/
│   ├── clinic-core/    shared foundation: database, config, error, prelude
│   ├── patients/       capability — patients + their vitals
│   ├── scheduling/     capability — appointments
│   ├── billing/        capability — invoices
│   └── clinic-server/  the program that starts the system (the only main.rs)
├── migrations/         all database changes, numbered, never edited once applied
├── templates/          HTML overrides (optional) — no Rust here
├── static/             CSS / JS / images (optional) — no Rust here
└── docs/ARCHITECTURE.md  the layout explained in plain language
```

- **A crate = a business capability** (patients, scheduling, billing) —
  never one crate per table. Tables are modules *inside* a capability.
- **`clinic-core`** holds what more than one capability needs.
- **`clinic-server`** is the only crate that runs; it asks each capability
  to register its models, then serves the admin.
- **`templates/` and `static/`** live outside the crates so a frontend
  developer never opens a Rust file.

## Run it

```sh
createdb clinic_admin_dev
cp .env.example .env          # then edit DATABASE_URL
cargo run -p clinic-server    # migrations run automatically on boot
# open http://127.0.0.1:8000/admin
```

Create the first admin user with the framework CLI:

```sh
cargo install rustio-admin-cli   # provides the `rustio-admin` binary
rustio-admin user create --email admin@clinic.local --role administrator
```

## Add a capability

1. `cargo new --lib crates/<capability>` and depend on `rustio-admin` +
   `clinic-core`.
2. Add a model module with `#[derive(RustioAdmin)]` + `impl Model` +
   `impl ModelAdmin`, and a `pub fn register(admin: Admin) -> Admin`.
3. Add the crate to `clinic-server`'s `Cargo.toml` and one
   `register()` call in `main.rs`.
4. Add a numbered migration in `migrations/`.

That is the whole integration story — append a crate, append a
`register()` call, append a migration. Nothing existing is rewritten.
