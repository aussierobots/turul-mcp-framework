use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=../../examples/resource-test-server");
    println!("cargo:rerun-if-changed=../../examples/prompts-test-server");
    println!("cargo:rerun-if-changed=../../examples/tools-test-server");

    // Build the test server binaries that tests depend on
    let servers = vec![
        "resource-test-server",
        "prompts-test-server",
        "tools-test-server",
    ];

    for server in servers {
        println!("cargo:warning=Building test server: {}", server);

        let status = Command::new("cargo")
            .args(&["build", "-p", server])
            .status()
            .expect(&format!("Failed to execute cargo build for {}", server));

        if !status.success() {
            panic!("Failed to build {}", server);
        }
    }
}
