use cairn_core::{
    EntityId, EntityRecord, FeatureId, Hash32, RelationRecord, RelationRole,
    RelationalEntityCollection,
};
use cairn_invent::InventionLimits;
use cairn_rkl::ExecLimits;
use cairn_theory::{
    ConditionalConstructionExample, ConditionalConstructionSynthesizer, EntityGeometryRender,
    EntityGeometryScene, GeometricEntity, RelationalGeometryScene, RenderedEntity,
};
use std::collections::BTreeMap;

#[test]
fn invents_relational_selection_and_variable_cardinality_geometry_jointly() {
    let training = [
        family("vector", 20, 23, 2, 1),
        family("sequence", 27, 25, 5, 2),
    ];
    let report = ConditionalConstructionSynthesizer
        .synthesize(&training, limits())
        .expect("conditional program");
    assert_eq!(report.examples, 2);
    assert!(report.positive_bindings > 0);
    assert!(report.negative_bindings > 0);

    let held_out = family("diagram", 34, 31, 8, 3);
    let result = ConditionalConstructionSynthesizer
        .execute(&report.program, &held_out.source, ExecLimits::default())
        .expect("held-out execution");
    assert_eq!(result.render, held_out.expected);
    assert_eq!(result.render.entities.len(), 3);

    let mut changed = held_out.source.clone();
    changed.relations.relations.clear();
    assert!(ConditionalConstructionSynthesizer
        .execute(&report.program, &changed, ExecLimits::default())
        .is_err());
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
    let mut relations = Vec::new();
    let mut records = Vec::new();
    for index in 0..selected_count {
        let offset = i64::try_from(index * 5).expect("small");
        let name = format!("{domain}:container:{index}");
        let identity = Hash32::digest(name.as_bytes());
        geometry.push(GeometricEntity {
            identity,
            value: 100 + i64::try_from(index).expect("small") + shift * 10,
            points: [
                (shift + offset, shift),
                (shift + offset + 1, shift),
                (shift + offset + 2, shift + 1),
            ]
            .into_iter()
            .map(|(row, column)| vec![row, column])
            .collect(),
        });
        records.push(record(identity, index));
        for member in 0..(index + 2) {
            let member_name = format!("{domain}:member:{index}:{member}");
            let member_id = Hash32::digest(member_name.as_bytes());
            let member_offset = i64::try_from(member).expect("small");
            geometry.push(GeometricEntity {
                identity: member_id,
                value: 500 + i64::try_from(index * 10 + member).expect("small") + shift * 20,
                points: [vec![shift + offset + member_offset + 2, shift + 5]].into(),
            });
            records.push(record(member_id, member + index));
            relations.push(RelationRecord {
                role: RelationRole("membership".to_owned()),
                subject: EntityId(identity),
                object: EntityId(member_id),
            });
        }
    }
    let orphan = Hash32::digest(format!("{domain}:orphan").as_bytes());
    geometry.push(GeometricEntity {
        identity: orphan,
        value: 900 + shift,
        points: [vec![shift + 12, shift + 9]].into(),
    });
    records.push(record(orphan, 2));
    let source_geometry = EntityGeometryScene {
        extents: vec![rows, columns],
        background: 0,
        entities: geometry,
    };
    let selected = source_geometry
        .entities
        .iter()
        .filter(|entity| {
            relations
                .iter()
                .any(|relation| relation.subject == EntityId(entity.identity))
        })
        .cloned()
        .collect::<Vec<_>>();
    let min_row = selected
        .iter()
        .flat_map(|entity| &entity.points)
        .map(|point| point[0])
        .min()
        .expect("row");
    let max_row = selected
        .iter()
        .flat_map(|entity| &entity.points)
        .map(|point| point[0])
        .max()
        .expect("row");
    let min_column = selected
        .iter()
        .flat_map(|entity| &entity.points)
        .map(|point| point[1])
        .min()
        .expect("column");
    let max_column = selected
        .iter()
        .flat_map(|entity| &entity.points)
        .map(|point| point[1])
        .max()
        .expect("column");
    let mut output = selected
        .iter()
        .map(|entity| RenderedEntity {
            value: entity.value,
            points: entity
                .points
                .iter()
                .map(|point| vec![point[0] - min_row, max_column - point[1]])
                .collect(),
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
            extents: vec![
                u32::try_from(max_row - min_row + 1).expect("extent"),
                u32::try_from(max_column - min_column + 1).expect("extent"),
            ],
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
            i64::try_from(distractor % 3).expect("small"),
        )]),
    }
}
