use std::time::{Duration, Instant};

#[derive(Clone)]
pub struct Selection<P, S> {
    pub provider: P,
    pub source: S,
    pub generation: u64,
    pub attempted: Instant,
}

pub struct DisplayState<P, S, D> {
    pub snapshot: Option<D>,
    selection: Option<Selection<P, S>>,
    generation: u64,
    mutating: bool,
}
impl<P: Clone, S: Clone, D> Default for DisplayState<P, S, D> {
    fn default() -> Self {
        Self {
            snapshot: None,
            selection: None,
            generation: 0,
            mutating: false,
        }
    }
}
impl<P: Clone, S: Clone, D> DisplayState<P, S, D> {
    pub fn begin(&mut self, provider: P, source: S, now: Instant) -> Option<u64> {
        if self.mutating {
            return None;
        }
        self.generation = self.generation.wrapping_add(1);
        self.selection = Some(Selection {
            provider,
            source,
            generation: self.generation,
            attempted: now,
        });
        self.snapshot = None;
        Some(self.generation)
    }
    pub fn finish(&mut self, generation: u64, data: Option<D>) -> bool {
        if self.mutating || generation != self.generation {
            return false;
        }
        self.snapshot = data;
        true
    }
    pub fn start_mutation(&mut self) -> bool {
        if self.mutating {
            return false;
        }
        self.mutating = true;
        self.generation = self.generation.wrapping_add(1);
        self.snapshot = None;
        self.selection = None;
        true
    }
    pub fn end_mutation(&mut self) {
        self.mutating = false;
    }
    pub fn due(&mut self, now: Instant) -> Option<Selection<P, S>> {
        if self.mutating {
            return None;
        }
        let selection = self.selection.as_mut()?;
        if now.duration_since(selection.attempted) < Duration::from_secs(300) {
            return None;
        }
        selection.attempted = now;
        Some(selection.clone())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn old_account_result_cannot_overwrite_a_new_selection() {
        let mut state = DisplayState::default();
        let old = state.begin("first", "source", Instant::now()).unwrap();
        let new = state.begin("second", "source", Instant::now()).unwrap();
        assert!(!state.finish(old, Some("old account")));
        assert!(state.finish(new, Some("new account")));
        assert_eq!(state.snapshot, Some("new account"));
    }
    #[test]
    fn credential_mutation_blocks_reads_and_discards_old_snapshot() {
        let mut state = DisplayState::default();
        let old = state.begin("first", "source", Instant::now()).unwrap();
        state.finish(old, Some("old account"));
        assert!(state.start_mutation());
        assert_eq!(state.snapshot, None);
        assert!(!state.start_mutation());
        assert!(state.begin("first", "source", Instant::now()).is_none());
        assert!(!state.finish(old, Some("old account")));
        state.end_mutation();
        assert!(state.begin("first", "source", Instant::now()).is_some());
    }
    #[test]
    fn periodic_refresh_has_a_five_minute_deadline() {
        let mut state: DisplayState<_, _, String> = DisplayState::default();
        let now = Instant::now();
        state.begin("selected", "source", now);
        assert!(state.due(now + Duration::from_secs(299)).is_none());
        let selected = state.due(now + Duration::from_secs(300)).unwrap();
        assert_eq!(selected.provider, "selected");
        assert_eq!(selected.source, "source");
        assert!(state.due(now + Duration::from_secs(301)).is_none());
        state.start_mutation();
        state.end_mutation();
        assert!(state.due(now + Duration::from_secs(1000)).is_none());
    }
}
