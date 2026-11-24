//! Import-graph discovery for multi-file programs.
//!
//! The compiler seams stay single-program: imported files are resolved from the
//! root file, parsed once to discover their imports, then concatenated in
//! dependency order and compiled as one program. Names are therefore
//! module-wide rather than namespaced (a documented Phase 6 limitation).

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use nava_ast::ItemKind;

/// The concatenated source of the root file and all its imports.
pub struct Loaded {
    pub source: String,
}

/// Resolve `input` and all of its (transitive) imports into one source text.
pub fn load(input: &Path) -> Result<Loaded, String> {
    let mut visited = HashSet::new();
    let mut ordered = Vec::new();
    visit(input, &mut visited, &mut ordered)?;
    let mut source = String::new();
    for (_, text) in &ordered {
        if !source.is_empty() {
            source.push('\n');
        }
        source.push_str(text);
    }
    Ok(Loaded { source })
}

fn visit(
    path: &Path,
    visited: &mut HashSet<PathBuf>,
    ordered: &mut Vec<(PathBuf, String)>,
) -> Result<(), String> {
    let canonical = fs::canonicalize(path)
        .map_err(|error| format!("could not read '{}': {error}", path.display()))?;
    if !visited.insert(canonical.clone()) {
        return Ok(());
    }
    let text = fs::read_to_string(&canonical)
        .map_err(|error| format!("could not read '{}': {error}", canonical.display()))?;
    let parsed = nava_parser::parse(&text);
    let directory = canonical
        .parent()
        .ok_or_else(|| format!("module '{}' has no parent directory", canonical.display()))?;
    let mut imports = Vec::new();
    for item in &parsed.program.items {
        if let ItemKind::Import { source, .. } = item.kind.unwrap_export() {
            imports.push(source.clone());
        }
    }
    for import in imports {
        let mut dependency = directory.join(&import);
        // Extensionless specifiers resolve to `.ts`, the initial source format.
        if dependency.extension().is_none() && !dependency.is_file() {
            dependency.set_extension("ts");
        }
        visit(&dependency, visited, ordered)?;
    }
    ordered.push((canonical, text));
    Ok(())
}
