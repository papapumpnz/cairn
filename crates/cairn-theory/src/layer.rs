use cairn_core::{CanonicalEncode, CanonicalValue, Hash32};
use std::collections::BTreeSet;

/// One categorical coordinate layer in an exact finite scene.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CoordinateLayer {
    pub value: i64,
    pub points: BTreeSet<Vec<i64>>,
}

/// Modality-neutral finite layer scene used by graphics and field frontends.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LayerScene {
    pub extents: Vec<u32>,
    pub background: i64,
    pub layers: Vec<CoordinateLayer>,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum LayerRole {
    MinimumCardinality,
    MaximumCardinality,
}

/// Executable coordinate-set construction synthesized from the set substrate.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum LayerSetExpression {
    Input(LayerRole),
    Union(LayerRole, LayerRole),
    Intersection(LayerRole, LayerRole),
    Difference(LayerRole, LayerRole),
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum LayerRenderFrame {
    PreserveSource,
    TightContent,
}

/// Learned selection, construction, categorical binding, and renderer policy.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct LayerMechanism {
    pub expression: LayerSetExpression,
    pub foreground: LayerRole,
    pub frame: LayerRenderFrame,
}

impl LayerMechanism {
    /// Canonical semantic identity excluding examples and modality metadata.
    ///
    /// # Errors
    ///
    /// Returns canonical encoding failure.
    pub fn semantic_hash(&self) -> Result<Hash32, cairn_core::CanonicalError> {
        self.encode_canonical().map(|bytes| Hash32::digest(&bytes))
    }
}

impl CanonicalEncode for LayerMechanism {
    fn encode_canonical(&self) -> Result<Vec<u8>, cairn_core::CanonicalError> {
        CanonicalValue::Array(vec![
            CanonicalValue::Text("cairn.layer-mechanism.v1".to_owned()),
            expression_value(&self.expression),
            CanonicalValue::Unsigned(role_code(self.foreground)),
            CanonicalValue::Unsigned(match self.frame {
                LayerRenderFrame::PreserveSource => 0,
                LayerRenderFrame::TightContent => 1,
            }),
        ])
        .encode_canonical()
    }
}

/// Exact semantic output of a layer mechanism before any modality realization.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LayerRender {
    pub extents: Vec<u32>,
    pub background: i64,
    pub foreground: i64,
    pub points: BTreeSet<Vec<i64>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LayerLearningExample {
    pub scene: LayerScene,
    pub expected: LayerRender,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LayerLearningLimits {
    pub maximum_candidates: u64,
    pub maximum_points: usize,
}

impl Default for LayerLearningLimits {
    fn default() -> Self {
        Self {
            maximum_candidates: 1_000,
            maximum_points: 65_536,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LayerMechanismError {
    EmptyExamples,
    EmptyBound,
    InvalidScene,
    DimensionMismatch,
    PointOutOfBounds,
    DuplicateLayerValue,
    AmbiguousRole,
    EmptyConstruction,
    Capacity,
    CandidateCapacity { maximum: u64 },
    NoMechanism,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct LayerMechanismLearner;

impl LayerMechanismLearner {
    /// Enumerate bounded set expressions and explicit renderer bindings, then
    /// retain the minimum-cost mechanism replaying every example exactly.
    ///
    /// # Errors
    ///
    /// Rejects malformed scenes, empty bounds, candidate exhaustion, and
    /// evidence not expressible by the declared general set grammar.
    pub fn learn(
        &self,
        examples: &[LayerLearningExample],
        limits: LayerLearningLimits,
    ) -> Result<LayerMechanism, LayerMechanismError> {
        if examples.is_empty() {
            return Err(LayerMechanismError::EmptyExamples);
        }
        if limits.maximum_candidates == 0 || limits.maximum_points == 0 {
            return Err(LayerMechanismError::EmptyBound);
        }
        for example in examples {
            validate_scene(&example.scene, limits.maximum_points)?;
        }
        let roles = [LayerRole::MinimumCardinality, LayerRole::MaximumCardinality];
        let mut expressions = roles
            .into_iter()
            .map(LayerSetExpression::Input)
            .collect::<Vec<_>>();
        for first in roles {
            for second in roles {
                if first == second {
                    continue;
                }
                expressions.extend([
                    LayerSetExpression::Intersection(first, second),
                    LayerSetExpression::Difference(first, second),
                    LayerSetExpression::Union(first, second),
                ]);
            }
        }
        expressions.sort_by(|left, right| {
            expression_cost(left)
                .cmp(&expression_cost(right))
                .then_with(|| left.cmp(right))
        });
        let mut evaluated = 0_u64;
        for expression in expressions {
            for foreground in roles {
                for frame in [
                    LayerRenderFrame::PreserveSource,
                    LayerRenderFrame::TightContent,
                ] {
                    evaluated = evaluated
                        .checked_add(1)
                        .ok_or(LayerMechanismError::Capacity)?;
                    if evaluated > limits.maximum_candidates {
                        return Err(LayerMechanismError::CandidateCapacity {
                            maximum: limits.maximum_candidates,
                        });
                    }
                    let candidate = LayerMechanism {
                        expression: expression.clone(),
                        foreground,
                        frame,
                    };
                    if examples.iter().all(|example| {
                        self.execute(&candidate, &example.scene, limits.maximum_points)
                            .as_ref()
                            == Ok(&example.expected)
                    }) {
                        return Ok(candidate);
                    }
                }
            }
        }
        Err(LayerMechanismError::NoMechanism)
    }

    /// Execute selection, coordinate construction, categorical rebinding, and
    /// explicit frame normalization through the exact reference path.
    ///
    /// # Errors
    ///
    /// Rejects malformed scenes, tied roles, empty tight constructions, and
    /// point or arithmetic capacity violations.
    pub fn execute(
        &self,
        mechanism: &LayerMechanism,
        scene: &LayerScene,
        maximum_points: usize,
    ) -> Result<LayerRender, LayerMechanismError> {
        validate_scene(scene, maximum_points)?;
        let points = evaluate_expression(&mechanism.expression, scene)?;
        if points.len() > maximum_points {
            return Err(LayerMechanismError::Capacity);
        }
        let foreground = select_layer(scene, mechanism.foreground)?.value;
        match mechanism.frame {
            LayerRenderFrame::PreserveSource => Ok(LayerRender {
                extents: scene.extents.clone(),
                background: scene.background,
                foreground,
                points,
            }),
            LayerRenderFrame::TightContent => tight_render(scene.background, foreground, points),
        }
    }
}

fn validate_scene(scene: &LayerScene, maximum_points: usize) -> Result<(), LayerMechanismError> {
    if scene.extents.is_empty() || scene.extents.contains(&0) || scene.layers.is_empty() {
        return Err(LayerMechanismError::InvalidScene);
    }
    let mut values = BTreeSet::new();
    let mut total = 0_usize;
    for layer in &scene.layers {
        if layer.value == scene.background || !values.insert(layer.value) {
            return Err(LayerMechanismError::DuplicateLayerValue);
        }
        total = total
            .checked_add(layer.points.len())
            .ok_or(LayerMechanismError::Capacity)?;
        if total > maximum_points {
            return Err(LayerMechanismError::Capacity);
        }
        for point in &layer.points {
            if point.len() != scene.extents.len() {
                return Err(LayerMechanismError::DimensionMismatch);
            }
            if point.iter().zip(&scene.extents).any(|(value, extent)| {
                *value < 0
                    || u64::try_from(*value).map_or(true, |value| value >= u64::from(*extent))
            }) {
                return Err(LayerMechanismError::PointOutOfBounds);
            }
        }
    }
    Ok(())
}

fn select_layer(
    scene: &LayerScene,
    role: LayerRole,
) -> Result<&CoordinateLayer, LayerMechanismError> {
    let target = match role {
        LayerRole::MinimumCardinality => scene.layers.iter().map(|layer| layer.points.len()).min(),
        LayerRole::MaximumCardinality => scene.layers.iter().map(|layer| layer.points.len()).max(),
    }
    .ok_or(LayerMechanismError::InvalidScene)?;
    let matches = scene
        .layers
        .iter()
        .filter(|layer| layer.points.len() == target)
        .collect::<Vec<_>>();
    if matches.len() != 1 {
        return Err(LayerMechanismError::AmbiguousRole);
    }
    Ok(matches[0])
}

fn evaluate_expression(
    expression: &LayerSetExpression,
    scene: &LayerScene,
) -> Result<BTreeSet<Vec<i64>>, LayerMechanismError> {
    match expression {
        LayerSetExpression::Input(role) => Ok(select_layer(scene, *role)?.points.clone()),
        LayerSetExpression::Union(first, second) => Ok(select_layer(scene, *first)?
            .points
            .union(&select_layer(scene, *second)?.points)
            .cloned()
            .collect()),
        LayerSetExpression::Intersection(first, second) => Ok(select_layer(scene, *first)?
            .points
            .intersection(&select_layer(scene, *second)?.points)
            .cloned()
            .collect()),
        LayerSetExpression::Difference(first, second) => Ok(select_layer(scene, *first)?
            .points
            .difference(&select_layer(scene, *second)?.points)
            .cloned()
            .collect()),
    }
}

fn tight_render(
    background: i64,
    foreground: i64,
    points: BTreeSet<Vec<i64>>,
) -> Result<LayerRender, LayerMechanismError> {
    let dimensions = points
        .first()
        .map(Vec::len)
        .ok_or(LayerMechanismError::EmptyConstruction)?;
    let minima = (0..dimensions)
        .map(|axis| points.iter().map(|point| point[axis]).min())
        .collect::<Option<Vec<_>>>()
        .ok_or(LayerMechanismError::EmptyConstruction)?;
    let maxima = (0..dimensions)
        .map(|axis| points.iter().map(|point| point[axis]).max())
        .collect::<Option<Vec<_>>>()
        .ok_or(LayerMechanismError::EmptyConstruction)?;
    let extents = minima
        .iter()
        .zip(&maxima)
        .map(|(minimum, maximum)| {
            maximum
                .checked_sub(*minimum)
                .and_then(|value| value.checked_add(1))
                .and_then(|value| u32::try_from(value).ok())
                .ok_or(LayerMechanismError::Capacity)
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
                        .ok_or(LayerMechanismError::Capacity)
                })
                .collect::<Result<Vec<_>, _>>()
        })
        .collect::<Result<BTreeSet<_>, _>>()?;
    Ok(LayerRender {
        extents,
        background,
        foreground,
        points,
    })
}

fn expression_cost(expression: &LayerSetExpression) -> u8 {
    match expression {
        LayerSetExpression::Input(_) => 1,
        LayerSetExpression::Intersection(_, _) | LayerSetExpression::Difference(_, _) => 3,
        LayerSetExpression::Union(_, _) => 4,
    }
}

const fn role_code(role: LayerRole) -> u64 {
    match role {
        LayerRole::MinimumCardinality => 0,
        LayerRole::MaximumCardinality => 1,
    }
}

fn expression_value(expression: &LayerSetExpression) -> CanonicalValue {
    match expression {
        LayerSetExpression::Input(role) => CanonicalValue::Array(vec![
            CanonicalValue::Unsigned(0),
            CanonicalValue::Unsigned(role_code(*role)),
        ]),
        LayerSetExpression::Union(first, second) => binary_expression_value(1, *first, *second),
        LayerSetExpression::Intersection(first, second) => {
            binary_expression_value(2, *first, *second)
        }
        LayerSetExpression::Difference(first, second) => {
            binary_expression_value(3, *first, *second)
        }
    }
}

fn binary_expression_value(code: u64, first: LayerRole, second: LayerRole) -> CanonicalValue {
    CanonicalValue::Array(vec![
        CanonicalValue::Unsigned(code),
        CanonicalValue::Unsigned(role_code(first)),
        CanonicalValue::Unsigned(role_code(second)),
    ])
}
