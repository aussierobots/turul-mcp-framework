//! Consolidated example validation test suite.
//!
//! Groups: server/derive/builders/client/http_server/lambda examples,
//! readme examples, working examples validation

#[path = "../server_examples.rs"]
mod server_examples;

#[path = "../derive_examples.rs"]
mod derive_examples;

#[path = "../builders_examples.rs"]
mod builders_examples;

#[path = "../client_examples.rs"]
mod client_examples;

#[path = "../http_server_examples.rs"]
mod http_server_examples;

#[path = "../lambda_examples.rs"]
mod lambda_examples;

#[path = "../lambda_streaming_real.rs"]
mod lambda_streaming_real;

#[path = "../readme_examples.rs"]
mod readme_examples;

#[path = "../working_examples_validation.rs"]
mod working_examples_validation;
