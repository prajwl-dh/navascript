//! Import-graph discovery for multi-file programs.
//!
//! Imported files are resolved from the root file in dependency order and
//! concatenated into one program for the compiler seams. Phase 8.1 keeps a
//! source map so diagnostics name the originating file; per-module name
//! scoping remains a documented limitation.

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use navascript_ast::{FileId, ItemKind};
use navascript_diagnostics::SourceMap;

/// One file in the compiled import graph.
pub struct LoadedFile {
    pub id: FileId,
    pub path: PathBuf,
    pub source: String,
    pub start: u32,
}

/// The concatenated source plus the map needed to attribute diagnostics.
pub struct Loaded {
    pub source: String,
    pub files: Vec<LoadedFile>,
    pub map: SourceMap,
}

/// Resolve `input` and all of its (transitive) imports into one source text.
pub fn load(input: &Path) -> Result<Loaded, String> {
    let mut visited = HashSet::new();
    let mut ordered = Vec::new();
    visit(input, &mut visited, &mut ordered)?;
    let mut source = String::new();
    let mut files = Vec::new();
    let mut map = SourceMap::new();
    for (index, (path, text)) in ordered.iter().enumerate() {
        if !source.is_empty() {
            source.push('\n');
        }
        let start = source.len() as u32;
        let id = FileId(index as u32);
        let name = path.to_string_lossy().into_owned();
        map.add(id, name, text.clone(), start);
        source.push_str(text);
        files.push(LoadedFile {
            id,
            path: path.clone(),
            source: text.clone(),
            start,
        });
    }
    Ok(Loaded { source, files, map })
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
    let parsed = navascript_parser::parse(&text);
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
        // Extensionless specifiers resolve to `.ns`, the initial source format.
        if dependency.extension().is_none() && !dependency.is_file() {
            dependency.set_extension("ns");
        }
        visit(&dependency, visited, ordered)?;
    }
    ordered.push((canonical, text));
    Ok(())
}
