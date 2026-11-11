//! Agent harness: loop, tool registry, policy, approvals, memory, scheduler.
//!
//! Only the scheduler and notification settings exist so far; the rest lands with
//! the agent milestone
//! (see docs/TECHNICAL_PLAN.md, section 6).

pub mod notifications;
pub mod scheduler;
