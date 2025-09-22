# Documentation Testing Strategy

## Philosophy

**Documentation correctness is critical** - users' first experience with broken examples creates immediate frustration and reduces trust in the framework.

## Three-Tier Testing Strategy

### 1. **Critical API Examples** (Normal doctests - always run)
- Main entry points (`McpServer::builder()`)
- Core macros (`#[mcp_tool]`, `#[mcp_resource]`)
- Basic usage patterns
- **Criteria**: Must compile and demonstrate correct API usage
- **Performance**: Fast, < 1 second

### 2. **Syntax Validation** (`no_run` - compile only)
- Complex examples that require external dependencies
- Full application examples
- Integration patterns
- **Criteria**: Must compile but don't need to execute
- **Performance**: Fast, < 3 seconds

### 3. **Full Integration** (`ignore` - manual/CI only)
- Examples requiring servers, databases, network
- Performance-sensitive examples
- Long-running examples
- **Criteria**: Full end-to-end functionality
- **Performance**: Slower, run on demand

## Implementation Guidelines

### Enable Critical Tests
```rust
/// ```rust
/// use turul_mcp_derive::mcp_tool;
/// #[mcp_tool(name = "add")]
/// async fn add(a: f64, b: f64) -> f64 { a + b }
/// ```
```

### Use `no_run` for Complex Examples
```rust
/// ```rust,no_run
/// let server = McpServer::builder()
///     .name("my-server")
///     .tool_fn(my_tool)
///     .bind_address("127.0.0.1:8080".parse()?)
///     .build()?;
/// server.run().await
/// ```
```

### Keep `ignore` for Integration
```rust
/// ```rust,ignore
/// // Full server setup with database, external services
/// let server = setup_production_server().await?;
/// server.run_with_graceful_shutdown().await
/// ```
```

## CI Integration

Add to CI pipeline:
```bash
# Always run critical tests
cargo test --doc

# Weekly comprehensive test
cargo test --doc -- --ignored
```

## Benefits

1. **User Trust**: Working examples on first try
2. **Maintenance**: Automatic detection of API breaks
3. **Performance**: Fast default tests, comprehensive on-demand
4. **Developer Experience**: Clear feedback on documentation quality

## Current Status

- ✅ **Fixed broken doctest** in `builder.rs`
- ✅ **Enabled critical examples** in `turul-mcp-derive`
- ✅ **All 30+ ignored tests pass** when run manually
- 📋 **Next**: Systematically review and categorize all doctests