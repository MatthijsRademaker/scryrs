//! Standalone trace and hotspot foundation for scryrs.

use scryrs_types::{FeatureDescriptor, Hotspot, TraceEvent};

pub fn descriptor() -> FeatureDescriptor {
    FeatureDescriptor {
        id: "core",
        title: "scryrs-core",
        summary: "standalone trace ingestion and hotspot detection foundation",
    }
}

/// Minimal deterministic hotspot scorer for scaffold validation.
pub fn score_events(events: &[TraceEvent]) -> Vec<Hotspot> {
    let mut hotspots = Vec::new();

    for event in events {
        if let Some(index) = hotspots
            .iter()
            .position(|hotspot: &Hotspot| hotspot.subject.as_str() == event.subject.as_str())
        {
            hotspots[index].score += 1;
        } else {
            hotspots.push(Hotspot {
                subject: event.subject.clone(),
                score: 1,
            });
        }
    }

    hotspots.sort_by(|left, right| {
        right
            .score
            .cmp(&left.score)
            .then_with(|| left.subject.cmp(&right.subject))
    });

    hotspots
}

#[cfg(test)]
mod tests {
    use scryrs_types::TraceEventKind;

    use super::*;

    #[test]
    fn scores_repeated_subjects_first() {
        let events = vec![
            TraceEvent {
                kind: TraceEventKind::FileRead,
                subject: "src/a.rs".to_string(),
            },
            TraceEvent {
                kind: TraceEventKind::Search,
                subject: "routing".to_string(),
            },
            TraceEvent {
                kind: TraceEventKind::FileRead,
                subject: "src/a.rs".to_string(),
            },
        ];

        let hotspots = score_events(&events);

        assert_eq!(hotspots[0].subject, "src/a.rs");
        assert_eq!(hotspots[0].score, 2);
    }
}
