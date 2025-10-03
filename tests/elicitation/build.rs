use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=../../examples/elicitation-server");

    println!("cargo:warning=Building test server: elicitation-server");

    let status = Command::new("cargo")
        .args(&["build", "-p", "elicitation-server"])
        .status()
        .expect("Failed to execute cargo build for elicitation-server");

    if !status.success() {
        panic!("Failed to build elicitation-server");
    }
}
