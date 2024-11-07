use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::process::Command;

use clap::Parser;
use codegen::Scope;

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
    #[clap(long, default_value = "../my-harness")]
    package: String,
}

fn main() {
    let opt = TranslateOption::parse();
    let desc_path = Path::new(&opt.desc);
    let const_path = Path::new(&opt.r#const);
    let project_path = Path::new(&opt.package);

    let main = generate_main();
    let translator = SyscallTranslator::new(desc_path, const_path);
    let syscall = translator.translate();
    make_crate(project_path, &main, &syscall);
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

fn make_crate(path: &Path, main: &str, syscall: &str) {
    // Create a directory first to prevent cargo adding the crate to this workspace
    cmd!("mkdir", "-p", path.to_str().unwrap(); "Failed to create project");
    cmd!([path] "cargo", "init", path.to_str().unwrap(); "Failed to create project");

    // Add dependencies
    cmd!([path] "cargo", "add",
        "serde",
        "syscalls",
        "uuid",
        "--no-default-features",
        "--features", "uuid/serde";
        "Failed to add dependencies"
    );
    cmd!([path] "cargo", "add",
        "heapless",
        "postcard",
        "--features", "heapless/serde";
        "Failed to add dependencies"
    );
    cmd!([path] "cargo", "add",
        "--git", "https://github.com/nine-point-eight-p/libafl_qemu_cmd",
        "libafl_qemu_cmd";
        "Failed to add dependencies"
    );
    cmd!([path] "cargo", "add",
        "--git", "https://github.com/nine-point-eight-p/syscall2struct",
        "syscall2struct-derive",
        "syscall2struct-helpers";
        "Failed to add dependencies"
    );

    // Export the content to the file
    let main_path = path.join("src").join("main.rs");
    export_to_file(&main_path, main);
    let syscall_path = path.join("src").join("syscall.rs");
    export_to_file(&syscall_path, syscall);

    // Format the code
    cmd!([path] "cargo", "fmt"; "Failed to format code");
}

fn export_to_file(path: &Path, content: &str) {
    let mut file = File::create(path).unwrap();
    file.write_all(content.as_bytes()).unwrap();
}

fn generate_main() -> String {
    let mut s = Scope::new();
    s.import("syscall", "*");
    s.import("libafl_qemu_cmd", "*");
    s.import("postcard", "take_from_bytes");
    s.import("syscall2struct_helpers", "*");
    s.raw("mod syscall;");
    s.new_fn("main")
        .line("// Add harness logic here")
        .line("todo!()");
    format!("#![no_std]\n{}", s.to_string())
}
