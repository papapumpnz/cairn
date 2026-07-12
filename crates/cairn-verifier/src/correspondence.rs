use cairn_core::{CanonicalEncode, CanonicalValue, Hash32};
use cairn_theory::{
    CorrespondenceFrame, EntityCorrespondenceExample, EntityCorrespondenceMechanism,
    EntityGeometryRender, EntityGeometryScene, RenderedEntity,
};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerifiedEntityCorrespondence {
    mechanism: Hash32,
    experiences: BTreeSet<Hash32>,
    replayed_entities: u64,
    replayed_points: u64,
    observed_dimensions: BTreeSet<usize>,
    observed_extents: BTreeSet<Vec<u32>>,
    observed_entity_counts: BTreeSet<usize>,
    observed_point_cardinalities: BTreeSet<Vec<usize>>,
    observed_values: BTreeSet<i64>,
}

impl VerifiedEntityCorrespondence {
    #[must_use]
    pub const fn mechanism(&self) -> Hash32 {
        self.mechanism
    }
    #[must_use]
    pub const fn experiences(&self) -> &BTreeSet<Hash32> {
        &self.experiences
    }
    #[must_use]
    pub const fn replayed_entities(&self) -> u64 {
        self.replayed_entities
    }
    #[must_use]
    pub const fn replayed_points(&self) -> u64 {
        self.replayed_points
    }
    #[must_use]
    pub const fn observed_dimensions(&self) -> &BTreeSet<usize> {
        &self.observed_dimensions
    }
    #[must_use]
    pub const fn observed_extents(&self) -> &BTreeSet<Vec<u32>> {
        &self.observed_extents
    }
    #[must_use]
    pub const fn observed_entity_counts(&self) -> &BTreeSet<usize> {
        &self.observed_entity_counts
    }
    #[must_use]
    pub const fn observed_point_cardinalities(&self) -> &BTreeSet<Vec<usize>> {
        &self.observed_point_cardinalities
    }
    #[must_use]
    pub const fn observed_values(&self) -> &BTreeSet<i64> {
        &self.observed_values
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EntityCorrespondenceVerifyError {
    InsufficientIndependentEvidence,
    DuplicateExperience,
    InvalidScene,
    InvalidMechanism,
    ConstructionConflict,
    ReplayMismatch,
    Capacity,
    Canonicalization,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct EntityCorrespondenceVerifier;

impl EntityCorrespondenceVerifier {
    /// Independently replay correspondence and complete point-level lineage.
    ///
    /// # Errors
    ///
    /// Rejects insufficient or malformed evidence, incomplete/conflicting
    /// construction, resource excess, and output disagreement.
    pub fn verify(
        &self,
        mechanism: &EntityCorrespondenceMechanism,
        examples: &[EntityCorrespondenceExample],
        maximum_points: usize,
    ) -> Result<VerifiedEntityCorrespondence, EntityCorrespondenceVerifyError> {
        if examples.len() < 2 || maximum_points == 0 {
            return Err(EntityCorrespondenceVerifyError::InsufficientIndependentEvidence);
        }
        let mut receipt = VerifiedEntityCorrespondence {
            mechanism: mechanism
                .semantic_hash()
                .map_err(|_| EntityCorrespondenceVerifyError::Canonicalization)?,
            experiences: BTreeSet::new(),
            replayed_entities: 0,
            replayed_points: 0,
            observed_dimensions: BTreeSet::new(),
            observed_extents: BTreeSet::new(),
            observed_entity_counts: BTreeSet::new(),
            observed_point_cardinalities: BTreeSet::new(),
            observed_values: BTreeSet::new(),
        };
        for example in examples {
            if !receipt.experiences.insert(experience_hash(example)?) {
                return Err(EntityCorrespondenceVerifyError::DuplicateExperience);
            }
            let (render, sources) = reference_execute(mechanism, &example.source, maximum_points)?;
            if render != example.expected
                || sources.len()
                    != example
                        .source
                        .entities
                        .iter()
                        .map(|entity| entity.points.len())
                        .sum::<usize>()
            {
                return Err(EntityCorrespondenceVerifyError::ReplayMismatch);
            }
            for entity in &example.source.entities {
                if entity
                    .points
                    .iter()
                    .any(|point| !sources.contains(&(entity.identity, point.clone())))
                {
                    return Err(EntityCorrespondenceVerifyError::ReplayMismatch);
                }
            }
            receipt.replayed_entities = receipt
                .replayed_entities
                .checked_add(
                    u64::try_from(example.source.entities.len())
                        .map_err(|_| EntityCorrespondenceVerifyError::Capacity)?,
                )
                .ok_or(EntityCorrespondenceVerifyError::Capacity)?;
            receipt.replayed_points = receipt
                .replayed_points
                .checked_add(
                    u64::try_from(sources.len())
                        .map_err(|_| EntityCorrespondenceVerifyError::Capacity)?,
                )
                .ok_or(EntityCorrespondenceVerifyError::Capacity)?;
            receipt
                .observed_dimensions
                .insert(example.source.extents.len());
            receipt
                .observed_extents
                .insert(example.source.extents.clone());
            receipt
                .observed_entity_counts
                .insert(example.source.entities.len());
            receipt.observed_point_cardinalities.insert(
                example
                    .source
                    .entities
                    .iter()
                    .map(|entity| entity.points.len())
                    .collect(),
            );
            receipt
                .observed_values
                .extend(example.source.entities.iter().map(|entity| entity.value));
        }
        Ok(receipt)
    }
}

type SourcePoint = (Hash32, Vec<i64>);

fn reference_execute(
    mechanism: &EntityCorrespondenceMechanism,
    scene: &EntityGeometryScene,
    maximum_points: usize,
) -> Result<(EntityGeometryRender, BTreeSet<SourcePoint>), EntityCorrespondenceVerifyError> {
    validate(scene, maximum_points)?;
    let dimensions = scene.extents.len();
    let expected = (0..dimensions)
        .map(|axis| u32::try_from(axis).unwrap_or(u32::MAX))
        .collect::<BTreeSet<_>>();
    if mechanism.output_axis_sources.len() != dimensions
        || mechanism
            .output_axis_sources
            .iter()
            .copied()
            .collect::<BTreeSet<_>>()
            != expected
        || mechanism
            .reversed_output_axes
            .iter()
            .any(|axis| usize::try_from(*axis).map_or(true, |axis| axis >= dimensions))
    {
        return Err(EntityCorrespondenceVerifyError::InvalidMechanism);
    }
    let minima = extrema(scene, dimensions, true)?;
    let maxima = extrema(scene, dimensions, false)?;
    let extents = mechanism
        .output_axis_sources
        .iter()
        .map(|axis| {
            let axis =
                usize::try_from(*axis).map_err(|_| EntityCorrespondenceVerifyError::Capacity)?;
            match mechanism.frame {
                CorrespondenceFrame::PreserveObservation => Ok(scene.extents[axis]),
                CorrespondenceFrame::TightCollection => maxima[axis]
                    .checked_sub(minima[axis])
                    .and_then(|value| value.checked_add(1))
                    .and_then(|value| u32::try_from(value).ok())
                    .ok_or(EntityCorrespondenceVerifyError::Capacity),
            }
        })
        .collect::<Result<Vec<_>, _>>()?;
    let mut occupied = BTreeMap::<Vec<i64>, Hash32>::new();
    let mut sources = BTreeSet::new();
    let mut entities = Vec::new();
    for entity in &scene.entities {
        let mut points = BTreeSet::new();
        for source in &entity.points {
            let target = transform(mechanism, scene, source, &minima, &maxima)?;
            if occupied
                .insert(target.clone(), entity.identity)
                .is_some_and(|owner| owner != entity.identity)
            {
                return Err(EntityCorrespondenceVerifyError::ConstructionConflict);
            }
            points.insert(target);
            sources.insert((entity.identity, source.clone()));
        }
        entities.push(RenderedEntity {
            value: entity.value,
            points,
        });
    }
    entities.sort_by(|left, right| {
        left.value
            .cmp(&right.value)
            .then_with(|| left.points.cmp(&right.points))
    });
    Ok((
        EntityGeometryRender {
            extents,
            background: scene.background,
            entities,
        },
        sources,
    ))
}

fn validate(
    scene: &EntityGeometryScene,
    maximum_points: usize,
) -> Result<(), EntityCorrespondenceVerifyError> {
    if scene.extents.is_empty() || scene.extents.contains(&0) || scene.entities.is_empty() {
        return Err(EntityCorrespondenceVerifyError::InvalidScene);
    }
    let mut ids = BTreeSet::new();
    let mut total = 0_usize;
    for entity in &scene.entities {
        if !ids.insert(entity.identity) || entity.points.is_empty() {
            return Err(EntityCorrespondenceVerifyError::InvalidScene);
        }
        total = total
            .checked_add(entity.points.len())
            .ok_or(EntityCorrespondenceVerifyError::Capacity)?;
        if total > maximum_points {
            return Err(EntityCorrespondenceVerifyError::Capacity);
        }
        if entity.points.iter().any(|point| {
            point.len() != scene.extents.len()
                || point.iter().zip(&scene.extents).any(|(value, extent)| {
                    *value < 0
                        || u64::try_from(*value).map_or(true, |value| value >= u64::from(*extent))
                })
        }) {
            return Err(EntityCorrespondenceVerifyError::InvalidScene);
        }
    }
    Ok(())
}

fn extrema(
    scene: &EntityGeometryScene,
    dimensions: usize,
    minimum: bool,
) -> Result<Vec<i64>, EntityCorrespondenceVerifyError> {
    (0..dimensions)
        .map(|axis| {
            let values = scene
                .entities
                .iter()
                .flat_map(|entity| &entity.points)
                .map(|point| point[axis]);
            if minimum { values.min() } else { values.max() }
                .ok_or(EntityCorrespondenceVerifyError::InvalidScene)
        })
        .collect()
}

fn transform(
    mechanism: &EntityCorrespondenceMechanism,
    scene: &EntityGeometryScene,
    point: &[i64],
    minima: &[i64],
    maxima: &[i64],
) -> Result<Vec<i64>, EntityCorrespondenceVerifyError> {
    mechanism
        .output_axis_sources
        .iter()
        .copied()
        .enumerate()
        .map(|(output_axis, source_axis)| {
            let source_axis = usize::try_from(source_axis)
                .map_err(|_| EntityCorrespondenceVerifyError::Capacity)?;
            let reversed = mechanism.reversed_output_axes.contains(
                &u32::try_from(output_axis)
                    .map_err(|_| EntityCorrespondenceVerifyError::Capacity)?,
            );
            match (mechanism.frame, reversed) {
                (CorrespondenceFrame::PreserveObservation, false) => Ok(point[source_axis]),
                (CorrespondenceFrame::PreserveObservation, true) => i64::from(
                    scene.extents[source_axis]
                        .checked_sub(1)
                        .ok_or(EntityCorrespondenceVerifyError::Capacity)?,
                )
                .checked_sub(point[source_axis])
                .ok_or(EntityCorrespondenceVerifyError::Capacity),
                (CorrespondenceFrame::TightCollection, false) => point[source_axis]
                    .checked_sub(minima[source_axis])
                    .ok_or(EntityCorrespondenceVerifyError::Capacity),
                (CorrespondenceFrame::TightCollection, true) => maxima[source_axis]
                    .checked_sub(point[source_axis])
                    .ok_or(EntityCorrespondenceVerifyError::Capacity),
            }
        })
        .collect()
}

fn experience_hash(
    example: &EntityCorrespondenceExample,
) -> Result<Hash32, EntityCorrespondenceVerifyError> {
    CanonicalValue::Array(vec![
        CanonicalValue::Text("cairn.entity-correspondence-experience.v1".to_owned()),
        unsigneds(&example.source.extents),
        signed(example.source.background),
        CanonicalValue::Array(
            example
                .source
                .entities
                .iter()
                .map(|entity| {
                    CanonicalValue::Array(vec![
                        CanonicalValue::Bytes(entity.identity.as_bytes().to_vec()),
                        signed(entity.value),
                        point_set(&entity.points),
                    ])
                })
                .collect(),
        ),
        unsigneds(&example.expected.extents),
        signed(example.expected.background),
        CanonicalValue::Array(
            example
                .expected
                .entities
                .iter()
                .map(|entity| {
                    CanonicalValue::Array(vec![signed(entity.value), point_set(&entity.points)])
                })
                .collect(),
        ),
    ])
    .encode_canonical()
    .map(|bytes| Hash32::digest(&bytes))
    .map_err(|_| EntityCorrespondenceVerifyError::Canonicalization)
}

fn unsigneds(values: &[u32]) -> CanonicalValue {
    CanonicalValue::Array(
        values
            .iter()
            .map(|value| CanonicalValue::Unsigned(u64::from(*value)))
            .collect(),
    )
}
fn point_set(points: &BTreeSet<Vec<i64>>) -> CanonicalValue {
    CanonicalValue::Array(
        points
            .iter()
            .map(|point| CanonicalValue::Array(point.iter().copied().map(signed).collect()))
            .collect(),
    )
}
fn signed(value: i64) -> CanonicalValue {
    if value >= 0 {
        CanonicalValue::Unsigned(value.unsigned_abs())
    } else {
        CanonicalValue::Negative(value)
    }
}
