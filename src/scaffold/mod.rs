use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const MAIN_TEMPLATE: &str = r#"mod data;

use dataspec::DataCatalog;

#[tokio::main]
async fn main() {
    let catalog: DataCatalog = data::register_data();
    dataspec::spec_handler(&catalog).await;
}
"#;

const BUILD_TEMPLATE: &str = r#"fn main() {
    dataspec::spec_builder("data-specs", "src/data.rs").expect("failed to build data catalog");
}
"#;

const LIB_TEMPLATE: &str = r#"pub mod data;
pub use data::*;
"#;

const GITIGNORE_EXTRA: &str = "src/data.rs\n";

const DUMMY_CONFIG: &str = r#"# config

## Type
config

| Key | Value | Description |
| --- | --- | --- |
| `provider` | `dryrun` | Storage backend: dryrun, bq, pg |
| `environment` | `development` | Current runtime environment |
"#;

const DUMMY_MODEL: &str = r#"# dummy_model
Dummy model for getting started.

## Type
model

## Transformation
### Code
```sql
SELECT 1 AS id
```
"#;

const DUMMY_TEMPLATE: &str = r#"# dummy_template
Reusable SQL fragment.

## Type
template

## Transformation
### Code
```sql
SELECT * FROM {{dummy_model}}
```
"#;

const DUMMY_OPERATION: &str = r#"# dummy_operation
Standalone operation.

## Type
operation

## Tags
- maintenance

## Transformation
### Code
```sql
SELECT 1
```
"#;

const DUMMY_TEST: &str = r#"# dummy_test
Data quality check.

## Type
test

## Transformation
### Code
```sql
SELECT COUNT(*) FROM {{dummy_model}}
```
"#;

pub fn create_project(name: &str, path: &Path) -> Result<(), String> {
    let project_dir = path.join(name);
    let dataspec_path = default_dataspec_path();

    let output = Command::new("cargo")
        .current_dir(path)
        .arg("new")
        .arg(name)
        .output()
        .map_err(|e| format!("failed to run cargo new: {e}"))?;

    if !output.status.success() {
        return Err(format!(
            "cargo new failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    write_project_files(&project_dir, &dataspec_path, name)?;
    Ok(())
}

fn write_project_files(
    project_dir: &Path,
    dataspec_path: &Path,
    project_name: &str,
) -> Result<(), String> {
    fs::write(project_dir.join("src/main.rs"), MAIN_TEMPLATE)
        .map_err(|e| format!("failed to write main.rs: {e}"))?;
    fs::write(project_dir.join("src/lib.rs"), LIB_TEMPLATE)
        .map_err(|e| format!("failed to write lib.rs: {e}"))?;
    fs::write(project_dir.join("build.rs"), BUILD_TEMPLATE)
        .map_err(|e| format!("failed to write build.rs: {e}"))?;

    patch_cargo_toml(project_dir, dataspec_path)?;
    patch_gitignore(project_dir)?;
    write_dummy_specs(project_dir)?;
    fs::write(project_dir.join("README.md"), readme_content(project_name))
        .map_err(|e| format!("failed to write README.md: {e}"))?;

    Ok(())
}

fn readme_content(project_name: &str) -> String {
    format!(
        r#"# {project_name}

Data Specs project — markdown specs in `data-specs/` are compiled at build time into `src/data.rs`.

## Quick start

```bash
cargo build
cargo run -- transform --names dummy_model
cargo run -- list --models
```

## Project layout

```
{project_name}/
├── Cargo.toml
├── README.md
├── build.rs
├── data-specs/
│   ├── config/config.md
│   ├── models/
│   ├── templates/
│   ├── operations/
│   └── tests/
└── src/
    ├── main.rs
    ├── lib.rs
    └── data.rs              # generated — do not edit (gitignored)
```

## CLI

### Transform

Run transformations for models by name or tag:

```bash
# Single model (uses default transformation)
cargo run -- transform --names dummy_model

# Explicit transformation
cargo run -- transform --names dummy_model::my_transformation_v2

# By tags
cargo run -- transform --tags core,reporting

# Runtime variables and table mappings
cargo run -- transform --names my_model \
  --vars report_year=2024 \
  --mappings my_model=dataset.table_id

# JSON output
cargo run -- transform --names dummy_model --json
```

### List

Inspect catalog contents:

```bash
cargo run -- list --models
cargo run -- list --operations
cargo run -- list --transformations
cargo run -- list --templates
cargo run -- list --tests

# By name
cargo run -- list --names dummy_model --models

# JSON
cargo run -- list --models --json
```

## Storage backends

Set `provider` in `data-specs/config/config.md`:

| `provider` | Description |
|------------|-------------|
| `dryrun` | Default. Logs SQL, no warehouse call |
| `bq` | Google BigQuery |
| `pg` / `postgres` | PostgreSQL |

## Spec format

See the [specs README](https://github.com/Dataspecs/specs/blob/main/README.md) for the full format reference.
"#
    )
}

fn patch_cargo_toml(project_dir: &Path, dataspec_path: &Path) -> Result<(), String> {
    let cargo_path = project_dir.join("Cargo.toml");
    let mut content = fs::read_to_string(&cargo_path)
        .map_err(|e| format!("failed to read Cargo.toml: {e}"))?;

    content = content.replace("edition = \"2024\"", "edition = \"2021\"");

    let dataspec_path_str = dataspec_path
        .canonicalize()
        .unwrap_or_else(|_| dataspec_path.to_path_buf())
        .display()
        .to_string();

    let deps_block = format!(
        r#"[dependencies]
dataspec = {{ path = "{dataspec_path_str}", features = ["bq", "pg"] }}
tokio = {{ version = "1", features = ["full"] }}

[build-dependencies]
dataspec = {{ path = "{dataspec_path_str}" }}
"#
    );

    if let Some(idx) = content.find("[dependencies]") {
        content.truncate(idx);
    }
    if !content.ends_with('\n') {
        content.push('\n');
    }
    content.push_str(&deps_block);

    fs::write(&cargo_path, content).map_err(|e| format!("failed to write Cargo.toml: {e}"))?;
    Ok(())
}

fn patch_gitignore(project_dir: &Path) -> Result<(), String> {
    let gitignore_path = project_dir.join(".gitignore");
    let mut content = fs::read_to_string(&gitignore_path).unwrap_or_default();
    if !content.contains("src/data.rs") {
        if !content.is_empty() && !content.ends_with('\n') {
            content.push('\n');
        }
        content.push_str(GITIGNORE_EXTRA);
        fs::write(&gitignore_path, content)
            .map_err(|e| format!("failed to write .gitignore: {e}"))?;
    }
    Ok(())
}

fn write_dummy_specs(project_dir: &Path) -> Result<(), String> {
    let specs_root = project_dir.join("data-specs");
    let dirs = [
        ("config", "config.md", DUMMY_CONFIG),
        ("models", "dummy_model.md", DUMMY_MODEL),
        ("templates", "dummy_template.md", DUMMY_TEMPLATE),
        ("operations", "dummy_operation.md", DUMMY_OPERATION),
        ("tests", "dummy_test.md", DUMMY_TEST),
    ];

    for (subdir, filename, content) in dirs {
        let dir = specs_root.join(subdir);
        fs::create_dir_all(&dir).map_err(|e| format!("failed to create {subdir}: {e}"))?;
        fs::write(dir.join(filename), content)
            .map_err(|e| format!("failed to write {filename}: {e}"))?;
    }

    Ok(())
}

fn default_dataspec_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
