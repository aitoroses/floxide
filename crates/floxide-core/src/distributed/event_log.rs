use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use std::vec::Vec;
use uuid::Uuid;

use crate::merge::Merge;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LoggedEvent<E> {
    pub uuid: Uuid,
    pub timestamp: u128,
    pub event: E,
}

/// In-memory append-only event log with interior mutability.
#[derive(Clone)]
pub struct EventLog<E> {
    events: Arc<Mutex<Vec<LoggedEvent<E>>>>,
}

impl<E> Default for EventLog<E> {
    fn default() -> Self {
        Self {
            events: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

impl<E: Debug + Clone> Debug for EventLog<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "EventLog {{ {:?} }}",
            self.events
                .lock()
                .unwrap()
                .iter()
                .map(|e| e.event.clone())
                .collect::<Vec<_>>()
        )
    }
}

impl<E: Clone> Merge for EventLog<E> {
    /// Merges another EventLog into this one, deduplicating by UUID and ordering by (timestamp, uuid).
    ///
    /// # Deadlock Prevention
    /// - If `self` and `other` are the same Arc, merging is a no-op (prevents self-deadlock).
    /// - Otherwise, always lock the two logs in a consistent order (by pointer address) to prevent lock order inversion deadlocks.
    fn merge(&mut self, other: Self) {
        let self_ptr = Arc::as_ptr(&self.events) as usize;
        let other_ptr = Arc::as_ptr(&other.events) as usize;
        if self_ptr == other_ptr {
            // Prevent self-deadlock: merging a log with itself is a no-op
            return;
        }
        // Lock in address order to prevent lock order inversion deadlocks
        let (first, second) = if self_ptr < other_ptr {
            (&self.events, &other.events)
        } else {
            (&other.events, &self.events)
        };
        let mut first_guard = first.lock().unwrap();
        let mut second_guard = second.lock().unwrap();
        // If self_ptr < other_ptr, first_guard is self, second_guard is other
        // If self_ptr > other_ptr, first_guard is other, second_guard is self
        // Always merge into self
        if self_ptr < other_ptr {
            first_guard.extend(second_guard.drain(..));
            first_guard.sort_by(|a, b| a.timestamp.cmp(&b.timestamp).then(a.uuid.cmp(&b.uuid)));
            first_guard.dedup_by_key(|e| e.uuid);
        } else {
            second_guard.extend(first_guard.drain(..));
            second_guard.sort_by(|a, b| a.timestamp.cmp(&b.timestamp).then(a.uuid.cmp(&b.uuid)));
            second_guard.dedup_by_key(|e| e.uuid);
            // Move merged data back to self
            *first_guard = second_guard.clone();
        }
    }
}

impl<E: Serialize> Serialize for EventLog<E> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let events = self.events.lock().unwrap();
        events.serialize(serializer)
    }
}

impl<'de, E: Deserialize<'de>> Deserialize<'de> for EventLog<E> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let events = Vec::<LoggedEvent<E>>::deserialize(deserializer)?;
        Ok(EventLog {
            events: Arc::new(Mutex::new(events)),
        })
    }
}

impl<E> EventLog<E> {
    pub fn new() -> Self {
        Self {
            events: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn append(&self, event: E) {
        let logged = LoggedEvent {
            uuid: Uuid::new_v4(),
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis(),
            event,
        };
        self.events.lock().unwrap().push(logged);
    }

    pub fn iter(&self) -> Vec<LoggedEvent<E>>
    where
        E: Clone,
    {
        self.events.lock().unwrap().clone()
    }

    /// Applies all events in order to the given state using the provided closure.
    pub fn apply_all<S, F>(&self, state: &mut S, apply_fn: F)
    where
        F: Fn(&E, &mut S),
        E: Clone,
    {
        for logged in self.iter() {
            apply_fn(&logged.event, state);
        }
    }

    /// Creates a state using Default, applies all events using the provided closure, and returns the resulting state.
    pub fn apply_all_default<S, F>(&self, apply_fn: F) -> S
    where
        S: Default,
        F: Fn(&E, &mut S),
        E: Clone,
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
        let log = EventLog::new();
        log.append(WorkflowEvent::WorkStarted { id: 1 });
        log.append(WorkflowEvent::WorkCompleted { id: 1, result: 42 });
        let events: Vec<_> = log.iter();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].event, WorkflowEvent::WorkStarted { id: 1 });
        assert_eq!(
            events[1].event,
            WorkflowEvent::WorkCompleted { id: 1, result: 42 }
        );
    }

    #[test]
    fn test_apply_all() {
        let log = EventLog::new();
        log.append(WorkflowEvent::WorkStarted { id: 1 });
        log.append(WorkflowEvent::WorkStarted { id: 2 });
        log.append(WorkflowEvent::WorkCompleted { id: 1, result: 10 });
        log.append(WorkflowEvent::WorkFailed {
            id: 2,
            reason: "error".to_string(),
        });

        let mut state = WorkflowState::default();
        log.apply_all(&mut state, |event, state| match event {
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
        });

        assert!(!state.running.contains(&1));
        assert!(!state.running.contains(&2));
        assert_eq!(state.completed.get(&1), Some(&10));
        assert_eq!(state.failed.get(&2), Some(&"error".to_string()));
    }

    #[test]
    fn test_apply_all_default() {
        let log = EventLog::new();
        log.append(WorkflowEvent::WorkStarted { id: 1 });
        log.append(WorkflowEvent::WorkStarted { id: 2 });
        log.append(WorkflowEvent::WorkCompleted { id: 1, result: 10 });
        log.append(WorkflowEvent::WorkFailed {
            id: 2,
            reason: "error".to_string(),
        });

        let state = log.apply_all_default(|event, state: &mut WorkflowState| match event {
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
        });

        assert!(!state.running.contains(&1));
        assert!(!state.running.contains(&2));
        assert_eq!(state.completed.get(&1), Some(&10));
        assert_eq!(state.failed.get(&2), Some(&"error".to_string()));
    }
}
