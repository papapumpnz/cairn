use cairn_core::{CanonicalEncode, CanonicalValue, Hash32};
use cairn_theory::{
    CoordinateLayer, LayerLearningExample, LayerMechanism, LayerRender, LayerRenderFrame,
    LayerRole, LayerScene, LayerSetExpression,
};
use std::collections::BTreeSet;

/// Opaque evidence that a modality-neutral layer mechanism independently
/// replayed multiple graphics experiences.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerifiedLayerMechanism {
    mechanism: Hash32,
    experiences: BTreeSet<Hash32>,
    replayed_points: u64,
    observed_dimensions: BTreeSet<usize>,
    observed_extents: BTreeSet<Vec<u32>>,
    observed_cardinalities: BTreeSet<Vec<usize>>,
    observed_backgrounds: BTreeSet<i64>,
    observed_foregrounds: BTreeSet<i64>,
}

impl VerifiedLayerMechanism {
    #[must_use]
    pub const fn mechanism(&self) -> Hash32 {
        self.mechanism
    }

    #[must_use]
    pub const fn experiences(&self) -> &BTreeSet<Hash32> {
        &self.experiences
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
    pub const fn observed_cardinalities(&self) -> &BTreeSet<Vec<usize>> {
        &self.observed_cardinalities
    }

    #[must_use]
    pub const fn observed_backgrounds(&self) -> &BTreeSet<i64> {
        &self.observed_backgrounds
    }

    #[must_use]
    pub const fn observed_foregrounds(&self) -> &BTreeSet<i64> {
        &self.observed_foregrounds
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LayerVerifyError {
    InsufficientIndependentEvidence,
    DuplicateExperience,
    InvalidScene,
    AmbiguousRole,
    EmptyConstruction,
    ReplayMismatch,
    Canonicalization,
    Capacity,
}

/// Separate exact reference implementation for coordinate selection, set
/// construction, categorical role binding, and renderer normalization.
#[derive(Clone, Copy, Debug, Default)]
pub struct LayerMechanismVerifier;

impl LayerMechanismVerifier {
    /// Independently replay a learned mechanism over at least two distinct
    /// source experiences.
    ///
    /// # Errors
    ///
    /// Rejects malformed scenes, ambiguous roles, duplicate evidence,
    /// canonicalization failure, and any output mismatch.
    pub fn verify(
        &self,
        mechanism: &LayerMechanism,
        examples: &[LayerLearningExample],
        maximum_points: usize,
    ) -> Result<VerifiedLayerMechanism, LayerVerifyError> {
        if examples.len() < 2 || maximum_points == 0 {
            return Err(LayerVerifyError::InsufficientIndependentEvidence);
        }
        let mechanism_hash = mechanism
            .semantic_hash()
            .map_err(|_| LayerVerifyError::Canonicalization)?;
        let mut experiences = BTreeSet::new();
        let mut replayed_points = 0_u64;
        let mut observed_dimensions = BTreeSet::new();
        let mut observed_extents = BTreeSet::new();
        let mut observed_cardinalities = BTreeSet::new();
        let mut observed_backgrounds = BTreeSet::new();
        let mut observed_foregrounds = BTreeSet::new();
        for example in examples {
            let experience = experience_hash(example)?;
            if !experiences.insert(experience) {
                return Err(LayerVerifyError::DuplicateExperience);
            }
            let actual = verifier_execute(mechanism, &example.scene, maximum_points)?;
            if actual != example.expected {
                return Err(LayerVerifyError::ReplayMismatch);
            }
            replayed_points = replayed_points
                .checked_add(
                    u64::try_from(actual.points.len()).map_err(|_| LayerVerifyError::Capacity)?,
                )
                .ok_or(LayerVerifyError::Capacity)?;
            observed_dimensions.insert(example.scene.extents.len());
            observed_extents.insert(example.scene.extents.clone());
            observed_cardinalities.insert(
                example
                    .scene
                    .layers
                    .iter()
                    .map(|layer| layer.points.len())
                    .collect(),
            );
            observed_backgrounds.insert(example.scene.background);
            observed_foregrounds.insert(example.expected.foreground);
        }
        Ok(VerifiedLayerMechanism {
            mechanism: mechanism_hash,
            experiences,
            replayed_points,
            observed_dimensions,
            observed_extents,
            observed_cardinalities,
            observed_backgrounds,
            observed_foregrounds,
        })
    }
}

fn verifier_execute(
    mechanism: &LayerMechanism,
    scene: &LayerScene,
    maximum_points: usize,
) -> Result<LayerRender, LayerVerifyError> {
    verifier_validate(scene, maximum_points)?;
    let points = verifier_expression(&mechanism.expression, scene)?;
    if points.len() > maximum_points {
        return Err(LayerVerifyError::Capacity);
    }
    let foreground = verifier_select(scene, mechanism.foreground)?.value;
    match mechanism.frame {
        LayerRenderFrame::PreserveSource => Ok(LayerRender {
            extents: scene.extents.clone(),
            background: scene.background,
            foreground,
            points,
        }),
        LayerRenderFrame::TightContent => {
            let dimensions = points
                .first()
                .map(Vec::len)
                .ok_or(LayerVerifyError::EmptyConstruction)?;
            let minima = (0..dimensions)
                .map(|axis| points.iter().map(|point| point[axis]).min())
                .collect::<Option<Vec<_>>>()
                .ok_or(LayerVerifyError::EmptyConstruction)?;
            let maxima = (0..dimensions)
                .map(|axis| points.iter().map(|point| point[axis]).max())
                .collect::<Option<Vec<_>>>()
                .ok_or(LayerVerifyError::EmptyConstruction)?;
            let extents = minima
                .iter()
                .zip(&maxima)
                .map(|(minimum, maximum)| {
                    maximum
                        .checked_sub(*minimum)
                        .and_then(|value| value.checked_add(1))
                        .and_then(|value| u32::try_from(value).ok())
                        .ok_or(LayerVerifyError::Capacity)
                })
                .collect::<Result<Vec<_>, _>>()?;
            let points = points
                .into_iter()
                .map(|point| {
                    point
                        .iter()
                        .zip(&minima)
                        .map(|(value, minimum)| {
                            value
                                .checked_sub(*minimum)
                                .ok_or(LayerVerifyError::Capacity)
                        })
                        .collect::<Result<Vec<_>, _>>()
                })
                .collect::<Result<BTreeSet<_>, _>>()?;
            Ok(LayerRender {
                extents,
                background: scene.background,
                foreground,
                points,
            })
        }
    }
}

fn verifier_validate(scene: &LayerScene, maximum_points: usize) -> Result<(), LayerVerifyError> {
    if scene.extents.is_empty() || scene.extents.contains(&0) || scene.layers.is_empty() {
        return Err(LayerVerifyError::InvalidScene);
    }
    let mut values = BTreeSet::new();
    let mut total = 0_usize;
    for layer in &scene.layers {
        if layer.value == scene.background || !values.insert(layer.value) {
            return Err(LayerVerifyError::InvalidScene);
        }
        total = total
            .checked_add(layer.points.len())
            .ok_or(LayerVerifyError::Capacity)?;
        if total > maximum_points {
            return Err(LayerVerifyError::Capacity);
        }
        for point in &layer.points {
            if point.len() != scene.extents.len()
                || point
                    .iter()
                    .zip(&scene.extents)
                    .any(|(coordinate, extent)| {
                        *coordinate < 0
                            || u64::try_from(*coordinate)
                                .map_or(true, |coordinate| coordinate >= u64::from(*extent))
                    })
            {
                return Err(LayerVerifyError::InvalidScene);
            }
        }
    }
    Ok(())
}

fn verifier_select(
    scene: &LayerScene,
    role: LayerRole,
) -> Result<&CoordinateLayer, LayerVerifyError> {
    let cardinality = match role {
        LayerRole::MinimumCardinality => scene.layers.iter().map(|layer| layer.points.len()).min(),
        LayerRole::MaximumCardinality => scene.layers.iter().map(|layer| layer.points.len()).max(),
    }
    .ok_or(LayerVerifyError::InvalidScene)?;
    let matching = scene
        .layers
        .iter()
        .filter(|layer| layer.points.len() == cardinality)
        .collect::<Vec<_>>();
    if matching.len() != 1 {
        return Err(LayerVerifyError::AmbiguousRole);
    }
    Ok(matching[0])
}

fn verifier_expression(
    expression: &LayerSetExpression,
    scene: &LayerScene,
) -> Result<BTreeSet<Vec<i64>>, LayerVerifyError> {
    match expression {
        LayerSetExpression::Input(role) => Ok(verifier_select(scene, *role)?.points.clone()),
        LayerSetExpression::Union(first, second) => Ok(verifier_select(scene, *first)?
            .points
            .union(&verifier_select(scene, *second)?.points)
            .cloned()
            .collect()),
        LayerSetExpression::Intersection(first, second) => Ok(verifier_select(scene, *first)?
            .points
            .intersection(&verifier_select(scene, *second)?.points)
            .cloned()
            .collect()),
        LayerSetExpression::Difference(first, second) => Ok(verifier_select(scene, *first)?
            .points
            .difference(&verifier_select(scene, *second)?.points)
            .cloned()
            .collect()),
    }
}

fn experience_hash(example: &LayerLearningExample) -> Result<Hash32, LayerVerifyError> {
    layer_example_value(example)
        .encode_canonical()
        .map(|bytes| Hash32::digest(&bytes))
        .map_err(|_| LayerVerifyError::Canonicalization)
}

fn layer_example_value(example: &LayerLearningExample) -> CanonicalValue {
    CanonicalValue::Array(vec![
        CanonicalValue::Text("cairn.layer-experience.v1".to_owned()),
        integer_array(example.scene.extents.iter().map(|value| i64::from(*value))),
        signed(example.scene.background),
        CanonicalValue::Array(
            example
                .scene
                .layers
                .iter()
                .map(|layer| {
                    CanonicalValue::Array(vec![signed(layer.value), point_array(&layer.points)])
                })
                .collect(),
        ),
        integer_array(
            example
                .expected
                .extents
                .iter()
                .map(|value| i64::from(*value)),
        ),
        signed(example.expected.background),
        signed(example.expected.foreground),
        point_array(&example.expected.points),
    ])
}

fn point_array(points: &BTreeSet<Vec<i64>>) -> CanonicalValue {
    CanonicalValue::Array(
        points
            .iter()
            .map(|point| integer_array(point.iter().copied()))
            .collect(),
    )
}

fn integer_array(values: impl Iterator<Item = i64>) -> CanonicalValue {
    CanonicalValue::Array(values.map(signed).collect())
}

fn signed(value: i64) -> CanonicalValue {
    if value >= 0 {
        CanonicalValue::Unsigned(value.unsigned_abs())
    } else {
        CanonicalValue::Negative(value)
    }
}
