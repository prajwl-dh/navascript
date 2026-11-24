//! End-to-end tests: compile a `.ns` file to a native executable and run it.

use std::path::{Path, PathBuf};
use std::process::Command;

fn output_path(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("navascript-native-{}-{name}", std::process::id()))
}

fn compile(source: &str, name: &str, extra: &[&str]) -> Option<PathBuf> {
    if !navascript_driver::build::native_tools_available() {
        eprintln!("skipping native test: LLVM toolchain not available");
        return None;
    }
    let source = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples")
        .join(source);
    let output = output_path(name);
    let status = Command::new(env!("CARGO_BIN_EXE_navascript"))
        .arg("build")
        .arg(&source)
        .arg("-o")
        .arg(&output)
        .args(extra)
        .status()
        .expect("run navascript build");
    assert!(status.success(), "navascript build failed for {name}");
    Some(output)
}

#[test]
fn fibonacci_prints_55_natively() {
    let Some(binary) = compile("fibonacci.ns", "fib", &[]) else {
        return;
    };
    let output = Command::new(&binary).output().expect("run fibonacci");
    assert!(output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        "55",
        "native fibonacci output"
    );
    let _ = std::fs::remove_file(binary);
}

#[test]
fn multiple_functions_with_distinct_value_id_ranges_compile() {
    let Some(binary) = compile("multifunction.ns", "multi", &[]) else {
        return;
    };
    let output = Command::new(&binary)
        .output()
        .expect("run multifunction program");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "5");
    let _ = std::fs::remove_file(binary);
}

#[test]
fn booleans_print_as_true_or_false() {
    let Some(binary) = compile("booleans.ns", "booleans", &[]) else {
        return;
    };
    let output = Command::new(&binary)
        .output()
        .expect("run booleans program");
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "false\ntrue\nfalse\ntrue\n"
    );
    let _ = std::fs::remove_file(binary);
}

#[test]
fn numbers_print_without_precision_loss() {
    let Some(binary) = compile("numbers.ns", "numbers", &[]) else {
        return;
    };
    let output = Command::new(&binary).output().expect("run numbers program");
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "1234567.89\n0.00001234\n1.5\n"
    );
    let _ = std::fs::remove_file(binary);
}

#[test]
fn strings_run_natively() {
    let Some(binary) = compile("strings.ns", "strings", &[]) else {
        return;
    };
    let output = Command::new(&binary).output().expect("run strings program");
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "hello world\n11\nh\ntrue\n"
    );
    let _ = std::fs::remove_file(binary);
}

#[test]
fn arrays_run_natively() {
    let Some(binary) = compile("arrays.ns", "arrays", &[]) else {
        return;
    };
    let output = Command::new(&binary).output().expect("run arrays program");
    assert_eq!(String::from_utf8_lossy(&output.stdout), "1\n42\n3\n");
    let _ = std::fs::remove_file(binary);
}

#[test]
fn objects_run_natively() {
    let Some(binary) = compile("objects.ns", "objects", &[]) else {
        return;
    };
    let output = Command::new(&binary).output().expect("run objects program");
    assert_eq!(String::from_utf8_lossy(&output.stdout), "3\n9\n");
    let _ = std::fs::remove_file(binary);
}

#[test]
fn dynamic_indices_and_string_coercion_run_natively() {
    let Some(binary) = compile("dynamic_access.ns", "dynamic", &[]) else {
        return;
    };
    let output = Command::new(&binary)
        .output()
        .expect("run dynamic_access program");
    assert_eq!(String::from_utf8_lossy(&output.stdout), "20\nb\na1\n");
    let _ = std::fs::remove_file(binary);
}

#[test]
fn object_fields_keep_their_declared_order() {
    let Some(binary) = compile("object_fields.ns", "object-fields", &[]) else {
        return;
    };
    let output = Command::new(&binary)
        .output()
        .expect("run object_fields program");
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "navascript\n2\ntrue\n"
    );
    let _ = std::fs::remove_file(binary);
}

#[test]
fn out_of_bounds_index_is_a_runtime_error() {
    let Some(binary) = compile("runtime_error.ns", "runtime-error", &[]) else {
        return;
    };
    let output = Command::new(&binary)
        .output()
        .expect("run runtime_error program");
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("array index out of bounds"), "{stderr}");
    let _ = std::fs::remove_file(binary);
}

#[test]
fn loops_run_natively() {
    let Some(binary) = compile("loops.ns", "loops", &[]) else {
        return;
    };
    let output = Command::new(&binary).output().expect("run loops program");
    assert_eq!(String::from_utf8_lossy(&output.stdout), "15\n24\n3\n7\n");
    let _ = std::fs::remove_file(binary);
}

#[test]
fn for_of_runs_natively() {
    let Some(binary) = compile("for_of.ns", "for-of", &[]) else {
        return;
    };
    let output = Command::new(&binary).output().expect("run for_of program");
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "60\nabc\n10\n20\n30\n"
    );
    let _ = std::fs::remove_file(binary);
}

#[test]
fn break_and_continue_run_natively() {
    let Some(binary) = compile("break_continue.ns", "break-continue", &[]) else {
        return;
    };
    let output = Command::new(&binary)
        .output()
        .expect("run break/continue program");
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "4\n9\n3\n2\n4\n2\n3\n"
    );
    let _ = std::fs::remove_file(binary);
}

#[test]
fn optional_chaining_runs_natively() {
    let Some(binary) = compile("optional.ns", "optional", &[]) else {
        return;
    };
    let output = Command::new(&binary)
        .output()
        .expect("run optional chaining program");
    assert_eq!(String::from_utf8_lossy(&output.stdout), "navascript\nb\n");
    let _ = std::fs::remove_file(binary);
}

#[test]
fn arrows_run_natively() {
    let Some(binary) = compile("arrows.ns", "arrows", &[]) else {
        return;
    };
    let output = Command::new(&binary).output().expect("run arrows program");
    assert_eq!(String::from_utf8_lossy(&output.stdout), "5\n25\npositive\n");
    let _ = std::fs::remove_file(binary);
}

#[test]
fn new_operators_run_natively() {
    let Some(binary) = compile("operators.ns", "operators", &[]) else {
        return;
    };
    let output = Command::new(&binary)
        .output()
        .expect("run operators program");
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "255\n10\n15\n1000\n15\n12\n24\n6\n0\n8\n3\n3\n4\n3\n3\n2\nhi!\nABC\n3\nyes\n"
    );
    let _ = std::fs::remove_file(binary);
}

#[test]
fn bigints_run_natively() {
    let Some(binary) = compile("bigint.ns", "bigint", &[]) else {
        return;
    };
    let output = Command::new(&binary).output().expect("run bigint program");
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "123\n255\n1000\n133\n113\n246\n12\n3\n256\ntrue\ntrue\n-123\n2\n"
    );
    let _ = std::fs::remove_file(binary);
}

#[test]
fn transitive_extensionless_imports_resolve() {
    if !navascript_driver::build::native_tools_available() {
        return;
    }
    let dir = std::env::temp_dir().join(format!("navascript-modules-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    std::fs::write(
        dir.join("a.ns"),
        "export function offset(): number { return 5; }",
    )
    .expect("write a.ns");
    std::fs::write(
        dir.join("b.ns"),
        "import { offset } from \"./a\";\nexport function addOffset(x: number): number { return x + offset(); }",
    )
    .expect("write b.ns");
    let main = dir.join("main.ns");
    std::fs::write(
        &main,
        "import { offset } from \"./a\";\nimport { addOffset } from \"./b\";\nconsole.log(offset());\nconsole.log(addOffset(10));",
    )
    .expect("write main.ns");
    let output = dir.join("main-bin");
    let status = Command::new(env!("CARGO_BIN_EXE_navascript"))
        .arg("build")
        .arg(&main)
        .arg("-o")
        .arg(&output)
        .status()
        .expect("run navascript build");
    assert!(
        status.success(),
        "navascript build failed for transitive imports"
    );
    let run = Command::new(&output).output().expect("run module program");
    assert_eq!(String::from_utf8_lossy(&run.stdout), "5\n15\n");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn modules_resolve_and_link_natively() {
    let Some(binary) = compile("modules.ns", "modules", &[]) else {
        return;
    };
    let output = Command::new(&binary).output().expect("run modules program");
    assert_eq!(String::from_utf8_lossy(&output.stdout), "42\n3\n");
    let _ = std::fs::remove_file(binary);
}

#[test]
fn unions_and_intersections_run_natively() {
    let Some(binary) = compile("unions.ns", "unions", &[]) else {
        return;
    };
    let output = Command::new(&binary).output().expect("run unions program");
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "10\nnull\nx\n2\ntrue\n"
    );
    let _ = std::fs::remove_file(binary);
}

#[test]
fn classes_and_inheritance_run_natively() {
    let Some(binary) = compile("classes.ns", "classes", &[]) else {
        return;
    };
    let output = Command::new(&binary).output().expect("run classes program");
    assert_eq!(String::from_utf8_lossy(&output.stdout), "5\n9\nq\n3\n30\n");
    let _ = std::fs::remove_file(binary);
}

#[test]
fn generics_run_natively() {
    let Some(binary) = compile("generics.ns", "generics", &[]) else {
        return;
    };
    let output = Command::new(&binary)
        .output()
        .expect("run generics program");
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "42\nnavascript\ntrue\n1\na\n7\n"
    );
    let _ = std::fs::remove_file(binary);
}

#[test]
fn optional_calls_run_natively() {
    let Some(binary) = compile("optional_call.ns", "optional-call", &[]) else {
        return;
    };
    let output = Command::new(&binary)
        .output()
        .expect("run optional call program");
    assert_eq!(String::from_utf8_lossy(&output.stdout), "hi navascript\n");
    let _ = std::fs::remove_file(binary);
}

#[test]
fn closures_and_higher_order_functions_run_natively() {
    let Some(binary) = compile("closures.ns", "closures", &[]) else {
        return;
    };
    let output = Command::new(&binary)
        .output()
        .expect("run closures program");
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "15\n42\n8\n7\n13\n1\n2\n2\n2\n6\n7\n10\n"
    );
    let _ = std::fs::remove_file(binary);
}

#[test]
fn async_await_runs_natively() {
    let Some(binary) = compile("async_await.ns", "async-await", &[]) else {
        return;
    };
    let output = Command::new(&binary).output().expect("run async program");
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "10\n6\nhello\n2\n4\n"
    );
    let _ = std::fs::remove_file(binary);
}

#[test]
fn exceptions_run_natively() {
    let Some(binary) = compile("exceptions.ns", "exceptions", &[]) else {
        return;
    };
    let output = Command::new(&binary)
        .output()
        .expect("run exceptions program");
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "bad value\ncleanup\nafter\ninner\nouter continues\nnormal\nnormal cleanup\n"
    );
    let _ = std::fs::remove_file(binary);
}

#[test]
fn release_mode_prints_the_same_result() {
    let Some(binary) = compile("fibonacci.ns", "fib-release", &["--release"]) else {
        return;
    };
    let output = Command::new(&binary).output().expect("run fibonacci");
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "55");
    let _ = std::fs::remove_file(binary);
}

#[test]
fn emit_ir_prints_llvm_ir_without_linking() {
    if !navascript_driver::build::native_tools_available() {
        return;
    }
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/fibonacci.ns");
    let output = Command::new(env!("CARGO_BIN_EXE_navascript"))
        .args(["build", "--emit-ir"])
        .arg(source)
        .output()
        .expect("run navascript build --emit-ir");
    assert!(output.status.success());
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(
        text.contains("define double @navascript_fibonacci"),
        "{text}"
    );
    assert!(text.contains("target triple"), "{text}");
}
