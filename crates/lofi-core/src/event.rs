#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Section {
    Intro,
    Groove,
    Drop,
    Breakdown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EventAction {
    SetSection(Section),
    SetSeed(u64),
    SetTempo { bpm_milli: u32 },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ScheduledEvent {
    pub fire_at_tick: i64,
    pub action: EventAction,
    pub id: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ScheduleError {
    Full,
}

#[derive(Clone, Debug)]
pub struct EventQueue<const N: usize> {
    len: usize,
    events: [Option<ScheduledEvent>; N],
}

impl<const N: usize> EventQueue<N> {
    pub const fn new() -> Self {
        Self {
            len: 0,
            events: [None; N],
        }
    }

    pub const fn len(&self) -> usize {
        self.len
    }

    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn push(&mut self, event: ScheduledEvent) -> Result<(), ScheduleError> {
        if self.len == N {
            return Err(ScheduleError::Full);
        }

        let mut insert_at = self.len;
        for ix in 0..self.len {
            if let Some(existing) = self.events[ix] {
                if existing.id == event.id {
                    self.events[ix] = Some(event);
                    self.sort();
                    return Ok(());
                }
                if event.fire_at_tick < existing.fire_at_tick && insert_at == self.len {
                    insert_at = ix;
                }
            }
        }

        let mut ix = self.len;
        while ix > insert_at {
            self.events[ix] = self.events[ix - 1];
            ix -= 1;
        }
        self.events[insert_at] = Some(event);
        self.len += 1;
        Ok(())
    }

    pub fn pop_due(&mut self, current_tick: i64) -> Option<ScheduledEvent> {
        let first = self.events[0]?;
        if first.fire_at_tick > current_tick {
            return None;
        }

        let out = first;
        for ix in 1..self.len {
            self.events[ix - 1] = self.events[ix];
        }
        self.len -= 1;
        self.events[self.len] = None;
        Some(out)
    }

    fn sort(&mut self) {
        let mut ix = 1;
        while ix < self.len {
            let item = self.events[ix];
            let mut j = ix;
            while j > 0
                && self.events[j - 1]
                    .map(|e| e.fire_at_tick)
                    .unwrap_or(i64::MAX)
                    > item.map(|e| e.fire_at_tick).unwrap_or(i64::MAX)
            {
                self.events[j] = self.events[j - 1];
                j -= 1;
            }
            self.events[j] = item;
            ix += 1;
        }
    }
}

impl<const N: usize> Default for EventQueue<N> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pops_in_tick_order() {
        let mut q: EventQueue<4> = EventQueue::new();
        q.push(ScheduledEvent {
            fire_at_tick: 20,
            action: EventAction::SetSection(Section::Drop),
            id: 2,
        })
        .unwrap();
        q.push(ScheduledEvent {
            fire_at_tick: 10,
            action: EventAction::SetSection(Section::Groove),
            id: 1,
        })
        .unwrap();
        assert_eq!(q.pop_due(9), None);
        assert_eq!(q.pop_due(10).unwrap().id, 1);
        assert_eq!(q.pop_due(20).unwrap().id, 2);
        assert!(q.is_empty());
    }
}
