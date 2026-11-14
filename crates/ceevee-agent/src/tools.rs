//! The tool registry (TECHNICAL_PLAN section 6.3).
//!
//! A tool is a Rust function with a name, a description, a JSON Schema for its input and a
//! [`Risk`] level. Agents only see the tools on their allow-list, and every call is checked
//! against the schema before it runs. The policy engine ([`crate::policy`]) decides what a risk
//! level may do.

use std::collections::BTreeMap;
use std::future::Future;
use std::pin::Pin;

use ceevee_ai::validate::validate;
use ceevee_core::{Error, Result};
use serde_json::Value;

/// How much a tool call can change, from least to most.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Risk {
    /// Looks something up. Runs freely.
    Read,
    /// Produces something reversible and visible (a new document version, a saved posting).
    Draft,
    /// Suggests a change to the user's data. Always becomes a proposal the user accepts.
    Propose,
    /// Does something in the world (submits, sends). Needs a user click unless policy allows it.
    Act,
}

#[derive(Debug, Clone)]
pub struct ToolSpec {
    /// Dotted name such as `search.get_posting`; the part before the dot is the group.
    pub name: &'static str,
    pub description: &'static str,
    pub risk: Risk,
    /// JSON Schema for the input object.
    pub input_schema: Value,
    /// The policy action an `Act` tool maps to, such as `apply.submit`. `None` for other risks.
    pub action: Option<&'static str>,
    /// The result contains text written by someone else (a posting, an email, a web page). Once a
    /// run has seen such a result, its act-level calls are queued for the user.
    pub untrusted_output: bool,
}

pub type ToolFuture = Pin<Box<dyn Future<Output = Result<Value>> + Send>>;

pub trait Tool: Send + Sync {
    fn spec(&self) -> &ToolSpec;
    fn call(&self, input: Value) -> ToolFuture;
}

/// A tool built from a closure, for small tools and tests.
pub struct FnTool<F> {
    spec: ToolSpec,
    f: F,
}

impl<F> FnTool<F>
where
    F: Fn(Value) -> ToolFuture + Send + Sync,
{
    pub fn new(spec: ToolSpec, f: F) -> Self {
        Self { spec, f }
    }
}

impl<F> Tool for FnTool<F>
where
    F: Fn(Value) -> ToolFuture + Send + Sync,
{
    fn spec(&self) -> &ToolSpec {
        &self.spec
    }
    fn call(&self, input: Value) -> ToolFuture {
        (self.f)(input)
    }
}

#[derive(Default)]
pub struct Registry {
    tools: BTreeMap<&'static str, Box<dyn Tool>>,
}

/// Does `name` match an allow-list entry? An entry is an exact name or a group such as `search.*`.
pub fn allowed(entry: &str, name: &str) -> bool {
    match entry.strip_suffix(".*") {
        Some(group) => name
            .strip_prefix(group)
            .is_some_and(|rest| rest.starts_with('.')),
        None => entry == name,
    }
}

impl Registry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, tool: impl Tool + 'static) -> Result<()> {
        let spec = tool.spec();
        if !spec.name.contains('.') {
            return Err(Error::Invalid(format!(
                "tool name \"{}\" needs a group, like search.get_posting",
                spec.name
            )));
        }
        if spec.risk == Risk::Act && spec.action.is_none() {
            return Err(Error::Invalid(format!(
                "act-level tool {} must name its policy action",
                spec.name
            )));
        }
        let name = spec.name;
        if self.tools.contains_key(name) {
            return Err(Error::Invalid(format!("tool {name} is already registered")));
        }
        self.tools.insert(name, Box::new(tool));
        Ok(())
    }

    pub fn get(&self, name: &str) -> Option<&dyn Tool> {
        self.tools.get(name).map(|t| t.as_ref())
    }

    /// The specs an agent may use, in name order.
    pub fn specs_for<'a>(&'a self, allow: &[&str]) -> Vec<&'a ToolSpec> {
        self.tools
            .values()
            .map(|t| t.spec())
            .filter(|s| allow.iter().any(|a| allowed(a, s.name)))
            .collect()
    }

    /// Check `input` against the tool's schema, then run it. The caller has already asked the
    /// policy engine whether the call may run.
    pub async fn call(&self, name: &str, input: Value) -> Result<Value> {
        let tool = self
            .get(name)
            .ok_or_else(|| Error::NotFound(format!("tool {name}")))?;
        validate(&tool.spec().input_schema, &input)
            .map_err(|e| Error::Invalid(format!("input for {name} rejected: {e}")))?;
        tool.call(input).await
    }
}

#[cfg(test)]
mod tests;
