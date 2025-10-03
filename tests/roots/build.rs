use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=../../examples/roots-server");

    println!("cargo:warning=Building test server: roots-server");

    let status = Command::new("cargo")
        .args(&["build", "-p", "roots-server"])
        .status()
        .expect("Failed to execute cargo build for roots-server");

    if !status.success() {
        panic!("Failed to build roots-server");
    }
}
