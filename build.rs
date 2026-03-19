use std::process::Command;
use std::env;
use std::path::PathBuf;

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let out_path = PathBuf::from(&out_dir);

    // Compile multiboot.s assembly
    let output = Command::new("as")
        .arg("--64")
        .arg("-o")
        .arg(out_path.join("multiboot.o"))
        .arg("src/arch/x86_64/multiboot.s")
        .output()
        .expect("Failed to assemble multiboot.s");

    if !output.status.success() {
        eprintln!("Assembly failed: {}", String::from_utf8_lossy(&output.stderr));
        panic!("Failed to compile multiboot.s");
    }

    // Link the object file
    println!("cargo:rustc-link-arg={}", out_path.join("multiboot.o").display());
}
