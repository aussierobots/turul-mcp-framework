use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=../../examples/tools-test-server");

    println!("cargo:warning=Building test server: tools-test-server");

    let status = Command::new("cargo")
        .args(&["build", "-p", "tools-test-server"])
        .status()
        .expect("Failed to execute cargo build for tools-test-server");

    if !status.success() {
        panic!("Failed to build tools-test-server");
    }
}
