//! Compile small programs against this library to check accepted and rejected
//! declarations. Each test calls rustc directly in a separate temporary
//! directory, without extra dependencies or another Cargo process.

use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static NEXT_DIRECTORY: AtomicUsize = AtomicUsize::new(0);

struct Compiler {
    directory: PathBuf,
    rustc: OsString,
    library: PathBuf,
}

impl Compiler {
    fn new() -> Self {
        let directory = std::env::temp_dir().join(format!(
            "abi-adapter-compile-{}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system clock must follow the Unix epoch")
                .as_nanos(),
            NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed),
        ));
        fs::create_dir(&directory).expect("create isolated compiler directory");
        let compiler = Self {
            library: directory.join("libabi_adapter.rlib"),
            directory,
            rustc: std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into()),
        };
        let output = compiler
            .command()
            .args(["--crate-name", "abi_adapter", "--crate-type", "rlib"])
            .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("src/lib.rs"))
            .arg("-o")
            .arg(&compiler.library)
            .output()
            .expect("run rustc for the current library");
        assert_success(&output);
        compiler
    }

    fn command(&self) -> Command {
        let mut command = Command::new(&self.rustc);
        command.args(["--edition=2024", "--color=never"]);
        command
    }

    fn compile(&self, source: &str, name: &str, dependency: &str, library: &Path) -> Output {
        let source_path = self.directory.join(format!("{name}.rs"));
        fs::write(&source_path, source).expect("write consumer fixture");
        let mut external = OsString::from(format!("{dependency}="));
        external.push(library);
        let mut search = OsString::from("dependency=");
        search.push(&self.directory);
        self.command()
            .args(["--crate-name", name, "--crate-type", "rlib"])
            .arg(&source_path)
            .arg("--extern")
            .arg(external)
            .arg("-L")
            .arg(search)
            .arg("-o")
            .arg(self.directory.join(format!("lib{name}.rlib")))
            .output()
            .expect("run rustc for the consumer fixture")
    }
}

impl Drop for Compiler {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.directory);
    }
}

fn assert_success(output: &Output) {
    assert!(
        output.status.success(),
        "fixture must compile:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn assert_rejected(output: &Output, code: Option<&str>, message: &str) {
    assert!(!output.status.success(), "fixture must be rejected");
    let stderr = String::from_utf8_lossy(&output.stderr);
    let errors: Vec<_> = stderr
        .lines()
        .filter(|line| line.starts_with("error: ") || line.starts_with("error["))
        .filter(|line| !line.starts_with("error: aborting due to "))
        .collect();
    let prefix = code.map_or_else(|| "error: ".to_owned(), |code| format!("error[{code}]: "));
    assert!(
        !errors.is_empty() && errors.iter().all(|line| line.starts_with(&prefix)),
        "only the expected error kind may be reported:\n{stderr}"
    );
    assert!(
        errors.iter().any(|line| line.contains(message)),
        "expected error message {message:?}:\n{stderr}"
    );
    if code.is_none() {
        assert_eq!(
            errors.len(),
            1,
            "one invalid declaration must produce one main error:\n{stderr}"
        );
    }
}

fn check_correction(
    compiler: &Compiler,
    source: &str,
    wrong: &str,
    corrected: &str,
    code: Option<&str>,
    message: &str,
) {
    assert_ne!(wrong, corrected);
    assert!(source.contains(wrong), "correction must change the fixture");
    // A nearby valid case prevents an unrelated fixture error from making the
    // rejection check pass. Check only useful message fragments, not snapshots.
    assert_success(&compiler.compile(
        &source.replace(wrong, corrected),
        "corrected",
        "abi_adapter",
        &compiler.library,
    ));
    assert_rejected(
        &compiler.compile(source, "rejected", "abi_adapter", &compiler.library),
        code,
        message,
    );
}

#[test]
fn invalid_return_width_cannot_be_hidden_by_a_consumer_macro() {
    let compiler = Compiler::new();
    let source = include_str!("fixtures/shadow_compile_error.rs");
    check_correction(
        &compiler,
        source,
        "-> 2 =>",
        "-> 1 =>",
        None,
        "ABI returns must be 0 or 1 slots; use -> 0 or -> 1",
    );
}

#[test]
fn common_syntax_errors_explain_the_correction() {
    let compiler = Compiler::new();
    for form in ["host", "guest(\"boundary.v1\")"] {
        for groups in ["[], [b], [c]", "[a], [], [c]", "[a], [b], []"] {
            check_correction(
                &compiler,
                &format!(
                    "pub fn business(a: u32, b: u32, c: u32) -> u32 {{ a + b + c }}
                    abi_adapter::abi! {{ {form} pub fn boundary({groups}) -> 1 => crate::business; }}"
                ),
                "[]",
                "[value]",
                None,
                "ABI slot groups cannot be empty; list at least one slot in each group",
            );
        }
        check_correction(
            &compiler,
            &format!(
                "pub fn business() {{}}
                abi_adapter::abi! {{ {form} pub fn boundary([]) -> 0 => crate::business; }}"
            ),
            "boundary([])",
            "boundary()",
            None,
            "omit all groups for a function with no arguments",
        );
        check_correction(
            &compiler,
            &format!(
                "pub fn business() {{}}
                abi_adapter::abi! {{ {form} pub fn boundary() => crate::business; }}"
            ),
            "boundary() =>",
            "boundary() -> 0 =>",
            None,
            "ABI return width is missing; add -> 0 or -> 1 before =>",
        );
    }
    check_correction(
        &compiler,
        "pub fn business(cx: &mut u32, value: u32) -> u32 { *cx += value; *cx }
        abi_adapter::abi! { host pub fn boundary(ctx cx: &mut u32; []) -> 1 => crate::business; }",
        "; []",
        "; [value]",
        None,
        "ABI slot groups cannot be empty; list at least one slot in each group",
    );
    check_correction(
        &compiler,
        "pub fn business(cx: &mut u32) -> u32 { *cx }
        abi_adapter::abi! { guest(\"boundary.v1\") pub fn boundary(ctx cx: &mut u32;) -> 1 => crate::business; }",
        "guest(\"boundary.v1\")",
        "host",
        None,
        "ABI guest functions do not accept ctx; use host for a context parameter",
    );
}

#[test]
fn forwarded_width_keeps_its_documented_matching_rules() {
    let compiler = Compiler::new();
    for width in ["0", "1"] {
        let result = if width == "0" { "()" } else { "u32" };
        check_correction(
            &compiler,
            &format!(
                "pub fn business() -> {result} {{ Default::default() }}
                macro_rules! boundary {{
                    ($width:literal) => {{
                        abi_adapter::abi! {{ host pub fn boundary() -> $width => crate::business; }}
                    }};
                }}
                boundary!({width});"
            ),
            "$width:literal",
            "$width:tt",
            None,
            "ABI returns must be 0 or 1 slots; use -> 0 or -> 1",
        );
    }
}

#[test]
fn conversions_and_argument_counts_report_the_expected_errors() {
    let compiler = Compiler::new();
    for (source, wrong, corrected, code, message) in [
        (
            "pub fn business(value: u32) -> u32 { value }
            abi_adapter::abi! { host pub fn boundary([x, y]) -> 1 => crate::business; }",
            "[x, y]",
            "[x]",
            "E0277",
            "u32: FromSlots<2>",
        ),
        (
            "pub fn business(value: bool) -> u32 { u32::from(value) }
            abi_adapter::abi! { host pub fn boundary([x]) -> 1 => crate::business; }",
            "value: bool",
            "value: u32",
            "E0277",
            "bool: FromSlots<1>",
        ),
        (
            "pub fn business() -> bool { Default::default() }
            abi_adapter::abi! { host pub fn boundary() -> 1 => crate::business; }",
            "-> bool",
            "-> u32",
            "E0277",
            "bool: IntoSlots<1>",
        ),
        (
            "pub fn business() -> [u32; 2] { Default::default() }
            abi_adapter::abi! { host pub fn boundary() -> 1 => crate::business; }",
            "[u32; 2]",
            "[u32; 1]",
            "E0277",
            "[u32; 2]: IntoSlots<1>",
        ),
        (
            "pub fn business() -> u32 { 7 }
            abi_adapter::abi! { host pub fn boundary() -> 0 => crate::business; }",
            "-> 0 =>",
            "-> 1 =>",
            "E0277",
            "u32: IntoSlots<0>",
        ),
        (
            "pub fn business(left: u32, right: u32) -> u32 { left + right }
            abi_adapter::abi! { host pub fn boundary([x]) -> 1 => crate::business; }",
            "boundary([x])",
            "boundary([x], [y])",
            "E0061",
            "takes 2 arguments",
        ),
    ] {
        check_correction(&compiler, source, wrong, corrected, Some(code), message);
    }
}

#[test]
fn ambiguous_targets_and_name_conflicts_report_the_expected_errors() {
    let compiler = Compiler::new();
    for (source, wrong, corrected, code, message) in [
        (
            "pub fn business<T: abi_adapter::IntoSlots<1>>(value: T) -> T { value }
            abi_adapter::abi! { host pub fn boundary([x]) -> 1 => crate::business; }",
            "=> crate::business;", "=> crate::business::<u32>;", "E0283", "type annotations needed",
        ),
        (
            "pub fn business(value: impl abi_adapter::IntoSlots<1>) -> u32 { value.into_slots()[0] }
            pub fn concrete(value: u32) -> u32 { business(value) }
            abi_adapter::abi! { host pub fn boundary([x]) -> 1 => crate::business; }",
            "=> crate::business;", "=> crate::concrete;", "E0283", "type annotations needed",
        ),
        (
            "pub fn business(left: u32, right: u32) -> u32 { left + right }
            abi_adapter::abi! { host pub fn boundary([x], [x]) -> 1 => crate::business; }",
            "[x], [x]", "[x], [y]", "E0415", "identifier `x` is bound more than once",
        ),
        (
            "pub fn business(value: u32) -> u32 { value }
            abi_adapter::abi! { host pub fn boundary([business]) -> 1 => business; }",
            "=> business;", "=> crate::business;", "E0618", "expected function",
        ),
        (
            "const x: u32 = 1;
            pub fn business(value: u32) -> u32 { value }
            abi_adapter::abi! { host pub fn boundary([x]) -> 1 => crate::business; }",
            "boundary([x])", "boundary([value])", "E0005", "refutable pattern in function argument",
        ),
    ] {
        check_correction(&compiler, source, wrong, corrected, Some(code), message);
    }
}

#[test]
fn disabled_declarations_skip_diagnostics_and_target_resolution() {
    let compiler = Compiler::new();
    for attribute in ["#[cfg(any())]", "#[cfg_attr(all(), cfg(any()))]"] {
        for form in ["host", "guest(\"disabled.v1\")"] {
            for declaration in [
                "fn boundary([value]) -> 2 => crate::missing;",
                "fn boundary([]) -> 1 => crate::missing;",
                "fn boundary() => crate::missing;",
                "fn boundary(ctx cx: Missing;) -> 2 => crate::missing;",
                "fn boundary(ctx cx: Missing; []) -> 1 => crate::missing;",
                "fn boundary(ctx cx: Missing;) => crate::missing;",
            ] {
                assert_success(&compiler.compile(
                    &format!(
                        "#![no_std]\nabi_adapter::abi! {{ {form} {attribute} {declaration} }}"
                    ),
                    "disabled",
                    "abi_adapter",
                    &compiler.library,
                ));
            }
        }
    }
}

#[test]
fn renamed_dependency_and_facade_work_without_std() {
    let compiler = Compiler::new();
    assert_success(&compiler.compile(
        include_str!("fixtures/renamed_dependency.rs"),
        "renamed_consumer",
        "adapter",
        &compiler.library,
    ));

    assert_success(&compiler.compile(
        "#![no_std]\npub use renamed_adapter::{FromSlots, IntoSlots, abi};\n",
        "adapter_facade",
        "renamed_adapter",
        &compiler.library,
    ));
    // This consumer only depends on the facade; it has no direct --extern
    // entry for abi-adapter. $crate must still find the underlying traits.
    assert_success(&compiler.compile(
        include_str!("fixtures/facade_consumer.rs"),
        "facade_consumer",
        "facade",
        &compiler.directory.join("libadapter_facade.rlib"),
    ));
}
