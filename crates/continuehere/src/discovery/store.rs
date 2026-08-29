use std::collections::BTreeMap;

use super::{
    DiscoveryCandidate, DiscoveryCandidateId, DiscoveryChange, DiscoveryError, DiscoverySource,
};

const MAX_DISCOVERY_CANDIDATES: usize = 128;

#[derive(Default)]
pub(crate) struct CandidateStore {
    candidates: BTreeMap<DiscoveryCandidateId, DiscoveryCandidate>,
}

impl CandidateStore {
    pub(crate) fn candidates(&self) -> Vec<DiscoveryCandidate> {
        self.candidates.values().cloned().collect()
    }

    pub(crate) fn upsert(
        &mut self,
        candidate: DiscoveryCandidate,
    ) -> Result<Option<DiscoveryChange>, DiscoveryError> {
        if let Some(existing) = self.candidates.get(candidate.id()) {
            if existing == &candidate {
                return Ok(None);
            }

            self.candidates
                .insert(candidate.id().clone(), candidate.clone());
            return Ok(Some(DiscoveryChange::Updated(candidate)));
        }

        if self.candidates.len() >= MAX_DISCOVERY_CANDIDATES {
            return Err(DiscoveryError::CandidateLimit);
        }

        self.candidates
            .insert(candidate.id().clone(), candidate.clone());
        Ok(Some(DiscoveryChange::Added(candidate)))
    }

    pub(crate) fn remove(&mut self, id: &DiscoveryCandidateId) -> Option<DiscoveryChange> {
        self.candidates.remove(id).map(DiscoveryChange::Removed)
    }

    pub(crate) fn remove_source(&mut self, source: DiscoverySource) -> Vec<DiscoveryChange> {
        let identifiers = self
            .candidates
            .values()
            .filter(|candidate| candidate.source() == source)
            .map(|candidate| candidate.id().clone())
            .collect::<Vec<_>>();
        identifiers
            .into_iter()
            .filter_map(|identifier| self.remove(&identifier))
            .collect()
    }

    pub(crate) fn clear(&mut self) -> Vec<DiscoveryChange> {
        let candidates = std::mem::take(&mut self.candidates);
        candidates
            .into_values()
            .map(DiscoveryChange::Removed)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use crate::models::ProtocolVersion;

    use super::CandidateStore;
    use crate::discovery::{
        DiscoveryCandidate, DiscoveryCandidateId, DiscoveryChange, DiscoveryEndpoint,
        DiscoverySource,
    };

    #[test]
    fn store_adds_updates_and_removes_candidates() {
        let id = DiscoveryCandidateId::new("candidate-1").expect("identifier should be valid");
        let first = candidate(id.clone(), 5200);
        let updated = candidate(id.clone(), 5300);
        let mut store = CandidateStore::default();

        assert!(matches!(
            store.upsert(first.clone()),
            Ok(Some(DiscoveryChange::Added(candidate))) if candidate == first
        ));
        assert_eq!(store.upsert(first), Ok(None));
        assert!(matches!(
            store.upsert(updated.clone()),
            Ok(Some(DiscoveryChange::Updated(candidate))) if candidate == updated
        ));
        assert!(matches!(
            store.remove(&id),
            Some(DiscoveryChange::Removed(candidate)) if candidate == updated
        ));
    }

    fn candidate(id: DiscoveryCandidateId, port: u16) -> DiscoveryCandidate {
        DiscoveryCandidate::new(
            id,
            [DiscoveryEndpoint::new("127.0.0.1", port).expect("endpoint should be valid")],
            ProtocolVersion::CURRENT,
            DiscoverySource::Local,
        )
        .expect("candidate should be valid")
    }
}
