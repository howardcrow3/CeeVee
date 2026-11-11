//! The seam between CeeVee and an inference engine.
//!
//! The llama.cpp backend (a later PR, behind a cargo feature) implements
//! [`LlmBackend`]; the worker and everything above it only see this trait, so
//! they are tested here with a fake engine.

use std::sync::atomic::AtomicBool;

use ceevee_core::Result;

#[derive(Debug, Clone, PartialEq)]
pub struct GenerateRequest {
    pub prompt: String,
    /// GBNF grammar that constrains the output (compiled from a JSON schema by the task layer).
    pub grammar: Option<String>,
    pub max_tokens: u32,
    pub temperature: f32,
}

impl GenerateRequest {
    pub fn new(prompt: impl Into<String>) -> Self {
        Self {
            prompt: prompt.into(),
            grammar: None,
            max_tokens: 1024,
            temperature: 0.2,
        }
    }
}

/// A local inference engine. Owned by exactly one worker thread, so methods take `&mut self`
/// and need no locking.
pub trait LlmBackend: Send + 'static {
    /// Load the model into memory. Called lazily before the first request and after an idle unload.
    fn load(&mut self) -> Result<()>;
    /// Free the model's memory.
    fn unload(&mut self);
    fn is_loaded(&self) -> bool;
    /// Generate text, calling `on_token` for each piece as it is produced. Implementations should
    /// check `cancel` between tokens and return [`ceevee_core::Error::Cancelled`] when it is set.
    fn generate(
        &mut self,
        req: &GenerateRequest,
        cancel: &AtomicBool,
        on_token: &mut dyn FnMut(&str),
    ) -> Result<String>;
}
