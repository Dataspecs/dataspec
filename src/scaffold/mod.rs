use std::fs;
use std::path::Path;
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

    write_project_files(&project_dir, name)?;
    Ok(())
}

fn write_project_files(project_dir: &Path, project_name: &str) -> Result<(), String> {
    fs::write(project_dir.join("src/main.rs"), MAIN_TEMPLATE)
        .map_err(|e| format!("failed to write main.rs: {e}"))?;
    fs::write(project_dir.join("src/lib.rs"), LIB_TEMPLATE)
        .map_err(|e| format!("failed to write lib.rs: {e}"))?;
    fs::write(project_dir.join("build.rs"), BUILD_TEMPLATE)
        .map_err(|e| format!("failed to write build.rs: {e}"))?;

    patch_cargo_toml(project_dir)?;
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

`transform`, `apply`, and `test` share the same runtime flags: `--names`, `--tags`, `--vars`, `--mappings`, `--debug`, and `--json`. Names and tags select different entity kinds depending on the command (see below). `transform` also accepts `--init` to run init hooks.

### Transform

Run transformations for models by name or tag. When a transformation defines hooks, the executor runs them around the transformation SQL in this order:

1. **Init** — only when `--init` is passed
2. **Pre** — always
3. **Transformation** — the model build SQL
4. **Post** — always

Hooks reference operations with optional prop overrides. Define them under `## Hooks` in a transformation spec, or under `### Hooks` in an embedded model transformation. See the [spec format reference](https://github.com/Dataspecs/specs/blob/main/README.md#spec-format-reference) for `Pre`, `Post`, and `Init` sections.

```bash
# Single model (uses default transformation)
cargo run -- transform --names dummy_model

# Explicit transformation
cargo run -- transform --names dummy_model::my_transformation_v2

# Run init hooks (e.g. one-time table setup) before pre/transformation/post
cargo run -- transform --names dummy_model --init

# By tags
cargo run -- transform --tags core,reporting

# Runtime variables and table mappings
cargo run -- transform --names my_model \
  --vars report_year=2024 \
  --mappings my_model=dataset.table_id

# JSON output
cargo run -- transform --names dummy_model --json
```

Example hooks in a transformation spec:

```markdown
## Hooks
### Pre
- [eth_set_block_range](../operations/eth_set_block_range)
    | Key | Value | Description |
    | --- | --- | --- |
    | `start_block` | `{{props__eth_default_start_block}}` | Inclusive lower bound |
    | `end_block` | `999999999` | Inclusive upper bound |
### Post
- [eth_update_watermark](../operations/eth_update_watermark)
    | Key | Value | Description |
    | --- | --- | --- |
    | `model_name` | `eth_blocks` | Model name |
    | `end_block` | `{{props__end_block}}` | Last processed block |
### Init
- [create_watermark_table](../operations/create_watermark_table)
```

Hook operations use SQL compiled at build time into each hook reference. Template chains are supported: `{{model.*}}` tags in template bodies are preserved during template inlining and resolved when the hook usage is compiled (see [Rendering rules](#rendering-rules)). At execution time only runtime variables (`{{vars__*}}`, `{{session_id}}`, `{{<model_name>}}`) are resolved.

### Apply

Run operations by name or tag:

```bash
# Single operation
cargo run -- apply --names dummy_operation

# By tags
cargo run -- apply --tags maintenance

# Runtime variables and table mappings
cargo run -- apply --names dummy_operation \
  --vars report_year=2024 \
  --mappings dummy_model=dataset.table_id

# JSON output
cargo run -- apply --names dummy_operation --json
```

### Test

Run tests linked to models (from the default or explicit transformation, plus column-level tests):

```bash
# Single model (uses default transformation)
cargo run -- test --names dummy_model

# Explicit transformation
cargo run -- test --names dummy_model::my_transformation_v2

# By tags
cargo run -- test --tags core,reporting

# Runtime variables and table mappings
cargo run -- test --names dummy_model \
  --vars report_year=2024 \
  --mappings dummy_model=dataset.table_id

# JSON output
cargo run -- test --names dummy_model --json
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

## Rendering rules

Hook operations and transformation/column tests compile in two steps: entity SQL is compiled first (`{{props__*}}` from config resolved; `{{model.*}}` preserved when referenced from a transformation), then each hook/test usage gets final SQL with model context and prop overrides applied. `{{model.handler}}` becomes `{{<model_name>}}` at usage compile and the table ID at runtime.

| Variable | When | Notes |
|----------|------|-------|
| `{{props__*}}`, `{{model.*}}` | Hook / test usage compile | Baked into hook/test SQL; works in template chains |
| `{{model.tested_column}}` | Test usage compile (column tests) | Column the test is assigned to; absent for transformation-level tests |
| `{{model.handler}}` | Usage compile → Runtime | Becomes `{{<model_name>}}`, then table ID |
| `{{vars__*}}`, `{{session_id}}`, `{{<model_name>}}` | Runtime | CLI / execution context |

See the [dataspec README](https://github.com/Dataspecs/dataspec#rendering-rules) for details and examples.

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

fn patch_cargo_toml(project_dir: &Path) -> Result<(), String> {
    let cargo_path = project_dir.join("Cargo.toml");
    let mut content = fs::read_to_string(&cargo_path)
        .map_err(|e| format!("failed to read Cargo.toml: {e}"))?;

    content = content.replace("edition = \"2024\"", "edition = \"2021\"");

    let dataspec_version = env!("CARGO_PKG_VERSION");

    let deps_block = format!(
        r#"[dependencies]
dataspec = {{ version = "{dataspec_version}", features = ["bq", "pg"] }}
tokio = {{ version = "1", features = ["full"] }}

[build-dependencies]
dataspec = {{ version = "{dataspec_version}" }}
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
