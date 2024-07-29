use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::process::Command;

use clap::Parser;

mod translator;
use translator::SyscallTranslator;

#[derive(Parser)]
#[command(version, about)]
struct TranslateOption {
    /// Path to the description file
    #[clap(long, default_value = "desc/test.txt")]
    desc: String,

    /// Path to the constants file
    #[clap(long, default_value = "desc/test.txt.const")]
    r#const: String,

    /// Path to the generated project, must not exist yet
    #[clap(long, default_value = "my-syscalls")]
    project: String,
}

fn main() {
    let opt = TranslateOption::parse();
    let desc_path = Path::new(&opt.desc);
    let const_path = Path::new(&opt.r#const);
    let project_path = Path::new(&opt.project);
    // let output_path = project_path.join("src").join("lib.rs");

    let trans = SyscallTranslator::new(desc_path, const_path);
    let content = trans.translate();
    export_to_crate(project_path, &content);
}

fn export_to_crate(path: &Path, content: &str) {
    assert!(
        Command::new("cargo")
            .arg("new")
            .arg("--lib")
            .arg(path)
            .status()
            .expect("Failed to create project")
            .success(),
        "Failed to create project"
    );

    add_dependency(
        path,
        &["serde", "--no-default-features", "--features=derive"],
    );
    add_dependency(path, &["syscalls"]);
    add_dependency(
        path,
        &[
            "--git",
            "https://github.com/nine-point-eight-p/syscall2struct",
        ],
    );

    let output_path = path.join("src").join("lib.rs");
    export_to_file(&output_path, content);

    assert!(
        Command::new("cargo")
            .current_dir(path)
            .arg("fmt")
            .status()
            .expect("Failed to format code")
            .success(),
        "Failed to format code"
    )
}

fn export_to_file(path: &Path, content: &str) {
    let mut file = File::create(path).unwrap();
    file.write_all(content.as_bytes()).unwrap();
}

fn add_dependency(path: &Path, args: &[&str]) {
    assert!(
        Command::new("cargo")
            .current_dir(path)
            .arg("add")
            .args(args)
            .status()
            .expect("Failed to add crate")
            .success(),
        "Failed to add crate"
    );
}
