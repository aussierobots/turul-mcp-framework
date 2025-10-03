use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=../../examples/prompts-test-server");

    println!("cargo:warning=Building test server: prompts-test-server");

    let status = Command::new("cargo")
        .args(&["build", "-p", "prompts-test-server"])
        .status()
        .expect("Failed to execute cargo build for prompts-test-server");

    if !status.success() {
        panic!("Failed to build prompts-test-server");
    }
}
