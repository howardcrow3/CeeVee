//! Agent harness: loop, tool registry, policy, approvals, memory, scheduler.
//!
//! So far: the scheduler, notification settings, the tool registry and the policy engine.
//! The loop, approvals queue and memory land in the following PRs
//! (see docs/TECHNICAL_PLAN.md, section 6).

pub mod approvals;
pub mod notifications;
pub mod policy;
pub mod power;
pub mod runner;
pub mod scheduler;
pub mod tools;
