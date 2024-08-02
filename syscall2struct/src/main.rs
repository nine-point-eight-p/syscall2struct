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
    #[clap(long, default_value = "../my-syscalls")]
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

// Generate a command and check if it's successful
macro_rules! cmd {
    ($cmd:expr, $($arg:expr),*; $msg:expr) => {
        assert!(
            Command::new($cmd)
                $(.arg($arg))*
                .status()
                .expect($msg)
                .success(),
            $msg
        );
    };
    // With working directory
    ([$wd:expr] $cmd:expr, $($arg:expr),*; $msg:expr) => {
        assert!(
            Command::new($cmd)
                .current_dir($wd)
                $(.arg($arg))*
                .status()
                .expect($msg)
                .success(),
            $msg
        );
    };
}

fn export_to_crate(path: &Path, content: &str) {
    // Create a directory first to prevent cargo adding the crate to this workspace
    cmd!("mkdir", "-p", path.to_str().unwrap(); "Failed to create project");
    cmd!([path] "cargo", "init", "--lib", path.to_str().unwrap(); "Failed to create project");

    // Add dependencies
    cmd!([path] "cargo", "add",
        "serde",
        "--no-default-features",
        "--features=derive";
        "Failed to add serde"
    );
    cmd!([path] "cargo", "add",
        "syscalls",
        "--no-default-features";
        "Failed to add syscalls"
    );
    cmd!([path] "cargo", "add",
        "--git", "https://github.com/nine-point-eight-p/syscall2struct",
        "syscall2struct-helpers";
        "Failed to add syscall2struct-helpers"
    );

    // Export the content to the file
    let output_path = path.join("src").join("lib.rs");
    export_to_file(&output_path, content);

    // Format the code
    cmd!([path] "cargo", "fmt"; "Failed to format code");
}

fn export_to_file(path: &Path, content: &str) {
    let mut file = File::create(path).unwrap();
    file.write_all(content.as_bytes()).unwrap();
}
