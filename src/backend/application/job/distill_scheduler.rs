use std::time::Duration;

use crate::backend::application::memory_manager::{MemoryManager, MemoryManagerError};
use crate::backend::application::port::memory_port::MemoryDistiller;

#[derive(Debug, Clone, Copy)]
pub struct DistillPolicy {
    /// How often the timer wakes to check.
    pub interval: Duration,
    /// Distill once this many messages are uncovered by any summary.
    pub threshold: u64,
}

impl Default for DistillPolicy {
    fn default() -> Self {
        Self {
            interval: Duration::from_secs(5 * 60),
            threshold: 80,
        }
    }
}

/// Owns the manager and the policy; runs the periodic check.
pub struct DistillScheduler<D: MemoryDistiller> {
    manager: MemoryManager<D>,
    policy: DistillPolicy,
}

impl<D: MemoryDistiller> DistillScheduler<D> {
    pub fn new(manager: MemoryManager<D>, policy: DistillPolicy) -> Self {
        Self { manager, policy }
    }

    pub async fn run(&self) {}

    async fn should_distill(&self) -> Result<bool, MemoryManagerError> {
        todo!("count uncovered messages, compare to policy.threshold")
    }
}
