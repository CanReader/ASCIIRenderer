// Library root — re-exports the crate's modules so that integration tests
// (and any future library consumers) can access them without going through main.rs.
pub mod api;
pub mod examples;
pub mod math;
pub mod model;
pub mod renderer;
