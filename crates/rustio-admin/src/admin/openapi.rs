//! Auto-generated OpenAPI 3.0 spec for every registered model.
//!
//! Exposed at `GET /admin/apis/openapi.json` (mounted in
//! `routes.rs`). Operators feed the document into API browsers
//! (Postman / Insomnia / Stoplight) without hand-writing schemas.
//!
//! Scope of v1:
//!
//! - One spec per running framework instance — built on every
//!   request from `Admin::entries()`. Fast: pure-functional walk
//!   over compile-time field metadata, no DB.
//! - Documents the existing JSON-on-write paths (the same surface
//!   `json_api.rs` already serves): list / detail / create /
//!   update / delete per model.
//! - Generates a JSON Schema per model under `components.schemas`,
//!   mapping `FieldType` → JSON types (`Bool` → boolean,
//!   integers → integer, strings / datetimes / file paths → string
//!   with `format` hints; optional variants become `nullable`).
//! - **Form-encoded request body** is documented as
//!   `application/x-www-form-urlencoded` — matching the actual
//!   wire shape. JSON-body parsing for writes is still pending
//!   per the JSON-write-paths v1 caveat.
//!
//! Out of scope for v1: per-route auth/permissions descriptions
//! (operators infer from the framework's role model), bulk
//! actions, framework auth endpoints (login / MFA / recovery —
//! those have their own contract surface in `DESIGN_RECOVERY.md`).

use serde_json::{json, Value};

use super::types::{Admin, AdminEntry, AdminField, FieldType};

/// Build the OpenAPI 3.0 document for `admin`. Returns the
/// document as `serde_json::Value` so the route handler can wrap
/// it via `json_api::json_response` without re-stringifying.
pub(crate) fn build_spec(admin: &Admin) -> Value {
    let mut paths = serde_json::Map::new();
    let mut schemas = serde_json::Map::new();

    for entry in admin.entries() {
        // Core entries (the synthetic User) have a bespoke admin
        // surface; their JSON endpoints aren't part of the
        // project-data API. Skip them from the spec so consumers
        // don't try to drive the User panel programmatically.
        if entry.core {
            continue;
        }
        let schema_name = schema_name_for(entry);
        schemas.insert(schema_name.clone(), schema_for_entry(entry));

        // List + create on /admin/<admin_name>
        let list_path = format!("/admin/{}", entry.admin_name);
        paths.insert(
            list_path,
            json!({
                "get": list_op(entry, &schema_name),
                "post": create_op(entry, &schema_name),
            }),
        );

        // Detail / update / delete on /admin/<admin_name>/{id}
        // — three distinct URLs the framework already serves;
        // the OpenAPI document lists them as separate paths so a
        // client knows which verb hits which URL.
        let detail_path = format!("/admin/{}/{{id}}", entry.admin_name);
        paths.insert(
            detail_path,
            json!({ "get": detail_op(entry, &schema_name) }),
        );

        let edit_path = format!("/admin/{}/{{id}}/edit", entry.admin_name);
        paths.insert(edit_path, json!({ "post": update_op(entry, &schema_name) }));

        let delete_path = format!("/admin/{}/{{id}}/delete", entry.admin_name);
        paths.insert(delete_path, json!({ "post": delete_op(entry) }));
    }

    let app_name = &admin.branding().app_name;
    json!({
        "openapi": "3.0.3",
        "info": {
            "title": format!("{app_name} admin API"),
            "version": "v1",
            "description": "Auto-generated from rustio-admin's registered models. See \
                          `Accept: application/json` on each list / detail / write \
                          endpoint for the response shape. Request bodies are still \
                          form-encoded per the v1 JSON-write-paths caveat."
        },
        "paths": Value::Object(paths),
        "components": {
            "schemas": Value::Object(schemas),
            "responses": {
                "ValidationError": {
                    "description": "Form validation failed.",
                    "content": {
                        "application/json": {
                            "schema": {
                                "type": "object",
                                "required": ["errors", "status"],
                                "properties": {
                                    "errors": {
                                        "type": "array",
                                        "items": { "type": "string" },
                                    },
                                    "status": { "type": "integer", "example": 400 },
                                },
                            },
                        },
                    },
                },
                "Error": {
                    "description": "Framework error (not-found / forbidden / internal).",
                    "content": {
                        "application/json": {
                            "schema": {
                                "type": "object",
                                "required": ["error", "status"],
                                "properties": {
                                    "error": { "type": "string" },
                                    "status": { "type": "integer" },
                                },
                            },
                        },
                    },
                },
            },
        },
    })
}

/// Per-entry component-schema name. Uses `singular_name` so
/// `appointments` → `Appointment` etc., matching what operators
/// already see in the admin chrome.
fn schema_name_for(entry: &AdminEntry) -> String {
    entry.singular_name.to_string()
}

/// Build the `components.schemas.<Model>` JSON Schema for one
/// entry. Each `AdminField` projects to one property with a
/// `type` + `nullable` flag inferred from `FieldType`. Read-only
/// `id` is included so list / detail responses describe the
/// shape clients actually see.
fn schema_for_entry(entry: &AdminEntry) -> Value {
    let mut props = serde_json::Map::new();
    let mut required: Vec<Value> = Vec::new();
    for f in entry.fields {
        props.insert(f.name.to_string(), schema_for_field(f));
        if !f.field_type.nullable() {
            required.push(Value::String(f.name.to_string()));
        }
    }
    let mut obj = serde_json::Map::new();
    obj.insert("type".into(), Value::String("object".into()));
    obj.insert("properties".into(), Value::Object(props));
    if !required.is_empty() {
        obj.insert("required".into(), Value::Array(required));
    }
    Value::Object(obj)
}

/// Map one `FieldType` to its JSON Schema fragment.
fn schema_for_field(field: &AdminField) -> Value {
    let mut obj = serde_json::Map::new();
    match field.field_type {
        FieldType::Bool => {
            obj.insert("type".into(), Value::String("boolean".into()));
        }
        FieldType::I32 | FieldType::I64 | FieldType::OptionalI64 => {
            obj.insert("type".into(), Value::String("integer".into()));
            if matches!(field.field_type, FieldType::I32) {
                obj.insert("format".into(), Value::String("int32".into()));
            } else {
                obj.insert("format".into(), Value::String("int64".into()));
            }
        }
        FieldType::F64 => {
            obj.insert("type".into(), Value::String("number".into()));
            obj.insert("format".into(), Value::String("double".into()));
        }
        FieldType::Decimal => {
            // Arbitrary-precision NUMERIC is serialised as a string to
            // avoid the precision loss a JSON number would incur; the
            // OpenAPI `decimal` format documents the intent.
            obj.insert("type".into(), Value::String("string".into()));
            obj.insert("format".into(), Value::String("decimal".into()));
        }
        FieldType::Uuid => {
            obj.insert("type".into(), Value::String("string".into()));
            obj.insert("format".into(), Value::String("uuid".into()));
        }
        FieldType::String | FieldType::OptionalString => {
            obj.insert("type".into(), Value::String("string".into()));
        }
        FieldType::Email => {
            obj.insert("type".into(), Value::String("string".into()));
            obj.insert("format".into(), Value::String("email".into()));
        }
        FieldType::Phone => {
            // No standard JSON Schema format for phone numbers; the
            // honest representation is a plain string.
            obj.insert("type".into(), Value::String("string".into()));
        }
        FieldType::DateTime | FieldType::OptionalDateTime => {
            obj.insert("type".into(), Value::String("string".into()));
            obj.insert("format".into(), Value::String("date-time".into()));
        }
        FieldType::Date => {
            obj.insert("type".into(), Value::String("string".into()));
            obj.insert("format".into(), Value::String("date".into()));
        }
        FieldType::Time => {
            obj.insert("type".into(), Value::String("string".into()));
            obj.insert("format".into(), Value::String("time".into()));
        }
        FieldType::FilePath | FieldType::OptionalFilePath => {
            obj.insert("type".into(), Value::String("string".into()));
            // No JSON Schema "format: file" exists; documenting as
            // a free-form string is the honest move. The form-
            // encoded request side accepts a multipart upload —
            // the schema describes the stored *path*, not the
            // upload mechanic.
            obj.insert(
                "description".into(),
                Value::String(
                    "Relative path under Admin::uploads_dir; on create / update \
                     the request body carries the upload via multipart/form-data."
                        .into(),
                ),
            );
        }
    }
    if field.field_type.nullable() {
        obj.insert("nullable".into(), Value::Bool(true));
    }
    Value::Object(obj)
}

fn list_op(entry: &AdminEntry, schema_name: &str) -> Value {
    json!({
        "summary": format!("List {} rows.", entry.display_name),
        "tags": [entry.admin_name],
        "parameters": [
            { "name": "q", "in": "query", "schema": { "type": "string" }, "description": "Free-text search across configured fields." },
            { "name": "sort", "in": "query", "schema": { "type": "string" } },
            { "name": "dir", "in": "query", "schema": { "type": "string", "enum": ["asc", "desc"] } },
            { "name": "page", "in": "query", "schema": { "type": "integer", "minimum": 1 } },
            { "name": "per_page", "in": "query", "schema": { "type": "integer", "enum": [25, 50, 100, 200] } },
        ],
        "responses": {
            "200": {
                "description": "Page of rows.",
                "content": {
                    "application/json": {
                        "schema": {
                            "type": "object",
                            "required": ["rows", "total", "page", "per_page", "pages"],
                            "properties": {
                                "rows": { "type": "array", "items": { "$ref": format!("#/components/schemas/{schema_name}") } },
                                "total": { "type": "integer" },
                                "page": { "type": "integer" },
                                "per_page": { "type": "integer" },
                                "pages": { "type": "integer" },
                            },
                        },
                    },
                },
            },
            "default": { "$ref": "#/components/responses/Error" },
        },
    })
}

fn create_op(entry: &AdminEntry, schema_name: &str) -> Value {
    json!({
        "summary": format!("Create a new {}.", entry.singular_name),
        "tags": [entry.admin_name],
        "requestBody": {
            "required": true,
            "content": {
                "application/x-www-form-urlencoded": {
                    "schema": { "$ref": format!("#/components/schemas/{schema_name}") },
                },
            },
        },
        "responses": {
            "201": {
                "description": "Row created.",
                "content": {
                    "application/json": {
                        "schema": {
                            "type": "object",
                            "required": ["ok", "admin_name", "id"],
                            "properties": {
                                "ok": { "type": "boolean", "example": true },
                                "admin_name": { "type": "string" },
                                "id": { "type": "integer" },
                            },
                        },
                    },
                },
            },
            "400": { "$ref": "#/components/responses/ValidationError" },
            "default": { "$ref": "#/components/responses/Error" },
        },
    })
}

fn detail_op(entry: &AdminEntry, schema_name: &str) -> Value {
    json!({
        "summary": format!("Fetch one {} by id.", entry.singular_name),
        "tags": [entry.admin_name],
        "parameters": [id_param()],
        "responses": {
            "200": {
                "description": "Row.",
                "content": {
                    "application/json": {
                        "schema": { "$ref": format!("#/components/schemas/{schema_name}") },
                    },
                },
            },
            "404": { "$ref": "#/components/responses/Error" },
            "default": { "$ref": "#/components/responses/Error" },
        },
    })
}

fn update_op(entry: &AdminEntry, schema_name: &str) -> Value {
    json!({
        "summary": format!("Update an existing {} by id.", entry.singular_name),
        "tags": [entry.admin_name],
        "parameters": [id_param()],
        "requestBody": {
            "required": true,
            "content": {
                "application/x-www-form-urlencoded": {
                    "schema": { "$ref": format!("#/components/schemas/{schema_name}") },
                },
            },
        },
        "responses": {
            "200": {
                "description": "Row updated.",
                "content": {
                    "application/json": {
                        "schema": {
                            "type": "object",
                            "required": ["ok", "admin_name", "id"],
                            "properties": {
                                "ok": { "type": "boolean" },
                                "admin_name": { "type": "string" },
                                "id": { "type": "integer" },
                            },
                        },
                    },
                },
            },
            "400": { "$ref": "#/components/responses/ValidationError" },
            "404": { "$ref": "#/components/responses/Error" },
            "default": { "$ref": "#/components/responses/Error" },
        },
    })
}

fn delete_op(entry: &AdminEntry) -> Value {
    json!({
        "summary": format!("Delete a {} by id.", entry.singular_name),
        "tags": [entry.admin_name],
        "parameters": [id_param()],
        "responses": {
            "200": {
                "description": "Row deleted.",
                "content": {
                    "application/json": {
                        "schema": {
                            "type": "object",
                            "required": ["ok", "admin_name", "id"],
                            "properties": {
                                "ok": { "type": "boolean" },
                                "admin_name": { "type": "string" },
                                "id": { "type": "integer" },
                            },
                        },
                    },
                },
            },
            "404": { "$ref": "#/components/responses/Error" },
            "default": { "$ref": "#/components/responses/Error" },
        },
    })
}

fn id_param() -> Value {
    json!({
        "name": "id",
        "in": "path",
        "required": true,
        "schema": { "type": "integer", "format": "int64" },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_admin() -> Admin {
        Admin::new()
    }

    #[test]
    fn spec_carries_openapi_version_and_info() {
        let spec = build_spec(&empty_admin());
        assert_eq!(spec["openapi"], json!("3.0.3"));
        assert!(spec["info"]["title"].is_string());
        assert!(spec["info"]["version"].is_string());
    }

    #[test]
    fn empty_admin_has_no_project_paths() {
        // The synthetic User entry is `core: true` and must be
        // excluded — operators don't drive `/admin/users` via this
        // generated API surface.
        let spec = build_spec(&empty_admin());
        let paths = spec["paths"].as_object().expect("paths is object");
        let project_paths: Vec<_> = paths.keys().filter(|k| !k.contains("/users")).collect();
        assert!(
            project_paths.is_empty(),
            "expected no project paths, got: {project_paths:?}"
        );
    }

    #[test]
    fn schemas_section_is_object() {
        let spec = build_spec(&empty_admin());
        assert!(spec["components"]["schemas"].is_object());
        // ValidationError + Error component responses present.
        assert!(spec["components"]["responses"]["ValidationError"].is_object());
        assert!(spec["components"]["responses"]["Error"].is_object());
    }

    #[test]
    fn schema_for_field_maps_field_types() {
        let f = |ty| AdminField {
            name: "x",
            label: "x",
            field_type: ty,
            editable: true,
            relation: None,
            choices: None,
        };
        assert_eq!(
            schema_for_field(&f(FieldType::Bool))["type"],
            json!("boolean")
        );
        assert_eq!(
            schema_for_field(&f(FieldType::I32))["type"],
            json!("integer")
        );
        assert_eq!(
            schema_for_field(&f(FieldType::I32))["format"],
            json!("int32")
        );
        assert_eq!(
            schema_for_field(&f(FieldType::I64))["type"],
            json!("integer")
        );
        assert_eq!(
            schema_for_field(&f(FieldType::I64))["format"],
            json!("int64")
        );
        let opt = schema_for_field(&f(FieldType::OptionalI64));
        assert_eq!(opt["type"], json!("integer"));
        assert_eq!(opt["nullable"], json!(true));
        let dt = schema_for_field(&f(FieldType::DateTime));
        assert_eq!(dt["type"], json!("string"));
        assert_eq!(dt["format"], json!("date-time"));
        let opt_str = schema_for_field(&f(FieldType::OptionalString));
        assert_eq!(opt_str["type"], json!("string"));
        assert_eq!(opt_str["nullable"], json!(true));
        let file = schema_for_field(&f(FieldType::FilePath));
        assert_eq!(file["type"], json!("string"));
        assert!(file["description"].is_string());
    }
}
