//! The ring: a bounded `VecDeque` of events with a sequence number each.
//! A cursor is the last sequence number a caller has seen, so a cursor read
//! is a binary search. Gaps (events herdr dropped or we missed) are markers
//! in the ring, so they sit in order with the events around them.
use serde_json::{Value, json};
use std::collections::VecDeque;
use std::time::{SystemTime, UNIX_EPOCH};

/// The event name of a gap marker.
pub const GAP: &str = "gap";

#[derive(Debug, Clone)]
pub struct Entry {
    pub seq: u64,
    /// herdr's `event` name, as sent (`tab_created`, `pane.agent_status_changed`).
    pub name: String,
    /// herdr's `data`, unchanged.
    pub data: Value,
    workspace_id: Option<String>,
    tab_id: Option<String>,
    pane_id: Option<String>,
    agent_status: Option<String>,
}

impl Entry {
    #[must_use]
    pub fn to_json(&self) -> Value {
        json!({"seq": self.seq, "event": self.name, "data": self.data})
    }

    #[must_use]
    pub fn is_gap(&self) -> bool {
        self.name == GAP
    }

    #[must_use]
    pub fn pane_id(&self) -> Option<&str> {
        self.pane_id.as_deref()
    }
}

/// `pane.agent_status_changed` and `pane_agent_status_changed` are one type.
#[must_use]
pub fn normalize(name: &str) -> String {
    name.replace('.', "_")
}

/// An id found at the top of `data` or one object down (`data.tab.tab_id`).
fn find(data: &Value, key: &str, depth: u8) -> Option<String> {
    let obj = data.as_object()?;
    if let Some(s) = obj.get(key).and_then(Value::as_str) {
        return Some(s.to_owned());
    }
    if depth == 0 {
        return None;
    }
    obj.values().find_map(|v| find(v, key, depth - 1))
}

#[derive(Debug, Default, Clone)]
pub struct Filter {
    /// Normalized event names; empty means all.
    pub types: Vec<String>,
    pub workspace_id: Option<String>,
    pub tab_id: Option<String>,
    pub pane_id: Option<String>,
    pub agent_status: Option<String>,
}

impl Filter {
    fn matches(&self, e: &Entry) -> bool {
        let same = |want: &Option<String>, have: &Option<String>| {
            want.as_ref().is_none_or(|w| have.as_ref() == Some(w))
        };
        (self.types.is_empty() || self.types.contains(&normalize(&e.name)))
            && same(&self.workspace_id, &e.workspace_id)
            && same(&self.tab_id, &e.tab_id)
            && same(&self.pane_id, &e.pane_id)
            && same(&self.agent_status, &e.agent_status)
    }
}

pub struct Page {
    pub events: Vec<Value>,
    pub next: u64,
    pub gaps: Vec<Value>,
}

pub struct Ring {
    capacity: usize,
    buf: VecDeque<Entry>,
    next_seq: u64,
    /// Times a subscription ended or herdr said events were lost.
    pub lost: u64,
    pub last_event_ms: Option<u64>,
}

impl Ring {
    #[must_use]
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity: capacity.max(1),
            buf: VecDeque::new(),
            next_seq: 1,
            lost: 0,
            last_event_ms: None,
        }
    }

    #[must_use]
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.buf.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.buf.is_empty()
    }

    #[must_use]
    pub fn last_seq(&self) -> u64 {
        self.next_seq - 1
    }

    /// Add an event as herdr sent it; returns its sequence number.
    pub fn push(&mut self, name: &str, data: Value) -> u64 {
        let seq = self.next_seq;
        self.next_seq += 1;
        let entry = Entry {
            seq,
            name: name.to_owned(),
            workspace_id: find(&data, "workspace_id", 2),
            tab_id: find(&data, "tab_id", 2),
            pane_id: find(&data, "pane_id", 2),
            agent_status: find(&data, "agent_status", 0),
            data,
        };
        if self.buf.len() == self.capacity {
            self.buf.pop_front();
        }
        self.buf.push_back(entry);
        self.last_event_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .ok()
            .and_then(|d| u64::try_from(d.as_millis()).ok());
        seq
    }

    /// Mark that events may be missing here.
    pub fn push_gap(&mut self, reason: &str, detail: &Value) -> u64 {
        self.push(GAP, json!({"reason": reason, "detail": detail}))
    }

    /// Index of the first entry with a sequence above `seq`.
    fn after(&self, seq: u64) -> usize {
        self.buf.partition_point(|e| e.seq <= seq)
    }

    /// Events after `since` (oldest first), or the latest ones without a
    /// cursor. Gap markers always pass the filter: they say the answer is
    /// incomplete.
    #[must_use]
    pub fn read(&self, since: Option<u64>, limit: usize, filter: &Filter) -> Page {
        let limit = limit.max(1);
        let last = self.last_seq();
        let mut gaps = Vec::new();
        let (picked, next): (Vec<&Entry>, u64) = if let Some(since) = since {
            let since = if since > last {
                gaps.push(json!({"from_seq": last + 1, "to_seq": since, "reason": "cursor is ahead of this server (it restarted)"}));
                last
            } else {
                since
            };
            if let Some(oldest) = self.buf.front().map(|e| e.seq)
                && oldest > since + 1
            {
                gaps.push(json!({"from_seq": since + 1, "to_seq": oldest - 1, "reason": "evicted from the buffer"}));
            }
            let mut picked = Vec::new();
            let mut next = last;
            for e in self.buf.iter().skip(self.after(since)) {
                if e.is_gap() || filter.matches(e) {
                    picked.push(e);
                    if picked.len() == limit {
                        // The cursor stops at the last event handed out.
                        next = e.seq;
                        break;
                    }
                }
            }
            (picked, next)
        } else {
            let mut picked: Vec<&Entry> = self
                .buf
                .iter()
                .rev()
                .filter(|e| e.is_gap() || filter.matches(e))
                .take(limit)
                .collect();
            picked.reverse();
            (picked, last)
        };
        for e in picked.iter().filter(|e| e.is_gap()) {
            gaps.push(json!({
                "from_seq": e.seq, "to_seq": e.seq,
                "reason": e.data["reason"], "detail": e.data["detail"],
            }));
        }
        Page {
            events: picked.iter().map(|e| e.to_json()).collect(),
            next,
            gaps,
        }
    }

    /// The first non-gap event after `since` that matches.
    #[must_use]
    pub fn first_after(&self, since: u64, filter: &Filter) -> Option<Entry> {
        self.buf
            .iter()
            .skip(self.after(since))
            .find(|e| !e.is_gap() && filter.matches(e))
            .cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tab(ring: &mut Ring, ws: &str, tab: &str) -> u64 {
        ring.push(
            "tab_created",
            json!({"tab": {"tab_id": tab, "workspace_id": ws}, "type": "tab_created"}),
        )
    }

    #[test]
    fn evicts_oldest_and_marks_a_stale_cursor() {
        let mut r = Ring::new(3);
        for i in 0..5 {
            tab(&mut r, "w1", &format!("w1:t{i}"));
        }
        assert_eq!((r.len(), r.last_seq()), (3, 5));
        let page = r.read(Some(1), 50, &Filter::default());
        let seqs: Vec<_> = page.events.iter().map(|e| e["seq"].as_u64()).collect();
        assert_eq!(seqs, [Some(3), Some(4), Some(5)]);
        assert_eq!(page.gaps[0]["from_seq"], 2);
        assert_eq!(page.gaps[0]["to_seq"], 2);
        assert_eq!(page.next, 5);
    }

    #[test]
    fn cursor_reads_resume_where_they_stopped() {
        let mut r = Ring::new(10);
        for i in 0..5 {
            tab(&mut r, "w1", &format!("w1:t{i}"));
        }
        let first = r.read(Some(0), 2, &Filter::default());
        assert_eq!(first.events.len(), 2);
        assert_eq!(first.next, 2);
        let rest = r.read(Some(first.next), 50, &Filter::default());
        assert_eq!(rest.events.len(), 3);
        assert_eq!(rest.next, 5);
        assert!(rest.gaps.is_empty());
        assert!(r.read(Some(5), 50, &Filter::default()).events.is_empty());
    }

    #[test]
    fn no_cursor_returns_the_latest() {
        let mut r = Ring::new(10);
        for i in 0..5 {
            tab(&mut r, "w1", &format!("w1:t{i}"));
        }
        let page = r.read(None, 2, &Filter::default());
        let seqs: Vec<_> = page.events.iter().map(|e| e["seq"].as_u64()).collect();
        assert_eq!(seqs, [Some(4), Some(5)]);
        assert_eq!(page.next, 5);
    }

    #[test]
    fn filters_by_type_ids_and_status() {
        let mut r = Ring::new(10);
        tab(&mut r, "w1", "w1:t1");
        tab(&mut r, "w2", "w2:t1");
        r.push(
            "pane.agent_status_changed",
            json!({"pane_id": "w1:p1", "workspace_id": "w1", "agent_status": "blocked"}),
        );
        let f = |f: Filter| r.read(Some(0), 50, &f).events.len();
        assert_eq!(
            f(Filter {
                workspace_id: Some("w1".into()),
                ..Filter::default()
            }),
            2
        );
        assert_eq!(
            f(Filter {
                tab_id: Some("w2:t1".into()),
                ..Filter::default()
            }),
            1
        );
        // Dotted and snake names are one type.
        assert_eq!(
            f(Filter {
                types: vec![normalize("pane_agent_status_changed")],
                ..Filter::default()
            }),
            1
        );
        assert_eq!(
            f(Filter {
                agent_status: Some("idle".into()),
                ..Filter::default()
            }),
            0
        );
        assert_eq!(
            f(Filter {
                pane_id: Some("w1:p1".into()),
                agent_status: Some("blocked".into()),
                ..Filter::default()
            }),
            1
        );
    }

    #[test]
    fn gap_markers_pass_filters_and_are_listed() {
        let mut r = Ring::new(10);
        tab(&mut r, "w1", "w1:t1");
        let g = r.push_gap("connection dropped", &json!({}));
        tab(&mut r, "w1", "w1:t2");
        let page = r.read(
            Some(0),
            50,
            &Filter {
                types: vec!["nothing".into()],
                ..Filter::default()
            },
        );
        assert_eq!(page.events.len(), 1);
        assert_eq!(page.gaps[0]["from_seq"], g);
        assert!(
            r.first_after(0, &Filter::default())
                .is_some_and(|e| e.seq == 1)
        );
        assert!(
            r.first_after(1, &Filter::default())
                .is_some_and(|e| e.seq == 3)
        );
    }
}
