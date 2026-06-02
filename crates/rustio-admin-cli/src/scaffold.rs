//! `rustio-admin startproject <name>` -- generate a fresh project skeleton
//! at `./<name>/`.
//!
//! Templates are baked into the binary via `include_str!` so the CLI
//! stays single-binary. Each template carries a `{{name}}` placeholder
//! that we substitute for the project name; everything else is
//! verbatim.

use std::fs;
use std::path::Path;

/// `(relative_target_path, template_body)` pairs. `target_path` is
/// relative to the new project's root and creates parent directories
/// on demand.
///
/// PR 1.5 doctrine (`DESIGN_ONBOARDING.md` §6): the default scaffold
/// ships NO domain model. `src/post.rs` and `migrations/0001_…`
/// moved into the `blog` preset so a `clinic` / `school` / `inventory`
/// / `custom` project stops feeling like a renamed blog. The
/// `templates/home.html` file (the first-boot homepage) and the
/// `migrations/.gitkeep` placeholder are now part of every project.
const PROJECT_TEMPLATES: &[(&str, &str)] = &[
    (
        "Cargo.toml",
        include_str!("../templates/project/Cargo.toml.tmpl"),
    ),
    (
        ".env.example",
        include_str!("../templates/project/.env.example"),
    ),
    (
        ".gitignore",
        include_str!("../templates/project/.gitignore"),
    ),
    (
        "README.md",
        include_str!("../templates/project/README.md.tmpl"),
    ),
    (
        "src/main.rs",
        include_str!("../templates/project/src/main.rs.tmpl"),
    ),
    (
        "templates/home.html",
        include_str!("../templates/project/templates/home.html"),
    ),
    (
        "migrations/.gitkeep",
        include_str!("../templates/project/migrations/.gitkeep"),
    ),
];

/// `blog` preset -- layered on top of `PROJECT_TEMPLATES`. The
/// `src/main.rs` slot is replaced wholesale by writing the preset
/// version *after* the minimal pass (`fs::write` overwrites
/// unconditionally), so the ordering
/// `PROJECT_TEMPLATES` → `BLOG_OVERRIDES` matters: keep
/// preset-owned filenames in the overrides slice and any new-only
/// files separate from them. New-only files (`src/post.rs`,
/// `src/comment.rs`, the two `create_*` migrations) are listed in
/// `BLOG_EXTRAS` to keep the rebuilt mental model from the
/// minimal scaffold honest.
const BLOG_OVERRIDES: &[(&str, &str)] = &[(
    "src/main.rs",
    include_str!("../templates/project_blog/src/main.rs.tmpl"),
)];

const BLOG_EXTRAS: &[(&str, &str)] = &[
    (
        "src/post.rs",
        include_str!("../templates/project_blog/src/post.rs.tmpl"),
    ),
    (
        "src/comment.rs",
        include_str!("../templates/project_blog/src/comment.rs.tmpl"),
    ),
    (
        "migrations/0001_create_posts.sql",
        include_str!("../templates/project_blog/migrations/0001_create_posts.sql"),
    ),
    (
        "migrations/0002_create_comments.sql",
        include_str!("../templates/project_blog/migrations/0002_create_comments.sql"),
    ),
];

/// `clinic` preset -- same layering as `blog`. Choosing the `clinic`
/// project type in the wizard produces a *real* clinic: `Patient` +
/// `Appointment` models, their migrations (seeded with example rows),
/// and a `src/main.rs` that already registers both. The point is that
/// the developer reaches a working, non-empty clinic admin without
/// hand-editing a single file (`DESIGN_ONBOARDING.md` §6).
const CLINIC_OVERRIDES: &[(&str, &str)] = &[(
    "src/main.rs",
    include_str!("../templates/project_clinic/src/main.rs.tmpl"),
)];

const CLINIC_EXTRAS: &[(&str, &str)] = &[
    (
        "src/patient.rs",
        include_str!("../templates/project_clinic/src/patient.rs.tmpl"),
    ),
    (
        "src/appointment.rs",
        include_str!("../templates/project_clinic/src/appointment.rs.tmpl"),
    ),
    (
        "migrations/0001_create_patients.sql",
        include_str!("../templates/project_clinic/migrations/0001_create_patients.sql"),
    ),
    (
        "migrations/0002_create_appointments.sql",
        include_str!("../templates/project_clinic/migrations/0002_create_appointments.sql"),
    ),
];

/// A slice of `(relative_target_path, template_body)` pairs — the
/// shape of every template table in this module.
type TemplateSet = &'static [(&'static str, &'static str)];

/// Select the `(overrides, extras)` template slices for a content
/// preset, or `None` for the neutral `minimal` scaffold. Keeps the
/// per-preset wiring in one place so `write_project_files` stays a
/// straight loop. `overrides` reuse a slot `PROJECT_TEMPLATES` already
/// wrote (not counted); `extras` are net-new files (counted).
fn preset_layers(preset: &str) -> Option<(TemplateSet, TemplateSet)> {
    match preset {
        "blog" => Some((BLOG_OVERRIDES, BLOG_EXTRAS)),
        "clinic" => Some((CLINIC_OVERRIDES, CLINIC_EXTRAS)),
        _ => None,
    }
}

/// Humanise the lower-case cargo crate name into a display title
/// for `Admin::app_name(...)` — splits on `-` / `_`, capitalises
/// each word, joins with a single space. `clinic` -> `Clinic`,
/// `my-clinic` -> `My Clinic`, `school_admin` -> `School Admin`.
/// Powers `{{name_title}}` substitution (Polish & Trust PR).
fn humanise_name(name: &str) -> String {
    name.split(['-', '_'])
        .filter(|w| !w.is_empty())
        .map(|w| {
            let mut cs = w.chars();
            match cs.next() {
                Some(c) => c.to_uppercase().collect::<String>() + cs.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Curated project-type identifiers from `DESIGN_ONBOARDING.md` §6
/// / the PR 1.2 wizard. Drives the `{{type_phrase}}` substitution
/// in `templates/home.html`. Unknown values (or the non-interactive
/// default) fall through to the `custom` phrasing -- neutral, never
/// blog-flavoured.
fn type_phrase(project_type: &str) -> &'static str {
    match project_type {
        "clinic" => "Clinic project initialized.",
        "school" => "School management project initialized.",
        "inventory" => "Inventory project initialized.",
        "blog" => "Blog project initialized.",
        _ => "Custom RustIO project initialized.",
    }
}

/// Valid preset names, surfaced verbatim in the error path so
/// `--preset foo` reports the closed list of choices.
const VALID_PRESETS: &[&str] = &["minimal", "blog", "clinic"];

/// One-line comment appended to the `rustio-admin migrate apply` step
/// in the Next-steps block. Content presets ship migrations and say
/// what they create; the neutral `minimal` scaffold points at
/// `startapp` for the first one.
fn migrate_hint(preset: &str) -> &'static str {
    match preset {
        "blog" => "creates the posts + comments tables",
        "clinic" => "creates the patients + appointments tables (with example rows)",
        _ => "no project migrations yet (`rustio-admin startapp <name>` adds the first)",
    }
}

pub fn project(name: &str, preset: &str) -> Result<(), String> {
    // Non-interactive scaffold paths (`rustio-admin startproject` and
    // `rustio-admin new --no-interactive`) do not collect project_type;
    // fall back to "custom" so the generated homepage is neutral.
    project_in(Path::new("."), name, preset, "custom")
}

/// Wizard-flavoured scaffold (PR 1.2). Same file set as [`project`],
/// plus a generated `.env` carrying the wizard's chosen `db_name`,
/// then a Next Steps block that omits the `cp .env.example .env`
/// step. The `project_type` is the wizard's collected choice and
/// drives the `{{type_phrase}}` substitution in `templates/home.html`
/// (PR 1.5 / `DESIGN_ONBOARDING.md` §6). Non-interactive callers of
/// `rustio-admin new` keep using [`project`] verbatim.
pub fn project_with_db(
    name: &str,
    preset: &str,
    db_name: &str,
    project_type: &str,
) -> Result<(), String> {
    project_with_db_in(Path::new("."), name, preset, db_name, project_type)
}

/// Workdir-parameterised variant -- `project()` calls this with
/// `Path::new(".")`. Pulled out so unit tests can scaffold under
/// a tempdir without changing the process working directory.
fn project_in(parent: &Path, name: &str, preset: &str, project_type: &str) -> Result<(), String> {
    let written = write_project_files(parent, name, preset, project_type)?;
    println!("Created `{name}/` ({preset} preset) with {written} files.");
    println!();
    println!("Next steps:");
    println!("  cd {name}");
    println!("  cp .env.example .env       # safe local defaults; edit before production");
    println!("  rustio-admin migrate apply # {}", migrate_hint(preset));
    println!("  rustio-admin user create --email admin@{name}.local --role administrator");
    println!(
        "  cargo run                  # http://127.0.0.1:8000 (homepage) + /admin + /admin/docs"
    );
    Ok(())
}

/// Workdir-parameterised wizard variant -- tests reach this via a
/// tempdir; `project_with_db` calls it with `Path::new(".")`.
fn project_with_db_in(
    parent: &Path,
    name: &str,
    preset: &str,
    db_name: &str,
    project_type: &str,
) -> Result<(), String> {
    let written = write_project_files(parent, name, preset, project_type)?;
    let project_root = parent.join(name);
    write_env_file(&project_root, db_name)?;
    println!(
        "Created `{name}/` ({preset} preset) with {} files (including .env).",
        written + 1
    );
    println!();
    println!("Next steps:");
    println!("  cd {name}");
    println!("  rustio-admin migrate apply # {}", migrate_hint(preset));
    println!("  rustio-admin user create --email admin@{name}.local --role administrator");
    println!(
        "  cargo run                  # http://127.0.0.1:8000 (homepage) + /admin + /admin/docs"
    );
    println!();
    // First-build expectation note. PR 1.4 / DESIGN_ONBOARDING.md §9
    // -- only printed on the wizard path so script users (who know
    // what to expect from `cargo build`) don't see it.
    println!("Note: the first `cargo run` may take several minutes -- that is normal for a fresh Rust project.");
    Ok(())
}

/// Pure file-writing core. Extracted so the wizard path can reuse
/// it before layering its own `.env` on top. Validates name +
/// preset, refuses an existing target dir, and returns the number
/// of files written so callers can report it. `{{name}}` and
/// `{{type_phrase}}` are substituted in every template; only
/// `templates/home.html` actually carries `{{type_phrase}}` today
/// (everywhere else the replacement is a no-op).
fn write_project_files(
    parent: &Path,
    name: &str,
    preset: &str,
    project_type: &str,
) -> Result<usize, String> {
    validate_name(name)?;
    if !VALID_PRESETS.contains(&preset) {
        return Err(format!(
            "unknown preset `{preset}`. Valid: {}",
            VALID_PRESETS.join(", ")
        ));
    }

    let dir = parent.join(name);
    let dir = dir.as_path();
    if dir.exists() {
        return Err(format!(
            "`{name}` already exists in the current directory. Pick a fresh name or remove it first."
        ));
    }

    let type_phrase = type_phrase(project_type);
    let name_title = humanise_name(name);
    let mut written = 0usize;
    for (rel, body) in PROJECT_TEMPLATES {
        let target = dir.join(rel);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("mkdir {}: {e}", parent.display()))?;
        }
        let body = body
            .replace("{{name}}", name)
            .replace("{{name_title}}", &name_title)
            .replace("{{type_phrase}}", type_phrase);
        fs::write(&target, body).map_err(|e| format!("write {}: {e}", target.display()))?;
        written += 1;
    }

    if let Some((overrides, extras)) = preset_layers(preset) {
        for (rel, body) in overrides.iter().chain(extras.iter()) {
            let target = dir.join(rel);
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent)
                    .map_err(|e| format!("mkdir {}: {e}", parent.display()))?;
            }
            let body = body
                .replace("{{name}}", name)
                .replace("{{name_title}}", &name_title)
                .replace("{{type_phrase}}", type_phrase);
            fs::write(&target, body).map_err(|e| format!("write {}: {e}", target.display()))?;
            // Overrides reuse a slot the minimal scaffold already
            // wrote -- don't double-count those. Extras are net-new
            // files; bump the counter for them.
            if extras.iter().any(|(p, _)| *p == *rel) {
                written += 1;
            }
        }
    }

    Ok(written)
}

/// Write the wizard-generated `.env` alongside the scaffold's
/// `.env.example`. The `.env.example` stays as the team-shareable
/// template; `.env` is the local-dev convenience the wizard adds
/// so the user does not need to `cp` it manually. `.gitignore` (from
/// the scaffold's template) already excludes `.env`.
fn write_env_file(project_root: &Path, db_name: &str) -> Result<(), String> {
    let path = project_root.join(".env");
    let body = format!(
        "# Generated by `rustio-admin new` wizard. Local-dev defaults.\n\
         # Review and rotate before deploying.\n\
         \n\
         RUST_LOG=info\n\
         \n\
         DB_HOST=localhost\n\
         DB_PORT=5432\n\
         DB_USER=postgres\n\
         DB_PASSWORD=postgres\n\
         DB_NAME={db_name}\n\
         \n\
         DATABASE_URL=postgres://${{DB_USER}}:${{DB_PASSWORD}}@${{DB_HOST}}:${{DB_PORT}}/${{DB_NAME}}\n\
         \n\
         # 32-byte URL-safe-base64 key for R3 MFA + R4 emergency-access.\n\
         # Generate with: openssl rand 32 | base64 | tr '+/' '-_' | tr -d '='\n\
         RUSTIO_SECRET_KEY=\n"
    );
    fs::write(&path, body).map_err(|e| format!("write {}: {e}", path.display()))
}

/// A project name must be a valid Rust crate identifier: ASCII
/// letters / digits / `-` / `_`, not starting with a digit, not
/// empty. The Cargo.toml template uses the name verbatim, so any
/// character cargo would reject here would just shift the failure
/// downstream.
fn validate_name(name: &str) -> Result<(), String> {
    if name.is_empty() {
        return Err("project name is required".into());
    }
    if name.starts_with(|c: char| c.is_ascii_digit()) {
        return Err("project name may not start with a digit".into());
    }
    let valid = name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
    if !valid {
        return Err("project name may only contain ASCII letters, digits, '-', and '_'".into());
    }
    Ok(())
}

// ---- startapp --------------------------

const APP_MODEL_TEMPLATE: &str = include_str!("../templates/app/model.rs.tmpl");
const APP_MIGRATION_TEMPLATE: &str = include_str!("../templates/app/migration.sql.tmpl");

// PR 2.1: with-fields path. Layered alongside the original templates
// so the no-fields path stays byte-identical with what 0.20.0 ships.
const APP_MODEL_WITH_FIELDS_TEMPLATE: &str =
    include_str!("../templates/app/model_with_fields.rs.tmpl");
const APP_MIGRATION_WITH_FIELDS_TEMPLATE: &str =
    include_str!("../templates/app/migration_with_fields.sql.tmpl");

/// `rustio-admin startapp <name>` entry point.
///
/// - With no `--field` flags AND non-TTY / `--no-interactive` / CI:
///   the output is byte-identical to what 0.20.0 shipped (one
///   placeholder `name` + `created_at` field). Compat-locked by
///   the `scaffold::tests::startapp_no_fields_writes_byte_identical_to_0_20_0` test.
/// - With `--field name:type` flags (repeatable): the parsed field
///   list drives the with-fields templates.
/// - In TTY mode with no `--field` flags AND `--no-interactive`
///   unset: an interactive prompt loop collects fields one per
///   line; an empty line ends the loop. If zero fields are
///   collected the placeholder path runs.
pub fn app(name: &str, field_args: Vec<String>, no_interactive: bool) -> Result<(), String> {
    validate_app_name(name)?;
    ensure_in_project_root()?;

    let singular = camel_case(name);
    // Pluralise via `app_fields::pluralise_snake` so `class` -> `classes`
    // (not the audit-broken `classs`), `bus` -> `buses`, `category` ->
    // `categories`, etc. Same helper drives FK target tables in
    // `app_fields::render`.
    let table = crate::app_fields::pluralise_snake(name);

    let model_path = Path::new("src").join(format!("{name}.rs"));
    if model_path.exists() {
        return Err(format!(
            "{} already exists; pick a different name or remove the file first.",
            model_path.display()
        ));
    }

    let next_version = next_migration_version()?;
    let migration_path =
        Path::new("migrations").join(format!("{next_version:04}_create_{table}.sql"));
    fs::create_dir_all("migrations").map_err(|e| format!("mkdir migrations: {e}"))?;

    // Resolve the field list. CLI flags take precedence; interactive
    // prompt fills it in when allowed.
    let fields = collect_fields(field_args, no_interactive)?;

    let field_count = if fields.is_empty() {
        // Compat path -- byte-identical to 0.20.0.
        let model_body = APP_MODEL_TEMPLATE
            .replace("{{Singular}}", &singular)
            .replace("{{name}}", name)
            .replace("{{table}}", &table);
        let migration_body = APP_MIGRATION_TEMPLATE
            .replace("{{name}}", name)
            .replace("{{table}}", &table);
        fs::write(&model_path, model_body)
            .map_err(|e| format!("write {}: {e}", model_path.display()))?;
        fs::write(&migration_path, migration_body)
            .map_err(|e| format!("write {}: {e}", migration_path.display()))?;
        None
    } else {
        let r = crate::app_fields::render(&fields);
        let model_body = APP_MODEL_WITH_FIELDS_TEMPLATE
            .replace("{{Singular}}", &singular)
            .replace("{{name}}", name)
            .replace("{{table}}", &table)
            .replace("{{imports}}", &r.imports)
            .replace("{{struct_fields}}", &r.struct_fields)
            .replace("{{columns_literal}}", &r.columns_literal)
            .replace("{{insert_columns_literal}}", &r.insert_columns_literal)
            .replace("{{from_row_assignments}}", &r.from_row_assignments)
            .replace("{{insert_values_expr}}", &r.insert_values_expr)
            .replace("{{list_display_literal}}", &r.list_display_literal)
            .replace("{{search_fields_literal}}", &r.search_fields_literal);
        let migration_body = APP_MIGRATION_WITH_FIELDS_TEMPLATE
            .replace("{{name}}", name)
            .replace("{{table}}", &table)
            .replace("{{column_decls}}", &r.column_decls_sql);
        fs::write(&model_path, model_body)
            .map_err(|e| format!("write {}: {e}", model_path.display()))?;
        fs::write(&migration_path, migration_body)
            .map_err(|e| format!("write {}: {e}", migration_path.display()))?;
        Some(fields.len())
    };

    // Spatial-orientation output: name files once, then refer to
    // them by their marker location in `src/main.rs`. Operational
    // values (paths, commands, URLs) get color via `console`; the
    // crate auto-degrades under NO_COLOR / non-TTY.
    print_startapp_summary(
        &singular,
        &model_path,
        &migration_path,
        &table,
        field_count,
        name,
    );
    Ok(())
}

/// One-shot console output for `rustio-admin startapp`. Spatial layout +
/// `console::style` for colour. NO_COLOR / non-TTY degrade
/// automatically (the `console` crate strips ANSI in that case).
fn print_startapp_summary(
    singular: &str,
    model_path: &Path,
    migration_path: &Path,
    table: &str,
    field_count: Option<usize>,
    name: &str,
) {
    use console::style;

    let path_style = |p: &str| style(p.to_string()).cyan();
    let cmd_style = |c: &str| style(c.to_string()).green();
    let url_style = |u: &str| style(u.to_string()).cyan().underlined();
    let mark_style = |m: &str| style(m.to_string()).dim();
    let check = style("✓".to_string()).green();

    println!();
    let header = match field_count {
        Some(n) => format!(
            "MODEL CREATED  {}  ({} field{})",
            style(singular).bold(),
            n,
            if n == 1 { "" } else { "s" }
        ),
        None => format!("MODEL CREATED  {}", style(singular).bold()),
    };
    println!("{header}");
    println!(
        "  {} {}",
        check,
        path_style(&model_path.display().to_string())
    );
    println!(
        "  {} {}",
        check,
        path_style(&migration_path.display().to_string())
    );
    println!();

    // Inspect main.rs for the three markers. Each missing marker
    // surfaces a single line of warning ABOVE the edit block so the
    // developer adds the marker first, then the edit it gates.
    let main_rs = Path::new("src").join("main.rs");
    let main_src = fs::read_to_string(&main_rs).unwrap_or_default();
    let mods_ok = main_src.contains("// rustio: modules");
    let imports_ok = main_src.contains("// rustio: imports");
    let models_ok = main_src.contains("// rustio: models");

    if !(mods_ok && imports_ok && models_ok) {
        let warn = style("WARNING").yellow().bold();
        println!("{warn}  `src/main.rs` is missing one or more rustio insertion markers.");
        println!("         Add the marker(s) below to your file manually before applying");
        println!("         the edits. `rustio-admin startapp` deliberately does NOT edit your");
        println!("         `src/main.rs` -- you stay the author.");
        println!();
        if !mods_ok {
            println!(
                "  add   {}  (suggested location: after the `use` block at the top)",
                mark_style("// rustio: modules")
            );
        }
        if !imports_ok {
            println!(
                "  add   {}  (suggested location: after the `// rustio: modules` line)",
                mark_style("// rustio: imports")
            );
        }
        if !models_ok {
            println!(
                "  add   {}  (suggested location: inside the `Admin::new()...` builder chain, before `;`)",
                mark_style("// rustio: models")
            );
        }
        println!();
    }

    println!(
        "{} edits in {}",
        style("3").bold(),
        path_style("src/main.rs")
    );
    println!(
        "  under  {}  add  {}",
        mark_style("// rustio: modules"),
        style(format!("mod {name};")).bold()
    );
    println!(
        "  under  {}  add  {}",
        mark_style("// rustio: imports "),
        style(format!("use {name}::{singular};")).bold()
    );
    println!(
        "  under  {}  add  {}",
        mark_style("// rustio: models  "),
        style(format!(".model::<{singular}>()")).bold()
    );
    println!();

    println!("  {}", cmd_style("$ rustio-admin migrate apply"));
    println!("  {}", cmd_style("$ cargo run"));
    println!(
        "  {}  {}",
        style("admin").dim(),
        url_style(&format!("http://127.0.0.1:8000/admin/{table}"))
    );
}

/// Parse the `--field` CLI args, then -- if zero fields landed AND
/// the wizard is eligible -- run the interactive prompt loop.
/// Returns the validated field list (may be empty, which routes the
/// caller to the compat / placeholder template path).
fn collect_fields(
    field_args: Vec<String>,
    no_interactive: bool,
) -> Result<Vec<crate::app_fields::Field>, String> {
    let mut fields: Vec<crate::app_fields::Field> = Vec::with_capacity(field_args.len());
    for raw in field_args {
        let field = crate::app_fields::parse_field(&raw).map_err(|e| e.format())?;
        fields.push(field);
    }
    crate::app_fields::validate_unique_names(&fields).map_err(|e| e.format())?;

    // Interactive prompt -- only when no CLI fields were passed AND
    // the wizard is eligible. Eligibility matches `rustio-admin new` (PR 1.2):
    // both stdin and stdout must be terminals; CI / NO_COLOR / explicit
    // `--no-interactive` bypass entirely.
    if fields.is_empty() && wizard_eligible(no_interactive) {
        fields = prompt_fields_interactively()?;
        crate::app_fields::validate_unique_names(&fields).map_err(|e| e.format())?;
    }

    Ok(fields)
}

fn wizard_eligible(no_interactive: bool) -> bool {
    use std::io::IsTerminal;
    if no_interactive {
        return false;
    }
    if std::env::var_os("CI").is_some() {
        return false;
    }
    std::io::stdin().is_terminal() && std::io::stdout().is_terminal()
}

/// Prompt the operator for fields one line at a time. Empty line
/// ends the loop. Invalid input re-prompts without consuming the
/// slot number. EOF (Ctrl-D) is treated as "done".
fn prompt_fields_interactively() -> Result<Vec<crate::app_fields::Field>, String> {
    use std::io::{self, Write};

    println!("--------------------------------------------------");
    println!("rustio-admin startapp -- field declarations");
    println!("--------------------------------------------------");
    println!();
    println!("Add fields one at a time. Format: <name>:<type>");
    println!("Press ENTER on an empty line to finish.");
    println!();
    println!("Types: str, text, int, bigint, bool, timestamp, json, fk:<Model>");
    println!();

    let mut out = Vec::<crate::app_fields::Field>::new();
    loop {
        print!("  field {}> ", out.len() + 1);
        io::stdout().flush().map_err(|e| format!("flush: {e}"))?;
        let mut buf = String::new();
        let n = io::stdin()
            .read_line(&mut buf)
            .map_err(|e| format!("read stdin: {e}"))?;
        if n == 0 {
            // EOF -- treat as "done".
            break;
        }
        let input = buf.trim();
        if input.is_empty() {
            break;
        }
        match crate::app_fields::parse_field(input) {
            Ok(f) => out.push(f),
            Err(e) => {
                // Show the four-part error inline and re-prompt
                // without advancing the slot counter.
                println!();
                println!("{}", e.format());
                println!();
            }
        }
    }
    if !out.is_empty() {
        println!();
        println!("Summary");
        for f in &out {
            println!("  {}: {:?}", f.name, f.kind);
        }
        println!();
    }
    Ok(out)
}

/// App names follow Rust module rules: ASCII lowercase letters /
/// digits / `_`, not starting with a digit. We deliberately reject
/// `-` here because it can't appear in a Rust module path without
/// `r#`-escapes. Stricter than `validate_name` on purpose.
fn validate_app_name(name: &str) -> Result<(), String> {
    if name.is_empty() {
        return Err("app name is required".into());
    }
    if name.starts_with(|c: char| c.is_ascii_digit()) {
        return Err("app name may not start with a digit".into());
    }
    let valid = name
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_');
    if !valid {
        return Err(
            "app name may only contain lowercase ASCII letters, digits, and '_' \
             (e.g. `post`, `course`, `book_review`)"
                .into(),
        );
    }
    Ok(())
}

/// Refuse to scaffold an app outside a project. We recognise a
/// project root by the combo of `Cargo.toml` and `src/main.rs`,
/// which `rustio-admin startproject` always lays down -- and which
/// `cargo new --bin` produces too.
fn ensure_in_project_root() -> Result<(), String> {
    if !Path::new("Cargo.toml").exists() {
        return Err(
            "no Cargo.toml in the current directory. Run `rustio-admin startapp` from the \
             project root, or scaffold a fresh project with `rustio-admin startproject <name>` \
             first."
                .into(),
        );
    }
    if !Path::new("src").join("main.rs").exists() {
        return Err(
            "no src/main.rs in the current directory. The CLI scaffolds models for \
             binary projects."
                .into(),
        );
    }
    Ok(())
}

/// snake_case → CamelCase. `book_review` → `BookReview`. Single-word
/// inputs hit the Title Case path, which is the same shape.
fn camel_case(snake: &str) -> String {
    let mut out = String::with_capacity(snake.len());
    let mut next_upper = true;
    for c in snake.chars() {
        if c == '_' {
            next_upper = true;
        } else if next_upper {
            out.extend(c.to_uppercase());
            next_upper = false;
        } else {
            out.push(c);
        }
    }
    out
}

/// Walk `migrations/` and return the next available `NNNN` prefix.
/// Picks `1` for an empty / missing directory; otherwise `max + 1`
/// across every parseable filename.
fn next_migration_version() -> Result<i64, String> {
    let dir = Path::new("migrations");
    if !dir.exists() {
        return Ok(1);
    }
    let mut highest: i64 = 0;
    for entry in fs::read_dir(dir).map_err(|e| format!("read_dir migrations: {e}"))? {
        let entry = entry.map_err(|e| format!("dir entry: {e}"))?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("sql") {
            continue;
        }
        let stem = match path.file_stem().and_then(|s| s.to_str()) {
            Some(s) => s,
            None => continue,
        };
        let prefix = stem.split_once('_').map(|(p, _)| p).unwrap_or(stem);
        if let Ok(n) = prefix.parse::<i64>() {
            if n > highest {
                highest = n;
            }
        }
    }
    Ok(highest + 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_names_accepted() {
        for name in &["my-app", "my_app", "MyApp", "app1", "a-b_c-1"] {
            assert!(validate_name(name).is_ok(), "should accept {name}");
        }
    }

    #[test]
    fn invalid_names_rejected() {
        for name in &["", "1app", "my app", "my/app", "my.app", "my\u{1F600}app"] {
            assert!(validate_name(name).is_err(), "should reject {name:?}");
        }
    }

    // `const_is_empty` correctly notes the standalone-const checks
    // below are compile-time constants -- but that's the point.
    // Catches a regression where a templates file gets emptied or
    // include_str! points at the wrong path.
    #[allow(clippy::const_is_empty)]
    #[test]
    fn every_template_carries_at_least_one_placeholder_or_fixed_content() {
        // Sanity check that the static slice is wired correctly.
        // Empty templates are also a regression -- `include_str!` would
        // happily load a zero-byte file but the scaffold would write
        // empty files into the new project.
        for (rel, body) in PROJECT_TEMPLATES {
            assert!(!body.is_empty(), "template {rel} is empty");
        }
        assert!(
            !APP_MODEL_TEMPLATE.is_empty(),
            "app model template is empty"
        );
        assert!(
            !APP_MIGRATION_TEMPLATE.is_empty(),
            "app migration template is empty"
        );
    }

    #[test]
    fn humanise_name_capitalises_words_and_splits_on_separators() {
        assert_eq!(humanise_name("clinic"), "Clinic");
        assert_eq!(humanise_name("my-clinic"), "My Clinic");
        assert_eq!(humanise_name("school_admin"), "School Admin");
        assert_eq!(humanise_name("acme_dental_clinic"), "Acme Dental Clinic");
        // Leading/trailing/repeated separators stay clean.
        assert_eq!(humanise_name("_foo__bar_"), "Foo Bar");
        assert_eq!(humanise_name(""), "");
    }

    #[test]
    fn camel_case_handles_single_word_and_snake() {
        assert_eq!(camel_case("post"), "Post");
        assert_eq!(camel_case("book_review"), "BookReview");
        assert_eq!(camel_case("a_b_c"), "ABC");
        assert_eq!(camel_case(""), "");
    }

    #[test]
    fn validate_app_name_accepts_typical_names() {
        for name in &["post", "course", "book_review", "user2", "v1"] {
            assert!(validate_app_name(name).is_ok(), "should accept {name}");
        }
    }

    #[test]
    fn validate_app_name_rejects_capitals_and_dashes() {
        for name in &["Post", "BOOK", "book-review", "1book", "", "book.review"] {
            assert!(validate_app_name(name).is_err(), "should reject {name:?}");
        }
    }

    // ---- project presets ----

    #[allow(clippy::const_is_empty)]
    #[test]
    fn blog_preset_templates_are_non_empty() {
        for (rel, body) in BLOG_OVERRIDES.iter().chain(BLOG_EXTRAS.iter()) {
            assert!(!body.is_empty(), "blog template {rel} is empty");
        }
    }

    #[allow(clippy::const_is_empty)]
    #[test]
    fn clinic_preset_templates_are_non_empty() {
        for (rel, body) in CLINIC_OVERRIDES.iter().chain(CLINIC_EXTRAS.iter()) {
            assert!(!body.is_empty(), "clinic template {rel} is empty");
        }
    }

    #[test]
    fn project_rejects_unknown_preset_with_valid_list_in_message() {
        let dir = unique_tempdir();
        let err =
            project_in(&dir, "proj", "definitely-not-a-preset", "custom").expect_err("must error");
        assert!(err.contains("unknown preset"), "got: {err}");
        assert!(err.contains("minimal"), "must list minimal: {err}");
        assert!(err.contains("blog"), "must list blog: {err}");
        assert!(err.contains("clinic"), "must list clinic: {err}");
    }

    /// The `clinic` preset must produce a *real* clinic — `Patient`
    /// and `Appointment` models, both migrations, and a `main.rs`
    /// that registers both — so the developer reaches a working,
    /// non-empty admin without hand-editing a file.
    #[test]
    fn project_clinic_wires_models_and_seeded_migrations() {
        let dir = unique_tempdir();
        project_in(&dir, "clinic", "clinic", "clinic").expect("clinic should scaffold");
        let root = dir.join("clinic");

        for f in [
            "src/patient.rs",
            "src/appointment.rs",
            "migrations/0001_create_patients.sql",
            "migrations/0002_create_appointments.sql",
        ] {
            assert!(root.join(f).exists(), "clinic preset must write {f}");
        }

        // main.rs registers both models and is fully wired — no
        // manual edit required on the first run.
        let main = fs::read_to_string(root.join("src/main.rs")).unwrap();
        assert!(
            main.contains("mod patient;"),
            "main.rs must declare patient module"
        );
        assert!(
            main.contains("mod appointment;"),
            "main.rs must declare appointment module"
        );
        assert!(
            main.contains(".model::<Patient>()"),
            "main.rs must register Patient: {main}"
        );
        assert!(
            main.contains(".model::<Appointment>()"),
            "main.rs must register Appointment"
        );
        // `{{name_title}}` must be substituted, not left literal.
        assert!(
            main.contains(".app_name(\"Clinic\")"),
            "main.rs app_name must be the humanised project name: {main}"
        );
        assert!(!main.contains("{{"), "no placeholder may survive: {main}");

        // The appointments migration seeds rows linked by patient name
        // so the foreign keys are correct regardless of generated ids.
        let appts =
            fs::read_to_string(root.join("migrations/0002_create_appointments.sql")).unwrap();
        assert!(
            appts.contains("REFERENCES patients(id)"),
            "appointments must FK to patients"
        );
        assert!(
            appts.contains("SELECT id FROM patients WHERE full_name ="),
            "seed must link appointments to patients by name"
        );
    }

    /// PR 1.5: the minimal scaffold is neutral — no `post.rs`, no
    /// `0001_create_posts.sql`, no Post registration in main.rs.
    /// A clinic / school / inventory project must stop feeling
    /// like a renamed blog demo.
    #[test]
    fn project_minimal_is_neutral_with_no_domain_models() {
        let dir = unique_tempdir();
        project_in(&dir, "proj", "minimal", "custom").expect("minimal should scaffold");
        let root = dir.join("proj");
        assert!(
            !root.join("src/post.rs").exists(),
            "post.rs must NOT exist in minimal scaffold (moved to blog preset)"
        );
        assert!(
            !root.join("src/comment.rs").exists(),
            "comment.rs must NOT exist in minimal scaffold"
        );
        assert!(
            !root.join("migrations/0001_create_posts.sql").exists(),
            "0001_create_posts.sql must NOT exist in minimal scaffold"
        );
        // main.rs must not reference Post or Comment anywhere.
        let main = fs::read_to_string(root.join("src/main.rs")).unwrap();
        assert!(
            !main.contains("Post"),
            "minimal main.rs must not mention Post: {main}"
        );
        assert!(
            !main.contains("Comment"),
            "minimal main.rs must not mention Comment"
        );
    }

    /// PR 1.5: the homepage at `/` is generated alongside the
    /// scaffold so a fresh `cargo run` shows something more
    /// inviting than a 404 at the root.
    #[test]
    fn project_minimal_writes_homepage_and_migrations_placeholder() {
        let dir = unique_tempdir();
        project_in(&dir, "clinic", "minimal", "clinic").expect("scaffold");
        let root = dir.join("clinic");
        assert!(
            root.join("templates/home.html").exists(),
            "templates/home.html must exist"
        );
        assert!(
            root.join("migrations/.gitkeep").exists(),
            "migrations/.gitkeep must exist (keeps empty dir visible to git)"
        );
        // main.rs serves the homepage via include_str! and uses Response::html.
        let main = fs::read_to_string(root.join("src/main.rs")).unwrap();
        assert!(
            main.contains("include_str!(\"../templates/home.html\")"),
            "main.rs must bake home.html: {main}"
        );
        assert!(main.contains("Response::html("), "main.rs must serve HTML");
    }

    /// PR 1.5: `{{type_phrase}}` substitution turns the user's
    /// project-type choice into a single calm phrase on the homepage.
    /// Each curated type has its own wording; anything else falls
    /// through to the neutral "Custom RustIO project" phrasing.
    #[test]
    fn homepage_substitutes_type_phrase_per_project_type() {
        let cases = [
            ("custom", "Custom RustIO project initialized."),
            ("clinic", "Clinic project initialized."),
            ("school", "School management project initialized."),
            ("inventory", "Inventory project initialized."),
            ("blog", "Blog project initialized."),
            // Unknown type falls back to custom — neutral, never wrong.
            ("unrecognised", "Custom RustIO project initialized."),
        ];
        for (ty, expected) in cases {
            let dir = unique_tempdir();
            let preset = if ty == "blog" { "blog" } else { "minimal" };
            project_in(&dir, "proj", preset, ty).unwrap_or_else(|e| panic!("{ty}: {e}"));
            let html =
                fs::read_to_string(dir.join("proj").join("templates").join("home.html")).unwrap();
            assert!(
                html.contains(expected),
                "type {ty} should render '{expected}'; got: {html}"
            );
            // The literal placeholder must not leak through.
            assert!(
                !html.contains("{{type_phrase}}"),
                "placeholder leaked for type {ty}: {html}"
            );
            // Project name substitution still works.
            assert!(html.contains("proj"), "project name must substitute");
        }
    }

    #[test]
    fn project_blog_ships_post_comment_and_both_migrations() {
        let dir = unique_tempdir();
        project_in(&dir, "blog", "blog", "blog").expect("blog should scaffold");
        let root = dir.join("blog");
        assert!(root.join("src/post.rs").exists(), "post.rs missing");
        assert!(root.join("src/comment.rs").exists(), "comment.rs missing");
        assert!(
            root.join("migrations/0001_create_posts.sql").exists(),
            "0001 migration missing"
        );
        assert!(
            root.join("migrations/0002_create_comments.sql").exists(),
            "0002 migration missing"
        );
        let main = fs::read_to_string(root.join("src/main.rs")).unwrap();
        assert!(main.contains(".model::<Post>()"), "Post must be registered");
        assert!(
            main.contains(".model::<Comment>()"),
            "Comment must be registered"
        );
        // Blog still uses the same homepage template.
        assert!(
            root.join("templates/home.html").exists(),
            "templates/home.html must exist on blog scaffold too"
        );
    }

    // ---- project_with_db (wizard path, PR 1.2 + PR 1.5) ----

    #[test]
    fn project_with_db_writes_env_with_chosen_db_name() {
        let dir = unique_tempdir();
        project_with_db_in(&dir, "clinic", "minimal", "clinic_2026", "clinic")
            .expect("wizard scaffold should succeed");
        let env = fs::read_to_string(dir.join("clinic").join(".env"))
            .expect(".env must exist after wizard scaffold");
        assert!(
            env.contains("DB_NAME=clinic_2026"),
            "DB_NAME line missing: {env}"
        );
        assert!(
            env.contains("DATABASE_URL=postgres://"),
            "DATABASE_URL line missing: {env}"
        );
        // The composed URL uses ${DB_NAME} substitution, so the literal
        // db name must NOT appear in the URL line itself.
        let url_line = env
            .lines()
            .find(|l| l.starts_with("DATABASE_URL="))
            .unwrap();
        assert!(
            url_line.contains("${DB_NAME}"),
            "URL should compose from components: {url_line}"
        );
    }

    #[test]
    fn project_with_db_also_writes_env_example_for_team_sharing() {
        let dir = unique_tempdir();
        project_with_db_in(&dir, "school", "minimal", "school_dev", "school").unwrap();
        let root = dir.join("school");
        assert!(
            root.join(".env").exists(),
            ".env (wizard-generated) must exist"
        );
        assert!(
            root.join(".env.example").exists(),
            ".env.example (team template) must still exist"
        );
        assert!(
            root.join(".gitignore").exists(),
            ".gitignore must exist (already excludes .env)"
        );
        let gi = fs::read_to_string(root.join(".gitignore")).unwrap();
        assert!(
            gi.lines().any(|l| l.trim() == ".env"),
            ".gitignore must exclude .env: {gi}"
        );
        // Homepage carries the chosen project type's phrase.
        let html = fs::read_to_string(root.join("templates/home.html")).unwrap();
        assert!(
            html.contains("School management project initialized."),
            "homepage should reflect school type: {html}"
        );
    }

    #[test]
    fn project_and_project_with_db_share_the_same_file_set() {
        let a = unique_tempdir();
        let b = unique_tempdir();
        project_in(&a, "proj", "minimal", "custom").unwrap();
        project_with_db_in(&b, "proj", "minimal", "proj_dev", "custom").unwrap();
        let mut a_files: Vec<_> = walk_files(&a.join("proj")).collect();
        let mut b_files: Vec<_> = walk_files(&b.join("proj")).collect();
        a_files.sort();
        b_files.sort();
        // The wizard adds exactly `.env`; everything else is shared.
        let expected_extra = ".env".to_string();
        let extras_in_b: Vec<_> = b_files
            .iter()
            .filter(|p| !a_files.contains(p))
            .cloned()
            .collect();
        assert_eq!(
            extras_in_b,
            vec![expected_extra],
            "only `.env` should be new"
        );
    }

    fn walk_files(root: &Path) -> impl Iterator<Item = String> + '_ {
        fn rec(dir: &Path, base: &Path, out: &mut Vec<String>) {
            for entry in fs::read_dir(dir).unwrap().flatten() {
                let path = entry.path();
                if path.is_dir() {
                    rec(&path, base, out);
                } else {
                    out.push(
                        path.strip_prefix(base)
                            .unwrap()
                            .to_string_lossy()
                            .into_owned(),
                    );
                }
            }
        }
        let mut out = Vec::new();
        rec(root, root, &mut out);
        out.into_iter()
    }

    /// Stdlib-only tempdir for scaffold tests -- no `tempfile` dep
    /// just for the scaffold suite.
    fn unique_tempdir() -> std::path::PathBuf {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let pid = std::process::id();
        let dir = std::env::temp_dir().join(format!("rustio-scaffold-{pid}-{n}"));
        fs::create_dir_all(&dir).unwrap();
        dir
    }
}
