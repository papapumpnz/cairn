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
    ConditionalConstructionExample, ConditionalConstructionSynthesizer, EntityGeometryRender,
    EntityGeometryScene, GeometricEntity, RelationalGeometryScene, RenderedEntity,
};
use cairn_verifier::{ConditionalConstructionVerifier, ConditionalConstructionVerifyError};
use std::collections::{BTreeMap, BTreeSet};

#[test]
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

    eprintln!("G2_CONDITIONAL_PROGRAM cohorts=2 experiences=4 output_cardinalities=1..6 fresh_candidates={} memory_attempts=2 memory_examples=0 heldout=2 field_transfer=1 changed_truth_accepts=0", first_report.candidates_evaluated);
}

fn limits() -> InventionLimits {
    InventionLimits {
        maximum_cost: 9,
        maximum_candidates: 1_000_000,
        maximum_expression_nodes: 10_000,
        execution: ExecLimits::default(),
    }
}

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
        selected_ids.insert(identity);
        geometry.push(GeometricEntity {
            identity,
            value: shift * 100 + index_i64,
            points: [vec![shift + index_i64 * 2, shift + index_i64 * 3]].into(),
        });
        records.push(record(identity, index + domain.len()));
        for member in 0..=(index % 3) {
            let member_i64 = i64::try_from(member).expect("small");
            let member_id = Hash32::digest(format!("{domain}:member:{index}:{member}").as_bytes());
            geometry.push(GeometricEntity {
                identity: member_id,
                value: shift * 1000 + index_i64 * 10 + member_i64,
                points: [vec![shift + index_i64 * 2 + member_i64 + 1, shift + 10]].into(),
            });
            records.push(record(member_id, member + index));
            relations.push(RelationRecord {
                role: RelationRole("ordered-contact".to_owned()),
                subject: EntityId(identity),
                object: EntityId(member_id),
            });
        }
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

fn field_case() -> (FiniteField, FiniteField) {
    let field = FiniteField {
        shape: vec![4, 5],
        values: vec![
            0, 10, 20, 0, 0, 0, 0, 0, 0, 0, 30, 40, 0, 0, 0, 0, 0, 0, 0, 0,
        ],
    };
    let expected = FiniteField {
        shape: field.shape.clone(),
        values: vec![0, 10, 0, 0, 0, 0, 0, 0, 0, 0, 30, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    };
    (field, expected)
}
