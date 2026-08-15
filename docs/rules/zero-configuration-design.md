# Zero-Configuration Design

Users NEVER specify method strings - framework auto-determines from types:

```rust
// CORRECT
#[derive(McpTool)]
struct Calculator;  // Framework → tools/call

// WRONG
#[mcp_tool(method = "tools/call")]  // NO METHOD STRINGS!
```
