use cairn_arc::run_training_batch_with_runtime_and_structural_bindings;
use cairn_core::{
    EntityId, EntityRecord, Hash32, RelationRecord, RelationRole, RelationalEntityCollection,
};
use cairn_field::LearnedStructuralFieldBinding;
use cairn_invent::InventionLimits;
use cairn_rkl::ExecLimits;
use cairn_runtime::CognitionRuntime;
use cairn_theory::{
    ConditionalConstructionExample, ConditionalConstructionSynthesizer,
    ConditionalEntityConstructionProgram, EntityGeometryRender, EntityGeometryScene,
    GeometricEntity, RelationalGeometryScene, RenderedEntity,
};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

#[test]
#[ignore = "quarantined first-10 non-ARC conditional-program causal ablation"]
fn frozen_arc10_conditional_program_ablation() {
    let corpus =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../datasets/arc-agi-1-v1.0.2/training");
    let runtime = CognitionRuntime::new();
    let blank = run_training_batch_with_runtime_and_structural_bindings(
        &corpus,
        10,
        limits(),
        ExecLimits::default(),
        &runtime,
        &[],
    )
    .expect("blank");
    let learned = non_arc_program();
    let transfer = run_training_batch_with_runtime_and_structural_bindings(
        &corpus,
        10,
        limits(),
        ExecLimits::default(),
        &runtime,
        &[LearnedStructuralFieldBinding::ConditionalOrderedContact(
            learned,
        )],
    )
    .expect("conditional transfer");
    fs::write(
        "/tmp/cairn_arc10_conditional_program_ablation_20260713.json",
        serde_json::to_string_pretty(&(blank.clone(), transfer.clone())).expect("json"),
    )
    .expect("report");
    eprintln!("ARC10_CONDITIONAL blank_exact={} transfer_exact={} blank_incorrect={} transfer_incorrect={}", blank.exact_tasks, transfer.exact_tasks, blank.incorrect_tasks, transfer.incorrect_tasks);
    assert_eq!(transfer.incorrect_tasks, 0);
}

fn non_arc_program() -> ConditionalEntityConstructionProgram {
    let examples = [
        family("vector", 12, 14, 2, 1),
        family("workflow", 18, 17, 4, 2),
        family("diagram", 22, 20, 6, 3),
    ];
    ConditionalConstructionSynthesizer
        .synthesize(&examples, limits())
        .expect("non-ARC program")
        .program
}

fn family(
    domain: &str,
    rows: u32,
    columns: u32,
    shift: i64,
    count: usize,
) -> ConditionalConstructionExample {
    let mut geometry = Vec::new();
    let mut records = Vec::new();
    let mut relations = Vec::new();
    let mut selected = BTreeSet::new();
    for index in 0..count {
        let i = i64::try_from(index).expect("small");
        let controller = Hash32::digest(format!("{domain}:c:{index}").as_bytes());
        let member = Hash32::digest(format!("{domain}:m:{index}").as_bytes());
        selected.insert(controller);
        geometry.push(GeometricEntity {
            identity: controller,
            value: shift * 100 + i,
            points: [vec![shift + i * 2, shift + i * 2]].into(),
        });
        geometry.push(GeometricEntity {
            identity: member,
            value: shift * 1000 + i,
            points: [vec![shift + i * 2, shift + i * 2 + 1]].into(),
        });
        records.push(EntityRecord {
            id: EntityId(controller),
            attributes: BTreeMap::new(),
        });
        records.push(EntityRecord {
            id: EntityId(member),
            attributes: BTreeMap::new(),
        });
        relations.push(RelationRecord {
            role: RelationRole("ordered-contact".to_owned()),
            subject: EntityId(controller),
            object: EntityId(member),
        });
    }
    let scene = EntityGeometryScene {
        extents: vec![rows, columns],
        background: 0,
        entities: geometry,
    };
    let mut entities = scene
        .entities
        .iter()
        .filter(|entity| selected.contains(&entity.identity))
        .map(|entity| RenderedEntity {
            value: entity.value,
            points: entity.points.clone(),
        })
        .collect::<Vec<_>>();
    entities.sort_by_key(|entity| entity.value);
    ConditionalConstructionExample {
        source: RelationalGeometryScene {
            geometry: scene,
            relations: RelationalEntityCollection {
                entities: records,
                relations,
            },
        },
        expected: EntityGeometryRender {
            extents: vec![rows, columns],
            background: 0,
            entities,
        },
    }
}

fn limits() -> InventionLimits {
    InventionLimits {
        maximum_cost: 9,
        maximum_candidates: 1_000_000,
        maximum_expression_nodes: 10_000,
        execution: ExecLimits::default(),
    }
}
