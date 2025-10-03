use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=../../examples/sampling-server");

    println!("cargo:warning=Building test server: sampling-server");

    let status = Command::new("cargo")
        .args(&["build", "-p", "sampling-server"])
        .status()
        .expect("Failed to execute cargo build for sampling-server");

    if !status.success() {
        panic!("Failed to build sampling-server");
    }
}
