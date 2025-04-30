use std::vec::Vec;
use serde::{Deserialize, Serialize};

use crate::merge::Merge;

/// Trait for applying an event to a state.
pub trait EventApplier<E, S> {
    fn apply(&self, event: &E, state: &mut S);
}

/// In-memory append-only event log.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventLog<E> {
    events: Vec<E>,
}

impl<E> Default for EventLog<E> {
    fn default() -> Self {
        Self::new()
    }
}

impl<E> Merge for EventLog<E> {
    fn merge(&mut self, other: Self) {
        self.events.extend(other.events);
    }
}

impl<E> EventLog<E> {
    pub fn new() -> Self {
        Self { events: Vec::new() }
    }

    pub fn append(&mut self, event: E) {
        self.events.push(event);
    }

    pub fn iter(&self) -> impl Iterator<Item = &E> {
        self.events.iter()
    }

    /// Applies all events in order to the given state using the provided closure.
    pub fn apply_all<S, F>(&self, state: &mut S, apply_fn: F)
    where
        F: Fn(&E, &mut S),
    {
        for event in &self.events {
            apply_fn(event, state);
        }
    }

    /// Creates a state using Default, applies all events using the provided closure, and returns the resulting state.
    pub fn apply_all_default<S, F>(&self, apply_fn: F) -> S
    where
        S: Default,
        F: Fn(&E, &mut S),
    {
        let mut state = S::default();
        self.apply_all(&mut state, apply_fn);
        state
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{HashMap, HashSet};

    // Example event type
    #[derive(Debug, Clone, PartialEq, Eq)]
    enum WorkflowEvent {
        WorkStarted { id: u64 },
        WorkCompleted { id: u64, result: i32 },
        WorkFailed { id: u64, reason: String },
    }

    // Example state type
    #[derive(Debug, Default, PartialEq, Eq)]
    struct WorkflowState {
        running: HashSet<u64>,
        completed: HashMap<u64, i32>,
        failed: HashMap<u64, String>,
    }

    #[test]
    fn test_append_and_iter() {
        let mut log = EventLog::new();
        log.append(WorkflowEvent::WorkStarted { id: 1 });
        log.append(WorkflowEvent::WorkCompleted { id: 1, result: 42 });
        let events: Vec<_> = log.iter().cloned().collect();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0], WorkflowEvent::WorkStarted { id: 1 });
        assert_eq!(events[1], WorkflowEvent::WorkCompleted { id: 1, result: 42 });
    }

    #[test]
    fn test_apply_all() {
        let mut log = EventLog::new();
        log.append(WorkflowEvent::WorkStarted { id: 1 });
        log.append(WorkflowEvent::WorkStarted { id: 2 });
        log.append(WorkflowEvent::WorkCompleted { id: 1, result: 10 });
        log.append(WorkflowEvent::WorkFailed { id: 2, reason: "error".to_string() });

        let mut state = WorkflowState::default();
        log.apply_all(&mut state, |event, state| {
            match event {
                WorkflowEvent::WorkStarted { id } => {
                    state.running.insert(*id);
                }
                WorkflowEvent::WorkCompleted { id, result } => {
                    state.running.remove(id);
                    state.completed.insert(*id, *result);
                }
                WorkflowEvent::WorkFailed { id, reason } => {
                    state.running.remove(id);
                    state.failed.insert(*id, reason.clone());
                }
            }
        });

        assert!(!state.running.contains(&1));
        assert!(!state.running.contains(&2));
        assert_eq!(state.completed.get(&1), Some(&10));
        assert_eq!(state.failed.get(&2), Some(&"error".to_string()));
    }

    #[test]
    fn test_apply_all_default() {
        let mut log = EventLog::new();
        log.append(WorkflowEvent::WorkStarted { id: 1 });
        log.append(WorkflowEvent::WorkStarted { id: 2 });
        log.append(WorkflowEvent::WorkCompleted { id: 1, result: 10 });
        log.append(WorkflowEvent::WorkFailed { id: 2, reason: "error".to_string() });

        let state = log.apply_all_default(|event, state: &mut WorkflowState| {
            match event {
                WorkflowEvent::WorkStarted { id } => {
                    state.running.insert(*id);
                }
                WorkflowEvent::WorkCompleted { id, result } => {
                    state.running.remove(id);
                    state.completed.insert(*id, *result);
                }
                WorkflowEvent::WorkFailed { id, reason } => {
                    state.running.remove(id);
                    state.failed.insert(*id, reason.clone());
                }
            }
        });

        assert!(!state.running.contains(&1));
        assert!(!state.running.contains(&2));
        assert_eq!(state.completed.get(&1), Some(&10));
        assert_eq!(state.failed.get(&2), Some(&"error".to_string()));
    }
}
