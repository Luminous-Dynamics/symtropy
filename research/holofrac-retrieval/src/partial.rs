//! Partial-context spatial retrieval with exact-oracle verification.
//!
//! Similarity ranks candidates only. Every candidate is checked against the
//! typed source graph before it can be returned as a verified match. A query
//! with no exact matches yields an empty verified result set (abstention), even
//! if approximate ranking still proposes nearby candidates.

use std::time::Instant;

use symtropy_hdc_core::{BipolarBundle, BipolarHV};

use crate::{
    FactSlot, HoloFracEncoder, HoloFracError, RelationKind, SpatialFact, SpatialGraph, SpatialIndex,
};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PartialSpatialQuery {
    pub subject: Option<String>,
    pub relation: Option<RelationKind>,
    pub object: Option<String>,
}

impl PartialSpatialQuery {
    pub fn new(
        subject: Option<String>,
        relation: Option<RelationKind>,
        object: Option<String>,
    ) -> Self {
        Self {
            subject,
            relation,
            object,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.subject.is_none() && self.relation.is_none() && self.object.is_none()
    }

    fn matches(&self, fact: &SpatialFact) -> bool {
        self.subject
            .as_ref()
            .is_none_or(|subject| subject == &fact.subject)
            && self
                .relation
                .is_none_or(|relation| relation == fact.relation)
            && self
                .object
                .as_ref()
                .is_none_or(|object| object == &fact.object)
    }
}

#[derive(Clone, Debug)]
pub struct PartialCandidate {
    pub fact: SpatialFact,
    /// Mean similarity across the query's known, role-aligned fields.
    pub slot_score: f32,
    /// Similarity between the bundled partial query and the full fact vector.
    pub holistic_score: f32,
    /// Exact typed-graph oracle result; similarity never sets this field.
    pub exact_match: bool,
}

#[derive(Clone, Debug, Default)]
pub struct PartialRetrievalMetrics {
    pub precision_at_k: f64,
    /// None means there were no exact answers, so recall is undefined.
    pub recall_at_k: Option<f64>,
    /// None means there were no exact answers.
    pub reciprocal_rank: Option<f64>,
    pub false_suggestions_at_k: usize,
    pub exact_hits_at_k: usize,
}

#[derive(Clone, Debug)]
pub struct PartialQueryReport {
    pub query: PartialSpatialQuery,
    pub top_k: usize,
    pub query_slot_count: usize,
    pub exact_match_count: usize,
    /// Exact matches returned by the source graph oracle.
    pub verified_results: Vec<SpatialFact>,
    /// Exact matches returned by the codebook-keyed postings index.
    pub exact_indexed_results: Vec<SpatialFact>,
    /// True when exact verification finds no results. Approximate suggestions
    /// may still be present in the candidate rankings.
    pub abstained: bool,
    pub slot_ranked: Vec<PartialCandidate>,
    pub holistic_ranked: Vec<PartialCandidate>,
    pub slot_metrics: PartialRetrievalMetrics,
    pub holistic_metrics: PartialRetrievalMetrics,
    pub query_encoding_micros: f64,
    pub slot_ranking_micros: f64,
    pub holistic_ranking_micros: f64,
    pub exact_oracle_micros: f64,
    pub exact_indexed_micros: f64,
}

fn candidate_order(
    left: &PartialCandidate,
    right: &PartialCandidate,
    slot: bool,
) -> std::cmp::Ordering {
    let left_score = if slot {
        left.slot_score
    } else {
        left.holistic_score
    };
    let right_score = if slot {
        right.slot_score
    } else {
        right.holistic_score
    };
    right_score
        .total_cmp(&left_score)
        .then_with(|| left.fact.canonical_key().cmp(&right.fact.canonical_key()))
}

fn metrics_at_k(
    ranked: &[PartialCandidate],
    exact_match_count: usize,
    top_k: usize,
) -> PartialRetrievalMetrics {
    let selected = &ranked[..ranked.len().min(top_k)];
    let exact_hits = selected
        .iter()
        .filter(|candidate| candidate.exact_match)
        .count();
    let precision_at_k = if selected.is_empty() {
        0.0
    } else {
        exact_hits as f64 / selected.len() as f64
    };
    let recall_at_k =
        (exact_match_count > 0).then_some(exact_hits as f64 / exact_match_count as f64);
    let reciprocal_rank = if exact_match_count == 0 {
        None
    } else {
        ranked
            .iter()
            .take(top_k)
            .position(|candidate| candidate.exact_match)
            .map(|rank| 1.0 / (rank + 1) as f64)
            .or(Some(0.0))
    };

    PartialRetrievalMetrics {
        precision_at_k,
        recall_at_k,
        reciprocal_rank,
        false_suggestions_at_k: selected.len() - exact_hits,
        exact_hits_at_k: exact_hits,
    }
}

impl SpatialIndex {
    /// Retrieve candidates from an incomplete directed relation query.
    ///
    /// The index and encoder are checked against the exact source graph and
    /// full encoder specification before query execution. Known entity scales
    /// are derived from the source graph and included as explicit HDC slots.
    pub fn rank_partial_query(
        &self,
        graph: &SpatialGraph,
        encoder: &HoloFracEncoder,
        query: &PartialSpatialQuery,
        top_k: usize,
    ) -> Result<PartialQueryReport, HoloFracError> {
        if query.is_empty() {
            return Err(HoloFracError::EmptyPartialQuery);
        }
        if top_k == 0 {
            return Err(HoloFracError::InvalidTopK);
        }

        self.validate_source(graph)?;
        self.validate_encoder(encoder)?;

        let query_started = Instant::now();
        let mut query_slots: Vec<(FactSlot, BipolarHV)> = Vec::with_capacity(5);
        let mut query_keys: Vec<(FactSlot, String)> = Vec::with_capacity(5);
        let mut query_bundle = BipolarBundle::new(encoder.spec().dimension);

        if let Some(subject) = query.subject.as_deref() {
            let scale = graph.entity_scale(subject)?;
            let subject_vector = encoder.memory.bind_role_value("subject", "entity", subject);
            let scale_vector =
                encoder
                    .memory
                    .bind_role_value("subject_scale", "spatial_scale", scale.as_str());
            query_bundle.add(&subject_vector)?;
            query_bundle.add(&scale_vector)?;
            query_slots.push((FactSlot::Subject, subject_vector));
            query_slots.push((FactSlot::SubjectScale, scale_vector));
            query_keys.push((FactSlot::Subject, subject.to_owned()));
            query_keys.push((FactSlot::SubjectScale, scale.as_str().to_owned()));
        }

        if let Some(relation) = query.relation {
            let relation_vector =
                encoder
                    .memory
                    .bind_role_value("predicate", "relation", relation.as_str());
            query_bundle.add(&relation_vector)?;
            query_slots.push((FactSlot::Predicate, relation_vector));
            query_keys.push((FactSlot::Predicate, relation.as_str().to_owned()));
        }

        if let Some(object) = query.object.as_deref() {
            let scale = graph.entity_scale(object)?;
            let object_vector = encoder.memory.bind_role_value("object", "entity", object);
            let scale_vector =
                encoder
                    .memory
                    .bind_role_value("object_scale", "spatial_scale", scale.as_str());
            query_bundle.add(&object_vector)?;
            query_bundle.add(&scale_vector)?;
            query_slots.push((FactSlot::Object, object_vector));
            query_slots.push((FactSlot::ObjectScale, scale_vector));
            query_keys.push((FactSlot::Object, object.to_owned()));
            query_keys.push((FactSlot::ObjectScale, scale.as_str().to_owned()));
        }

        let query_vector = query_bundle.finish(&encoder.memory.tie_breaker("spatial-fact-v2"))?;
        let query_encoding_micros = query_started.elapsed().as_secs_f64() * 1_000_000.0;

        let exact_started = Instant::now();
        let mut verified_results: Vec<_> = graph
            .facts()
            .iter()
            .filter(|fact| query.matches(fact))
            .cloned()
            .collect();
        verified_results.sort_by_key(SpatialFact::canonical_key);
        let exact_oracle_micros = exact_started.elapsed().as_secs_f64() * 1_000_000.0;
        let exact_match_count = verified_results.len();

        // Exact postings baseline: resolve each known role/value key to its
        // sorted fact-ID list, then intersect the shortest list with the rest.
        let indexed_started = Instant::now();
        let mut posting_lists = Vec::with_capacity(query_keys.len());
        let mut missing_key = false;
        for key in &query_keys {
            let Some(code_index) = self.role_codebook_index.get(key) else {
                missing_key = true;
                break;
            };
            let posting = self
                .role_postings
                .get(*code_index)
                .ok_or(HoloFracError::MissingCodebookEntry)?;
            posting_lists.push(posting);
        }
        posting_lists.sort_by_key(|posting| posting.len());
        let mut indexed_fact_ids = if missing_key {
            Vec::new()
        } else {
            posting_lists
                .first()
                .map_or_else(Vec::new, |posting| (*posting).clone())
        };
        for posting in posting_lists.iter().skip(1) {
            indexed_fact_ids.retain(|fact_id| posting.binary_search(fact_id).is_ok());
        }
        let mut exact_indexed_results: Vec<_> = indexed_fact_ids
            .into_iter()
            .map(|fact_id| {
                self.facts
                    .get(fact_id)
                    .map(|indexed| indexed.fact.clone())
                    .ok_or(HoloFracError::MissingCodebookEntry)
            })
            .collect::<Result<_, _>>()?;
        exact_indexed_results.sort_by_key(SpatialFact::canonical_key);
        let exact_indexed_micros = indexed_started.elapsed().as_secs_f64() * 1_000_000.0;
        if exact_indexed_results != verified_results {
            return Err(HoloFracError::IndexOracleMismatch);
        }

        let slot_started = Instant::now();
        let mut slot_ranked = Vec::with_capacity(self.facts.len());
        for indexed in &self.facts {
            let mut score_sum = 0.0_f64;
            for (slot, query_value) in &query_slots {
                let code_index = indexed.role_codebook_indices[slot.index()];
                let candidate_vector = self
                    .role_codebook
                    .get(code_index)
                    .ok_or(HoloFracError::MissingCodebookEntry)?;
                score_sum += f64::from(query_value.similarity(candidate_vector)?);
            }
            let slot_score = (score_sum / query_slots.len() as f64) as f32;
            slot_ranked.push(PartialCandidate {
                fact: indexed.fact.clone(),
                slot_score,
                holistic_score: 0.0,
                exact_match: query.matches(&indexed.fact),
            });
        }
        slot_ranked.sort_by(|left, right| candidate_order(left, right, true));
        let slot_ranking_micros = slot_started.elapsed().as_secs_f64() * 1_000_000.0;

        let holistic_started = Instant::now();
        let mut holistic_ranked = Vec::with_capacity(self.facts.len());
        for indexed in &self.facts {
            let holistic_score = query_vector.similarity(&indexed.bipolar)?;
            holistic_ranked.push(PartialCandidate {
                fact: indexed.fact.clone(),
                slot_score: 0.0,
                holistic_score,
                exact_match: query.matches(&indexed.fact),
            });
        }
        holistic_ranked.sort_by(|left, right| candidate_order(left, right, false));
        let holistic_ranking_micros = holistic_started.elapsed().as_secs_f64() * 1_000_000.0;

        let slot_metrics = metrics_at_k(&slot_ranked, exact_match_count, top_k);
        let holistic_metrics = metrics_at_k(&holistic_ranked, exact_match_count, top_k);

        Ok(PartialQueryReport {
            query: query.clone(),
            top_k,
            query_slot_count: query_slots.len(),
            exact_match_count,
            abstained: verified_results.is_empty(),
            verified_results,
            exact_indexed_results,
            slot_ranked,
            holistic_ranked,
            slot_metrics,
            holistic_metrics,
            query_encoding_micros,
            slot_ranking_micros,
            holistic_ranking_micros,
            exact_oracle_micros,
            exact_indexed_micros,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{EncoderSpec, reference_architecture_graph};

    fn make_encoder(dimension: usize, seed: u64) -> HoloFracEncoder {
        HoloFracEncoder::new(EncoderSpec {
            schema_version: 2,
            dimension,
            seed,
            scalar_levels: 257,
        })
        .unwrap()
    }

    #[test]
    fn partial_subject_relation_query_recovers_all_exact_supplies() {
        let graph = reference_architecture_graph().unwrap();
        let encoder = make_encoder(8_192, 19);
        let (index, _) = SpatialIndex::build(&graph, &encoder).unwrap();
        let query = PartialSpatialQuery::new(
            Some("water-main".into()),
            Some(RelationKind::Supplies),
            None,
        );

        let report = index
            .rank_partial_query(&graph, &encoder, &query, 2)
            .unwrap();
        assert_eq!(report.exact_match_count, 2);
        assert_eq!(report.verified_results.len(), 2);
        assert!(!report.abstained);
        assert_eq!(report.slot_metrics.recall_at_k, Some(1.0));
        assert_eq!(report.slot_metrics.precision_at_k, 1.0);
        assert_eq!(report.holistic_metrics.recall_at_k, Some(1.0));
    }

    #[test]
    fn exact_postings_matches_graph_oracle_for_all_partial_query_shapes() {
        let graph = reference_architecture_graph().unwrap();
        let encoder = make_encoder(4_096, 29);
        let (index, _) = SpatialIndex::build(&graph, &encoder).unwrap();
        let cases = [
            (
                PartialSpatialQuery::new(
                    Some("water-main".into()),
                    Some(RelationKind::Supplies),
                    None,
                ),
                2,
            ),
            (
                PartialSpatialQuery::new(
                    None,
                    Some(RelationKind::Supplies),
                    Some("kitchen".into()),
                ),
                1,
            ),
            (
                PartialSpatialQuery::new(Some("water-main".into()), None, Some("bathroom".into())),
                1,
            ),
        ];

        for (query, expected_count) in cases {
            let report = index
                .rank_partial_query(&graph, &encoder, &query, 5)
                .unwrap();
            assert_eq!(report.exact_indexed_results, report.verified_results);
            assert_eq!(report.exact_indexed_results.len(), expected_count);
        }
    }

    #[test]
    fn exact_postings_abstains_for_entity_absent_from_role_slot_index() {
        let graph = reference_architecture_graph().unwrap();
        let encoder = make_encoder(2_048, 30);
        let (index, _) = SpatialIndex::build(&graph, &encoder).unwrap();
        let query = PartialSpatialQuery::new(Some("corridor".into()), None, None);

        let report = index
            .rank_partial_query(&graph, &encoder, &query, 5)
            .unwrap();
        assert!(report.exact_indexed_results.is_empty());
        assert!(report.verified_results.is_empty());
        assert!(report.abstained);
    }

    #[test]
    fn no_answer_query_abstains_after_exact_verification() {
        let graph = reference_architecture_graph().unwrap();
        let encoder = make_encoder(8_192, 23);
        let (index, _) = SpatialIndex::build(&graph, &encoder).unwrap();
        let query = PartialSpatialQuery::new(
            Some("water-main".into()),
            Some(RelationKind::DrainsTo),
            None,
        );

        let report = index
            .rank_partial_query(&graph, &encoder, &query, 3)
            .unwrap();
        assert_eq!(report.exact_match_count, 0);
        assert!(report.verified_results.is_empty());
        assert!(report.abstained);
        assert_eq!(report.slot_metrics.recall_at_k, None);
        assert_eq!(report.holistic_metrics.recall_at_k, None);
        assert_eq!(report.slot_metrics.false_suggestions_at_k, 3);
    }

    #[test]
    fn partial_query_rejects_empty_and_zero_top_k_requests() {
        let graph = reference_architecture_graph().unwrap();
        let encoder = make_encoder(2_048, 31);
        let (index, _) = SpatialIndex::build(&graph, &encoder).unwrap();

        assert!(matches!(
            index.rank_partial_query(&graph, &encoder, &PartialSpatialQuery::default(), 3),
            Err(HoloFracError::EmptyPartialQuery)
        ));
        assert!(matches!(
            index.rank_partial_query(
                &graph,
                &encoder,
                &PartialSpatialQuery::new(Some("water-main".into()), None, None),
                0
            ),
            Err(HoloFracError::InvalidTopK)
        ));
    }

    #[test]
    fn partial_query_requires_known_entity_ids() {
        let graph = reference_architecture_graph().unwrap();
        let encoder = make_encoder(2_048, 37);
        let (index, _) = SpatialIndex::build(&graph, &encoder).unwrap();
        let query = PartialSpatialQuery::new(Some("unknown-pipe".into()), None, None);

        assert!(matches!(
            index.rank_partial_query(&graph, &encoder, &query, 2),
            Err(HoloFracError::Graph(_))
        ));
    }

    #[test]
    fn partial_query_preserves_relation_direction() {
        let graph = reference_architecture_graph().unwrap();
        let encoder = make_encoder(8_192, 41);
        let (index, _) = SpatialIndex::build(&graph, &encoder).unwrap();
        let query =
            PartialSpatialQuery::new(Some("kitchen".into()), Some(RelationKind::Supplies), None);

        let report = index
            .rank_partial_query(&graph, &encoder, &query, 5)
            .unwrap();
        assert_eq!(report.exact_match_count, 0);
        assert!(report.abstained);
    }
}
