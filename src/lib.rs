//! Template and DSL-facing contracts for Surgeist.
//!
//! This crate owns template-layer contracts. Keep app-authoring concepts typed
//! and reusable, while leaving cross-crate lowering and host integration to the
//! root `surgeist` facade.

#![forbid(unsafe_code)]

/// Crate identity string used by smoke tests and API artifacts.
pub const CRATE_NAME: &str = "surgeist-template";

#[cfg(test)]
mod tests {
    use super::CRATE_NAME;

    #[test]
    fn exposes_crate_identity() {
        assert_eq!(CRATE_NAME, "surgeist-template");
    }
}
