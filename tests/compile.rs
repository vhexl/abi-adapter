//! Cross-crate checks use rustc directly, with no test dependencies or nested
//! Cargo invocation. Each test builds the current library in its own directory.

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

#[test]
fn invalid_return_width_cannot_be_hidden_by_a_consumer_macro() {
    let compiler = Compiler::new();
    let source = include_str!("fixtures/shadow_compile_error.rs");

    // The same fixture with a supported width must compile. An unrelated
    // fixture error therefore cannot make the rejection check pass.
    assert_success(&compiler.compile(
        &source.replace("-> 2 =>", "-> 1 =>"),
        "supported_width",
        "abi_adapter",
        &compiler.library,
    ));

    let output = compiler.compile(source, "invalid_width", "abi_adapter", &compiler.library);
    assert!(
        !output.status.success(),
        "unsupported width must be rejected"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    let errors: Vec<_> = stderr
        .lines()
        .filter_map(|line| line.strip_prefix("error: "))
        .filter(|message| !message.starts_with("aborting due to "))
        .collect();
    assert_eq!(
        errors,
        [
            "ABI returns must be 0 or 1 slots",
            "ABI returns must be 0 or 1 slots",
        ],
        "both return-type and encoding rules must reject the width:\n{stderr}"
    );
    assert!(
        !stderr.lines().any(|line| line.starts_with("error[")),
        "no unrelated compiler error may mask the regression:\n{stderr}"
    );
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
