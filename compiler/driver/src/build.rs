//! Native build pipeline: LLVM IR → object → linked Mach-O executable.
//!
//! Tools are discovered from `LLVM_SYS_231_PREFIX` or the Homebrew LLVM prefix;
//! the target triple is queried from `llvm-config` and never hardcoded.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Locate the LLVM installation.
pub fn llvm_prefix() -> PathBuf {
    std::env::var_os("LLVM_SYS_231_PREFIX")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/opt/homebrew/opt/llvm"))
}

fn llvm_tool(name: &str) -> PathBuf {
    llvm_prefix().join("bin").join(name)
}

/// Whether the external native toolchain is available. Tests use this to skip
/// native-only assertions on machines without LLVM installed.
pub fn native_tools_available() -> bool {
    let present = |tool: &Path| {
        if tool.is_absolute() {
            tool.is_file()
        } else {
            which(tool).is_some()
        }
    };
    present(&llvm_tool("llvm-config"))
        && present(&llvm_tool("llc"))
        && present(&PathBuf::from("clang"))
}

/// Ask LLVM for the host target triple.
pub fn host_triple() -> Result<String, String> {
    let output = Command::new(llvm_tool("llvm-config"))
        .arg("--host-target")
        .output()
        .map_err(|error| format!("could not run llvm-config: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "llvm-config --host-target failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

/// Compile textual LLVM IR and the runtime shim into a native executable.
pub fn emit_native(ir: &str, runtime_c: &str, output: &Path, release: bool) -> Result<(), String> {
    if !native_tools_available() {
        return Err(format!(
            "native toolchain not found (LLVM prefix {}); install LLVM with `brew install llvm` or set LLVM_SYS_231_PREFIX",
            llvm_prefix().display()
        ));
    }

    use std::sync::atomic::{AtomicU32, Ordering};
    static BUILD_ID: AtomicU32 = AtomicU32::new(0);
    let workdir = std::env::temp_dir().join(format!(
        "nava-build-{}-{}",
        std::process::id(),
        BUILD_ID.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = fs::remove_dir_all(&workdir);
    fs::create_dir_all(&workdir)
        .map_err(|error| format!("could not create build directory: {error}"))?;

    let result = build_in(&workdir, ir, runtime_c, output, release);
    let _ = fs::remove_dir_all(&workdir);
    result
}

fn build_in(
    workdir: &Path,
    ir: &str,
    runtime_c: &str,
    output: &Path,
    release: bool,
) -> Result<(), String> {
    let ir_path = workdir.join("program.ll");
    let object_path = workdir.join("program.o");
    let runtime_path = workdir.join("runtime.c");
    let runtime_object = workdir.join("runtime.o");
    fs::write(&ir_path, ir).map_err(|error| format!("could not write LLVM IR: {error}"))?;
    fs::write(&runtime_path, runtime_c)
        .map_err(|error| format!("could not write runtime shim: {error}"))?;

    let triple = host_triple()?;
    let opt = if release { "-O3" } else { "-O0" };
    run(
        Command::new(llvm_tool("llc"))
            .args(["-filetype=obj", opt, "-mtriple", &triple])
            .arg("-o")
            .arg(&object_path)
            .arg(&ir_path),
        "llc",
    )?;
    run(
        Command::new("clang")
            .args(["-c", if release { "-O2" } else { "-O0" }])
            .arg(&runtime_path)
            .arg("-o")
            .arg(&runtime_object),
        "clang (runtime)",
    )?;
    run(
        Command::new("clang")
            .arg("-o")
            .arg(output)
            .arg(&object_path)
            .arg(&runtime_object),
        "clang (link)",
    )?;
    Ok(())
}

fn run(command: &mut Command, label: &str) -> Result<(), String> {
    let output = command
        .output()
        .map_err(|error| format!("could not run {label}: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "{label} failed:\n{}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    Ok(())
}

fn which(program: &Path) -> Option<PathBuf> {
    let paths = std::env::var_os("PATH")?;
    std::env::split_paths(&paths)
        .map(|dir| dir.join(program))
        .find(|candidate| candidate.is_file())
}
