use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=../../examples/resource-test-server");

    println!("cargo:warning=Building test server: resource-test-server");

    let status = Command::new("cargo")
        .args(&["build", "-p", "resource-test-server"])
        .status()
        .expect("Failed to execute cargo build for resource-test-server");

    if !status.success() {
        panic!("Failed to build resource-test-server");
    }
}
