//! Decides *when* to distill. A background timer wakes every `interval` and
//! checks how many messages are not yet covered by a summary. Once that count
//! crosses `threshold`, it asks the `MemoryManager` to compress.
//!
//! Scaffold only — every body is `todo!()`. The policy numbers and the backlog
//! count are yours to write.

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

    /// Run forever: wake every `policy.interval`, count the backlog, distill if
    /// it crossed the threshold.
    /// TODO: spawn this on your async runtime; decide how it is cancelled when
    /// the adventure closes.
    pub async fn run(&self) {
        // TODO:
        //   loop {
        //       tokio::time::sleep(self.policy.interval).await;
        //       if self.should_distill().await.unwrap_or(false) {
        //           if let Err(e) = self.manager.compress().await { log::warn!(...) }
        //       }
        //   }
        todo!("timer loop: sleep, check should_distill, call compress")
    }

    /// Are there at least `policy.threshold` messages not yet covered by a
    /// summary?
    /// TODO (your core logic): count messages after `last_covered` for the
    /// current adventure, compare to `policy.threshold`.
    /// NOTE: `MemoryManager` keeps its state/pool private and exposes no
    /// "uncovered count" today. You will need to add one accessor there (e.g.
    /// `MemoryManager::uncovered_count() -> Result<u64, _>`) before this body
    /// can be written — decide its shape rather than reaching into fields here.
    async fn should_distill(&self) -> Result<bool, MemoryManagerError> {
        todo!("count uncovered messages, compare to policy.threshold")
    }
}
