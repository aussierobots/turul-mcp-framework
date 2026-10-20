//! 2026-07-28 lane run of the shared derive argument-validation suite. The integration
//! package is pinned to the 2025-11-25 lane, so the default lane would otherwise have
//! no coverage of this contract; one source file serves both lanes.
#![cfg(feature = "protocol-2026-07-28")]

include!("../../../tests/undeclared_tool_arguments_test.rs");
