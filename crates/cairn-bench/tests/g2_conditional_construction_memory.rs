use cairn_core::{
    EntityId, EntityRecord, FeatureId, Hash32, RelationRecord, RelationRole,
    RelationalEntityCollection,
};
use cairn_field::{ConditionalConstructionFieldAdapter, FiniteField};
use cairn_invent::InventionLimits;
use cairn_memory::{ConditionalConstructionPromotion, ConditionalConstructionStore, ModuleStatus};
use cairn_rkl::ExecLimits;
use cairn_runtime::{CognitionError, CognitionRuntime, ConditionalConstructionMemoryRuntimeError};
use cairn_theory::{
    ConditionalConstructionExample, ConditionalConstructionSynthesizer, EntityFeature,
    EntityGeometryRender, EntityGeometryScene, GeometricEntity, RelationalGeometryScene,
    RenderedEntity,
};
use cairn_verifier::{ConditionalConstructionVerifier, ConditionalConstructionVerifyError};
use std::collections::{BTreeMap, BTreeSet};

#[test]
#[allow(clippy::too_many_lines)]
fn relation_conditioned_program_promotes_and_removes_future_examples() {
    let first = [
        family("vector-a", 17, 19, 2, 1),
        family("sequence-a", 23, 21, 4, 2),
    ];
    let second = [
        family("diagram-a", 28, 25, 6, 3),
        family("table-a", 31, 29, 8, 4),
    ];
    let first_report = ConditionalConstructionSynthesizer
        .synthesize(&first, limits())
        .expect("cohort one");
    let second_report = ConditionalConstructionSynthesizer
        .synthesize(&second, limits())
        .expect("cohort two");
    assert_eq!(
        first_report.program.semantic_hash().expect("id"),
        second_report.program.semantic_hash().expect("id")
    );
    assert!(first_report
        .program
        .selector
        .features
        .contains(&EntityFeature::TwoHopOutDegree(
            RelationRole("ordered-contact".to_owned()),
            RelationRole("ordered-contact".to_owned()),
        )));

    let first_receipt = ConditionalConstructionVerifier
        .verify(&first_report.program, &first, ExecLimits::default())
        .expect("verify one");
    let second_receipt = ConditionalConstructionVerifier
        .verify(&second_report.program, &second, ExecLimits::default())
        .expect("verify two");
    let mut store = ConditionalConstructionStore::new();
    assert_eq!(
        store
            .record_verified_cohort(&first_report.program, first_receipt)
            .expect("provisional"),
        ConditionalConstructionPromotion::Provisional {
            verified_cohorts: 1
        }
    );
    assert_eq!(
        store
            .record_verified_cohort(&second_report.program, second_receipt)
            .expect("persistent"),
        ConditionalConstructionPromotion::Persistent {
            verified_cohorts: 2
        }
    );
    let id = first_report.program.semantic_hash().expect("program id");
    let module = store.module(id).expect("module");
    assert_eq!(module.status(), ModuleStatus::Persistent);
    assert_eq!(module.verified_experiences(), 4);
    assert_eq!(
        module.support().output_cardinalities,
        BTreeSet::from([1, 2, 3, 4])
    );

    let held_out = [
        family("graph-b", 37, 35, 11, 5),
        family("workflow-b", 41, 39, 14, 6),
    ];
    let runtime = CognitionRuntime::with_conditional_construction_memory(store);
    let mut attempts = 0_u64;
    for example in &held_out {
        let outcome = runtime
            .propose_conditional_constructions_from_memory(&example.source, ExecLimits::default())
            .expect("memory proposal");
        attempts += outcome.persistent_programs_attempted;
        assert_eq!(outcome.proposals.len(), 1);
        assert_eq!(outcome.proposals[0].construction.render, example.expected);
    }
    ConditionalConstructionVerifier
        .verify(&first_report.program, &held_out, ExecLimits::default())
        .expect("held-out independent replay");
    assert_eq!(attempts, 2);
    assert!(first_report.candidates_evaluated > attempts * 10);

    let (field, expected_field) = field_case();
    assert_eq!(
        ConditionalConstructionFieldAdapter
            .predict_ordered_contact(&first_report.program, &field, ExecLimits::default())
            .expect("field adapter transfer"),
        expected_field
    );

    assert!(matches!(
        CognitionRuntime::new().propose_conditional_constructions_from_memory(
            &held_out[0].source,
            ExecLimits::default()
        ),
        Err(CognitionError::ConditionalConstructionMemory(
            ConditionalConstructionMemoryRuntimeError::NoVerifiedProposal
        ))
    ));

    let mut changed_truth = held_out.clone();
    for example in &mut changed_truth {
        for relation in &mut example.source.relations.relations {
            std::mem::swap(&mut relation.subject, &mut relation.object);
        }
    }
    assert!(matches!(
        ConditionalConstructionVerifier.verify(
            &first_report.program,
            &changed_truth,
            ExecLimits::default()
        ),
        Err(ConditionalConstructionVerifyError::ReplayMismatch
            | ConditionalConstructionVerifyError::Construction)
    ));

    eprintln!("G2_RELATION_PATH_PROGRAM cohorts=2 experiences=4 output_cardinalities=1..6 fresh_candidates={} memory_attempts=2 memory_examples=0 heldout=2 field_transfer=1 changed_truth_accepts=0", first_report.candidates_evaluated);
}

fn limits() -> InventionLimits {
    InventionLimits {
        maximum_cost: 9,
        maximum_candidates: 1_000_000,
        maximum_expression_nodes: 10_000,
        execution: ExecLimits::default(),
    }
}

#[allow(clippy::too_many_lines)]
fn family(
    domain: &str,
    rows: u32,
    columns: u32,
    shift: i64,
    selected_count: usize,
) -> ConditionalConstructionExample {
    let mut geometry = Vec::new();
    let mut records = Vec::new();
    let mut relations = Vec::new();
    let mut selected_ids = BTreeSet::new();
    for index in 0..selected_count {
        let index_i64 = i64::try_from(index).expect("small");
        let identity = Hash32::digest(format!("{domain}:controller:{index}").as_bytes());
        let bridge = Hash32::digest(format!("{domain}:bridge:{index}").as_bytes());
        let endpoint = Hash32::digest(format!("{domain}:endpoint:{index}").as_bytes());
        selected_ids.insert(identity);
        geometry.extend([
            geometric(
                identity,
                shift * 100 + index_i64,
                shift + index_i64 * 2,
                shift,
            ),
            geometric(
                bridge,
                shift * 1000 + index_i64 * 10,
                shift + index_i64 * 2,
                shift + 1,
            ),
            geometric(
                endpoint,
                shift * 1000 + index_i64 * 10 + 1,
                shift + index_i64 * 2,
                shift + 2,
            ),
        ]);
        records.extend([record(identity, 0), record(bridge, 0), record(endpoint, 0)]);
        relations.extend([
            relation("ordered-contact", identity, bridge),
            relation("ordered-contact", bridge, endpoint),
        ]);

        let decoy = Hash32::digest(format!("{domain}:decoy:{index}").as_bytes());
        let decoy_bridge = Hash32::digest(format!("{domain}:decoy-bridge:{index}").as_bytes());
        let decoy_endpoint = Hash32::digest(format!("{domain}:decoy-endpoint:{index}").as_bytes());
        geometry.extend([
            geometric(
                decoy,
                shift * 2000 + index_i64 * 10,
                shift + index_i64 * 2 + 1,
                shift + 5,
            ),
            geometric(
                decoy_bridge,
                shift * 3000 + index_i64 * 10,
                shift + index_i64 * 2 + 1,
                shift + 6,
            ),
            geometric(
                decoy_endpoint,
                shift * 3000 + index_i64 * 10 + 1,
                shift + index_i64 * 2 + 1,
                shift + 7,
            ),
        ]);
        records.extend([
            record(decoy, 0),
            record(decoy_bridge, 0),
            record(decoy_endpoint, 0),
        ]);
        relations.extend([
            relation("ordered-contact", decoy, decoy_bridge),
            relation("distracts", decoy_bridge, decoy_endpoint),
        ]);
    }
    let orphan = Hash32::digest(format!("{domain}:orphan").as_bytes());
    geometry.push(GeometricEntity {
        identity: orphan,
        value: shift * 10_000,
        points: [vec![shift + 15, shift + 14]].into(),
    });
    records.push(record(orphan, domain.len()));
    let source_geometry = EntityGeometryScene {
        extents: vec![rows, columns],
        background: 0,
        entities: geometry,
    };
    let mut output = source_geometry
        .entities
        .iter()
        .filter(|entity| selected_ids.contains(&entity.identity))
        .map(|entity| RenderedEntity {
            value: entity.value,
            points: entity.points.clone(),
        })
        .collect::<Vec<_>>();
    output.sort_by(|left, right| left.value.cmp(&right.value));
    ConditionalConstructionExample {
        source: RelationalGeometryScene {
            geometry: source_geometry,
            relations: RelationalEntityCollection {
                entities: records,
                relations,
            },
        },
        expected: EntityGeometryRender {
            extents: vec![rows, columns],
            background: 0,
            entities: output,
        },
    }
}

fn record(identity: Hash32, distractor: usize) -> EntityRecord {
    EntityRecord {
        id: EntityId(identity),
        attributes: BTreeMap::from([(
            FeatureId("distractor".to_owned()),
            i64::try_from(distractor % 4).expect("small"),
        )]),
    }
}

fn geometric(identity: Hash32, value: i64, row: i64, column: i64) -> GeometricEntity {
    GeometricEntity {
        identity,
        value,
        points: [vec![row, column]].into(),
    }
}

fn relation(role: &str, subject: Hash32, object: Hash32) -> RelationRecord {
    RelationRecord {
        role: RelationRole(role.to_owned()),
        subject: EntityId(subject),
        object: EntityId(object),
    }
}

fn field_case() -> (FiniteField, FiniteField) {
    let field = FiniteField {
        shape: vec![3, 7],
        values: vec![
            10, 20, 30, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 40, 50, 0, 0, 0, 0, 0,
        ],
    };
    let expected = FiniteField {
        shape: field.shape.clone(),
        values: vec![
            10, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ],
    };
    (field, expected)
}
