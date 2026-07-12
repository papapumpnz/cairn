use cairn_core::{CanonicalEncode, CanonicalValue, Hash32};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GeometricEntity {
    pub identity: Hash32,
    pub value: i64,
    pub points: BTreeSet<Vec<i64>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EntityGeometryScene {
    pub extents: Vec<u32>,
    pub background: i64,
    pub entities: Vec<GeometricEntity>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RenderedEntity {
    pub value: i64,
    pub points: BTreeSet<Vec<i64>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EntityGeometryRender {
    pub extents: Vec<u32>,
    pub background: i64,
    pub entities: Vec<RenderedEntity>,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum CorrespondenceFrame {
    PreserveObservation,
    TightCollection,
}

/// A source-independent bijection between coordinate axes plus explicit frame.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct EntityCorrespondenceMechanism {
    pub output_axis_sources: Vec<u32>,
    pub reversed_output_axes: BTreeSet<u32>,
    pub frame: CorrespondenceFrame,
}

impl EntityCorrespondenceMechanism {
    /// Canonical identity excluding source identities, values, and examples.
    ///
    /// # Errors
    ///
    /// Returns canonical encoding failure.
    pub fn semantic_hash(&self) -> Result<Hash32, cairn_core::CanonicalError> {
        self.encode_canonical().map(|bytes| Hash32::digest(&bytes))
    }
}

impl CanonicalEncode for EntityCorrespondenceMechanism {
    fn encode_canonical(&self) -> Result<Vec<u8>, cairn_core::CanonicalError> {
        CanonicalValue::Array(vec![
            CanonicalValue::Text("cairn.entity-correspondence.v1".to_owned()),
            CanonicalValue::Array(
                self.output_axis_sources
                    .iter()
                    .map(|axis| CanonicalValue::Unsigned(u64::from(*axis)))
                    .collect(),
            ),
            CanonicalValue::Array(
                self.reversed_output_axes
                    .iter()
                    .map(|axis| CanonicalValue::Unsigned(u64::from(*axis)))
                    .collect(),
            ),
            CanonicalValue::Unsigned(match self.frame {
                CorrespondenceFrame::PreserveObservation => 0,
                CorrespondenceFrame::TightCollection => 1,
            }),
        ])
        .encode_canonical()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PointConstructionProvenance {
    pub output: Vec<i64>,
    pub source: Vec<i64>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EntityConstructionProvenance {
    pub source_entity: Hash32,
    pub points: Vec<PointConstructionProvenance>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EntityConstruction {
    pub render: EntityGeometryRender,
    pub provenance: Vec<EntityConstructionProvenance>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EntityCorrespondenceExample {
    pub source: EntityGeometryScene,
    pub expected: EntityGeometryRender,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EntityCorrespondenceLimits {
    pub maximum_dimensions: usize,
    pub maximum_points: usize,
    pub maximum_candidates: usize,
}

impl Default for EntityCorrespondenceLimits {
    fn default() -> Self {
        Self {
            maximum_dimensions: 3,
            maximum_points: 65_536,
            maximum_candidates: 256,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LearnedEntityCorrespondence {
    pub mechanism: EntityCorrespondenceMechanism,
    pub candidates_evaluated: usize,
    pub examples_required: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EntityCorrespondenceError {
    EmptyExamples,
    InvalidScene,
    InvalidRender,
    DimensionCapacity,
    PointCapacity,
    CandidateCapacity,
    AmbiguousMechanism,
    NoMechanism,
    ConstructionConflict,
    Capacity,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct EntityCorrespondenceLearner;

impl EntityCorrespondenceLearner {
    /// Exhaustively learn one exact signed-axis and frame correspondence.
    ///
    /// # Errors
    ///
    /// Rejects invalid evidence, resource exhaustion, no exact explanation, or
    /// multiple semantically distinct exact mechanisms.
    pub fn learn(
        &self,
        examples: &[EntityCorrespondenceExample],
        limits: EntityCorrespondenceLimits,
    ) -> Result<LearnedEntityCorrespondence, EntityCorrespondenceError> {
        if examples.is_empty() {
            return Err(EntityCorrespondenceError::EmptyExamples);
        }
        for example in examples {
            validate_scene(&example.source, limits)?;
            validate_render(&example.expected, limits)?;
            if example.source.extents.len() != example.expected.extents.len() {
                return Err(EntityCorrespondenceError::InvalidRender);
            }
        }
        let dimensions = examples[0].source.extents.len();
        if examples
            .iter()
            .any(|example| example.source.extents.len() != dimensions)
        {
            return Err(EntityCorrespondenceError::InvalidScene);
        }
        let permutations = permutations(dimensions)?;
        let reversal_count = 1_usize
            .checked_shl(
                u32::try_from(dimensions).map_err(|_| EntityCorrespondenceError::Capacity)?,
            )
            .ok_or(EntityCorrespondenceError::Capacity)?;
        let mut matches = BTreeSet::new();
        let mut evaluated = 0_usize;
        for permutation in permutations {
            for reversal_mask in 0..reversal_count {
                for frame in [
                    CorrespondenceFrame::PreserveObservation,
                    CorrespondenceFrame::TightCollection,
                ] {
                    evaluated = evaluated
                        .checked_add(1)
                        .ok_or(EntityCorrespondenceError::Capacity)?;
                    if evaluated > limits.maximum_candidates {
                        return Err(EntityCorrespondenceError::CandidateCapacity);
                    }
                    let mechanism = EntityCorrespondenceMechanism {
                        output_axis_sources: permutation.clone(),
                        reversed_output_axes: (0..dimensions)
                            .filter(|axis| reversal_mask & (1_usize << axis) != 0)
                            .map(|axis| u32::try_from(axis).unwrap_or(u32::MAX))
                            .collect(),
                        frame,
                    };
                    if examples.iter().all(|example| {
                        self.execute(&mechanism, &example.source, limits)
                            .is_ok_and(|construction| construction.render == example.expected)
                    }) {
                        matches.insert(mechanism);
                    }
                }
            }
        }
        if matches.len() > 1 {
            return Err(EntityCorrespondenceError::AmbiguousMechanism);
        }
        let mechanism = matches
            .pop_first()
            .ok_or(EntityCorrespondenceError::NoMechanism)?;
        Ok(LearnedEntityCorrespondence {
            mechanism,
            candidates_evaluated: evaluated,
            examples_required: examples.len(),
        })
    }

    /// Execute correspondence and reconstruct every output point's source.
    ///
    /// # Errors
    ///
    /// Rejects malformed scenes/mechanisms, resource exhaustion, arithmetic
    /// failure, or conflicting entity construction.
    pub fn execute(
        &self,
        mechanism: &EntityCorrespondenceMechanism,
        source: &EntityGeometryScene,
        limits: EntityCorrespondenceLimits,
    ) -> Result<EntityConstruction, EntityCorrespondenceError> {
        validate_scene(source, limits)?;
        validate_mechanism(mechanism, source.extents.len())?;
        let dimensions = source.extents.len();
        let minima = coordinate_extrema(source, dimensions, true)?;
        let maxima = coordinate_extrema(source, dimensions, false)?;
        let output_extents = mechanism
            .output_axis_sources
            .iter()
            .map(|source_axis| {
                let axis = usize::try_from(*source_axis)
                    .map_err(|_| EntityCorrespondenceError::Capacity)?;
                match mechanism.frame {
                    CorrespondenceFrame::PreserveObservation => Ok(source.extents[axis]),
                    CorrespondenceFrame::TightCollection => maxima[axis]
                        .checked_sub(minima[axis])
                        .and_then(|span| span.checked_add(1))
                        .and_then(|extent| u32::try_from(extent).ok())
                        .ok_or(EntityCorrespondenceError::Capacity),
                }
            })
            .collect::<Result<Vec<_>, _>>()?;
        let mut occupied = BTreeMap::<Vec<i64>, Hash32>::new();
        let mut rendered = Vec::with_capacity(source.entities.len());
        let mut provenance = Vec::with_capacity(source.entities.len());
        for entity in &source.entities {
            let mut target_points = BTreeSet::new();
            let mut point_provenance = Vec::with_capacity(entity.points.len());
            for point in &entity.points {
                let output = transform_point(mechanism, source, point, &minima, &maxima)?;
                if occupied
                    .insert(output.clone(), entity.identity)
                    .is_some_and(|owner| owner != entity.identity)
                {
                    return Err(EntityCorrespondenceError::ConstructionConflict);
                }
                target_points.insert(output.clone());
                point_provenance.push(PointConstructionProvenance {
                    output,
                    source: point.clone(),
                });
            }
            point_provenance.sort_by(|left, right| left.output.cmp(&right.output));
            rendered.push(RenderedEntity {
                value: entity.value,
                points: target_points,
            });
            provenance.push(EntityConstructionProvenance {
                source_entity: entity.identity,
                points: point_provenance,
            });
        }
        rendered.sort_by(|left, right| {
            left.value
                .cmp(&right.value)
                .then_with(|| left.points.cmp(&right.points))
        });
        provenance.sort_by_key(|item| item.source_entity);
        Ok(EntityConstruction {
            render: EntityGeometryRender {
                extents: output_extents,
                background: source.background,
                entities: rendered,
            },
            provenance,
        })
    }
}

fn coordinate_extrema(
    scene: &EntityGeometryScene,
    dimensions: usize,
    minimum: bool,
) -> Result<Vec<i64>, EntityCorrespondenceError> {
    (0..dimensions)
        .map(|axis| {
            let values = scene
                .entities
                .iter()
                .flat_map(|entity| &entity.points)
                .map(|point| point[axis]);
            if minimum { values.min() } else { values.max() }
                .ok_or(EntityCorrespondenceError::InvalidScene)
        })
        .collect()
}

fn transform_point(
    mechanism: &EntityCorrespondenceMechanism,
    scene: &EntityGeometryScene,
    point: &[i64],
    minima: &[i64],
    maxima: &[i64],
) -> Result<Vec<i64>, EntityCorrespondenceError> {
    mechanism
        .output_axis_sources
        .iter()
        .copied()
        .enumerate()
        .map(|(output_axis, source_axis)| {
            let source_axis =
                usize::try_from(source_axis).map_err(|_| EntityCorrespondenceError::Capacity)?;
            let reversed = mechanism.reversed_output_axes.contains(
                &u32::try_from(output_axis).map_err(|_| EntityCorrespondenceError::Capacity)?,
            );
            match (mechanism.frame, reversed) {
                (CorrespondenceFrame::PreserveObservation, false) => Ok(point[source_axis]),
                (CorrespondenceFrame::PreserveObservation, true) => i64::from(
                    scene.extents[source_axis]
                        .checked_sub(1)
                        .ok_or(EntityCorrespondenceError::Capacity)?,
                )
                .checked_sub(point[source_axis])
                .ok_or(EntityCorrespondenceError::Capacity),
                (CorrespondenceFrame::TightCollection, false) => point[source_axis]
                    .checked_sub(minima[source_axis])
                    .ok_or(EntityCorrespondenceError::Capacity),
                (CorrespondenceFrame::TightCollection, true) => maxima[source_axis]
                    .checked_sub(point[source_axis])
                    .ok_or(EntityCorrespondenceError::Capacity),
            }
        })
        .collect()
}

fn validate_scene(
    scene: &EntityGeometryScene,
    limits: EntityCorrespondenceLimits,
) -> Result<(), EntityCorrespondenceError> {
    if scene.extents.is_empty()
        || scene.extents.len() > limits.maximum_dimensions
        || scene.extents.contains(&0)
        || scene.entities.is_empty()
    {
        return Err(EntityCorrespondenceError::InvalidScene);
    }
    let mut identities = BTreeSet::new();
    let mut total = 0_usize;
    for entity in &scene.entities {
        if !identities.insert(entity.identity) || entity.points.is_empty() {
            return Err(EntityCorrespondenceError::InvalidScene);
        }
        total = total
            .checked_add(entity.points.len())
            .ok_or(EntityCorrespondenceError::Capacity)?;
        if total > limits.maximum_points {
            return Err(EntityCorrespondenceError::PointCapacity);
        }
        for point in &entity.points {
            if point.len() != scene.extents.len()
                || point
                    .iter()
                    .zip(&scene.extents)
                    .any(|(coordinate, extent)| {
                        *coordinate < 0
                            || u64::try_from(*coordinate)
                                .map_or(true, |value| value >= u64::from(*extent))
                    })
            {
                return Err(EntityCorrespondenceError::InvalidScene);
            }
        }
    }
    Ok(())
}

fn validate_render(
    render: &EntityGeometryRender,
    limits: EntityCorrespondenceLimits,
) -> Result<(), EntityCorrespondenceError> {
    let scene = EntityGeometryScene {
        extents: render.extents.clone(),
        background: render.background,
        entities: render
            .entities
            .iter()
            .enumerate()
            .map(|(index, entity)| GeometricEntity {
                identity: Hash32::digest(&index.to_be_bytes()),
                value: entity.value,
                points: entity.points.clone(),
            })
            .collect(),
    };
    validate_scene(&scene, limits).map_err(|_| EntityCorrespondenceError::InvalidRender)
}

fn validate_mechanism(
    mechanism: &EntityCorrespondenceMechanism,
    dimensions: usize,
) -> Result<(), EntityCorrespondenceError> {
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
        return Err(EntityCorrespondenceError::InvalidScene);
    }
    Ok(())
}

fn permutations(dimensions: usize) -> Result<Vec<Vec<u32>>, EntityCorrespondenceError> {
    fn visit(
        prefix: &mut Vec<u32>,
        used: &mut BTreeSet<u32>,
        dimensions: usize,
        output: &mut Vec<Vec<u32>>,
    ) -> Result<(), EntityCorrespondenceError> {
        if prefix.len() == dimensions {
            output.push(prefix.clone());
            return Ok(());
        }
        for axis in 0..dimensions {
            let axis = u32::try_from(axis).map_err(|_| EntityCorrespondenceError::Capacity)?;
            if used.insert(axis) {
                prefix.push(axis);
                visit(prefix, used, dimensions, output)?;
                prefix.pop();
                used.remove(&axis);
            }
        }
        Ok(())
    }
    let mut output = Vec::new();
    visit(
        &mut Vec::new(),
        &mut BTreeSet::new(),
        dimensions,
        &mut output,
    )?;
    Ok(output)
}
