use std::path::PathBuf;

use dataspec::{parse_spec_dir, parse_spec_file, Entity};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 || args[1] != "parse" {
        eprintln!("Usage: dataspec parse <path-to-spec.md | directory>");
        std::process::exit(1);
    }

    let path = PathBuf::from(&args[2]);
    let result = if path.is_dir() {
        parse_spec_dir(&path).map(|entities| {
            for (file, entity) in entities {
                print_entity(&file, &entity);
            }
        })
    } else {
        parse_spec_file(&path).map(|parsed| {
            print_entity(&path, &parsed.entity);
            for derived in parsed.derived {
                print_entity(&path, &derived);
            }
        })
    };

    if let Err(err) = result {
        eprintln!("Error: {err}");
        std::process::exit(1);
    }
}

fn print_entity(path: &std::path::Path, entity: &Entity) {
    println!(
        "{}: {} ({})",
        path.display(),
        entity.name(),
        entity.kind()
    );
    println!(
        "{}",
        serde_json::to_string_pretty(entity).expect("serialize entity")
    );
}
