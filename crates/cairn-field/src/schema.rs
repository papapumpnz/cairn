use crate::{FieldError, FiniteField};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

pub type SymbolNeighborhoodRule = (i64, i64, Vec<(isize, isize)>);

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum FieldTransformation {
    CategoricalMap {
        mappings: Vec<(i64, i64)>,
    },
    TensorMask {
        background: i64,
    },
    FillEnclosed {
        background: i64,
        fill: i64,
    },
    PeriodicTile {
        period_rows: usize,
        period_columns: usize,
    },
    RobustPeriodicRepair {
        noise: i64,
    },
    UniquePanel {
        background: i64,
    },
    MarkerRayBroadcast {
        background: i64,
    },
    RotationalCompletion {
        background: i64,
    },
    TileDiagonalHalo {
        background: i64,
        halo: i64,
    },
    SymbolNeighborhood {
        background: i64,
        rules: Vec<SymbolNeighborhoodRule>,
    },
    DiagonalPeriodic {
        background: i64,
    },
    Gravity {
        background: i64,
        direction: Direction,
    },
    FillSpans {
        background: i64,
        horizontal: bool,
        vertical: bool,
    },
    ComponentSizeRank {
        background: i64,
    },
    Attract {
        background: i64,
        mover: i64,
        anchor: i64,
    },
    PanelSpanCompletion {
        background: i64,
    },
    BoundaryAnchoredShear {
        background: i64,
    },
    TemplateRayRepeat {
        background: i64,
    },
    PanelIntersection {
        background: i64,
        separator: i64,
        fill: i64,
        vertical_split: bool,
    },
    PeriodicRows {
        added_rows: usize,
        from: i64,
        to: i64,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LearnedFieldTheory {
    pub transformation: FieldTransformation,
    pub demonstrations_replayed: usize,
    pub candidates_evaluated: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FieldSchemaError {
    EmptyDemonstrations,
    InvalidField(FieldError),
    NoTheory,
    Capacity,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct FieldSchemaLearner;

impl FieldSchemaLearner {
    /// Induce a parameterized finite-field mechanism and require exact replay
    /// across every supplied demonstration.
    ///
    /// # Errors
    ///
    /// Rejects empty/invalid evidence or reports that the current generic
    /// schema vocabulary cannot explain it.
    pub fn learn(
        &self,
        pairs: &[(FiniteField, FiniteField)],
    ) -> Result<LearnedFieldTheory, FieldSchemaError> {
        let Some((first_input, first_output)) = pairs.first() else {
            return Err(FieldSchemaError::EmptyDemonstrations);
        };
        for (input, output) in pairs {
            input.validate().map_err(FieldSchemaError::InvalidField)?;
            output.validate().map_err(FieldSchemaError::InvalidField)?;
            require_2d(input)?;
            require_2d(output)?;
        }
        let mut candidates = candidates(first_input, first_output)?;
        candidates.extend(aggregate_candidates(pairs));
        candidates.sort();
        candidates.dedup();
        let mut evaluated = 0_u64;
        for transformation in candidates {
            evaluated = evaluated.saturating_add(1);
            let exact = pairs.iter().all(|(input, output)| {
                apply_transformation(&transformation, input).as_ref() == Ok(output)
            });
            if exact {
                return Ok(LearnedFieldTheory {
                    transformation,
                    demonstrations_replayed: pairs.len(),
                    candidates_evaluated: evaluated,
                });
            }
        }
        Err(FieldSchemaError::NoTheory)
    }

    /// Apply a frozen learned mechanism to a new finite field.
    ///
    /// # Errors
    ///
    /// Returns invalid geometry or capacity failures.
    pub fn predict(
        &self,
        theory: &LearnedFieldTheory,
        input: &FiniteField,
    ) -> Result<FiniteField, FieldSchemaError> {
        input.validate().map_err(FieldSchemaError::InvalidField)?;
        apply_transformation(&theory.transformation, input)
    }
}

#[allow(clippy::too_many_lines)] // Candidate families remain visibly bounded and auditable here.
fn candidates(
    input: &FiniteField,
    output: &FiniteField,
) -> Result<Vec<FieldTransformation>, FieldSchemaError> {
    let backgrounds = values(input);
    let mut output_values = values(output);
    output_values.extend(backgrounds.iter().copied());
    let mut candidates = Vec::new();
    if output.shape[0] == input.shape[0].saturating_mul(2)
        && output.shape[1] == input.shape[1].saturating_mul(2)
    {
        for background in &backgrounds {
            for halo in &output_values {
                if halo != background {
                    candidates.push(FieldTransformation::TileDiagonalHalo {
                        background: *background,
                        halo: *halo,
                    });
                }
            }
        }
    }
    if output.shape[0] == input.shape[0].saturating_mul(input.shape[0])
        && output.shape[1] == input.shape[1].saturating_mul(input.shape[1])
    {
        candidates.extend(
            backgrounds
                .iter()
                .map(|background| FieldTransformation::TensorMask {
                    background: *background,
                }),
        );
    }
    if input.shape == output.shape {
        for background in &backgrounds {
            candidates.push(FieldTransformation::MarkerRayBroadcast {
                background: *background,
            });
            candidates.push(FieldTransformation::RotationalCompletion {
                background: *background,
            });
            for fill in &output_values {
                if fill != background {
                    candidates.push(FieldTransformation::FillEnclosed {
                        background: *background,
                        fill: *fill,
                    });
                }
            }
            for direction in [
                Direction::Up,
                Direction::Down,
                Direction::Left,
                Direction::Right,
            ] {
                candidates.push(FieldTransformation::Gravity {
                    background: *background,
                    direction,
                });
            }
            for (horizontal, vertical) in [(true, false), (false, true), (true, true)] {
                candidates.push(FieldTransformation::FillSpans {
                    background: *background,
                    horizontal,
                    vertical,
                });
            }
            candidates.push(FieldTransformation::ComponentSizeRank {
                background: *background,
            });
            candidates.push(FieldTransformation::DiagonalPeriodic {
                background: *background,
            });
            candidates.push(FieldTransformation::PanelSpanCompletion {
                background: *background,
            });
            candidates.push(FieldTransformation::BoundaryAnchoredShear {
                background: *background,
            });
            candidates.push(FieldTransformation::TemplateRayRepeat {
                background: *background,
            });
            for mover in &backgrounds {
                for anchor in &backgrounds {
                    if mover != background && anchor != background && mover != anchor {
                        candidates.push(FieldTransformation::Attract {
                            background: *background,
                            mover: *mover,
                            anchor: *anchor,
                        });
                    }
                }
            }
        }
        let rows = usize::try_from(input.shape[0]).map_err(|_| FieldSchemaError::Capacity)?;
        let columns = usize::try_from(input.shape[1]).map_err(|_| FieldSchemaError::Capacity)?;
        for period_rows in 1..=rows {
            for period_columns in 1..=columns {
                candidates.push(FieldTransformation::PeriodicTile {
                    period_rows,
                    period_columns,
                });
            }
        }
        candidates.extend(
            backgrounds
                .iter()
                .map(|noise| FieldTransformation::RobustPeriodicRepair { noise: *noise }),
        );
        candidates.extend(neighborhood_candidates(input, output)?);
    }
    candidates.extend(panel_candidates(input, output)?);
    for background in &backgrounds {
        candidates.push(FieldTransformation::UniquePanel {
            background: *background,
        });
    }
    if input.shape[1] == output.shape[1] && output.shape[0] > input.shape[0] {
        let added_rows = usize::try_from(output.shape[0] - input.shape[0])
            .map_err(|_| FieldSchemaError::Capacity)?;
        for from in &backgrounds {
            for to in &output_values {
                if from != to {
                    candidates.push(FieldTransformation::PeriodicRows {
                        added_rows,
                        from: *from,
                        to: *to,
                    });
                }
            }
        }
    }
    Ok(candidates)
}

fn aggregate_candidates(pairs: &[(FiniteField, FiniteField)]) -> Vec<FieldTransformation> {
    if pairs
        .iter()
        .any(|(input, output)| input.shape != output.shape)
    {
        return Vec::new();
    }
    let mut mapping = BTreeMap::<i64, i64>::new();
    let mut changed = false;
    for (input, output) in pairs {
        for (&source, &target) in input.values.iter().zip(&output.values) {
            changed |= source != target;
            if mapping
                .insert(source, target)
                .is_some_and(|old| old != target)
            {
                return Vec::new();
            }
        }
    }
    let targets = mapping.values().copied().collect::<BTreeSet<_>>();
    let sources = mapping.keys().copied().collect::<BTreeSet<_>>();
    if targets.len() != mapping.len() || targets != sources {
        return Vec::new();
    }
    changed
        .then(|| FieldTransformation::CategoricalMap {
            mappings: mapping.into_iter().collect(),
        })
        .into_iter()
        .collect()
}

fn neighborhood_candidates(
    input: &FiniteField,
    output: &FiniteField,
) -> Result<Vec<FieldTransformation>, FieldSchemaError> {
    let (rows, columns) = dimensions(input)?;
    let mut candidates = Vec::new();
    for background in values(input) {
        let mut rules = Vec::new();
        for trigger in values(input)
            .into_iter()
            .filter(|value| *value != background)
        {
            let positions = input
                .values
                .iter()
                .enumerate()
                .filter_map(|(index, value)| (*value == trigger).then_some(index))
                .collect::<Vec<_>>();
            if positions.len() != 1 {
                continue;
            }
            let origin = (positions[0] / columns, positions[0] % columns);
            let mut additions = BTreeMap::<i64, Vec<(isize, isize)>>::new();
            for row_offset in -1_isize..=1 {
                for column_offset in -1_isize..=1 {
                    if row_offset == 0 && column_offset == 0 {
                        continue;
                    }
                    let Some(row) = origin.0.checked_add_signed(row_offset) else {
                        continue;
                    };
                    let Some(column) = origin.1.checked_add_signed(column_offset) else {
                        continue;
                    };
                    if row < rows && column < columns {
                        let index = row * columns + column;
                        if input.values[index] == background && output.values[index] != background {
                            additions
                                .entry(output.values[index])
                                .or_default()
                                .push((row_offset, column_offset));
                        }
                    }
                }
            }
            if additions.len() == 1 {
                let (paint, mut offsets) = additions.into_iter().next().expect("one addition");
                offsets.sort_unstable();
                rules.push((trigger, paint, offsets));
            }
        }
        if !rules.is_empty() {
            rules.sort();
            candidates.push(FieldTransformation::SymbolNeighborhood { background, rules });
        }
    }
    Ok(candidates)
}

fn panel_candidates(
    input: &FiniteField,
    output: &FiniteField,
) -> Result<Vec<FieldTransformation>, FieldSchemaError> {
    let rows = usize::try_from(input.shape[0]).map_err(|_| FieldSchemaError::Capacity)?;
    let columns = usize::try_from(input.shape[1]).map_err(|_| FieldSchemaError::Capacity)?;
    let output_values = values(output);
    let backgrounds = values(input);
    let mut candidates = Vec::new();
    if columns % 2 == 1 && output.shape == vec![input.shape[0], input.shape[1] / 2] {
        let middle = columns / 2;
        if let Some(separator) = uniform_column(input, middle)? {
            for background in &backgrounds {
                for fill in &output_values {
                    if fill != background {
                        candidates.push(FieldTransformation::PanelIntersection {
                            background: *background,
                            separator,
                            fill: *fill,
                            vertical_split: true,
                        });
                    }
                }
            }
        }
    }
    if rows % 2 == 1 && output.shape == vec![input.shape[0] / 2, input.shape[1]] {
        let middle = rows / 2;
        if let Some(separator) = uniform_row(input, middle)? {
            for background in &backgrounds {
                for fill in &output_values {
                    if fill != background {
                        candidates.push(FieldTransformation::PanelIntersection {
                            background: *background,
                            separator,
                            fill: *fill,
                            vertical_split: false,
                        });
                    }
                }
            }
        }
    }
    Ok(candidates)
}

fn apply_transformation(
    transformation: &FieldTransformation,
    input: &FiniteField,
) -> Result<FiniteField, FieldSchemaError> {
    match transformation {
        FieldTransformation::CategoricalMap { mappings } => categorical_map(input, mappings),
        FieldTransformation::TensorMask { background } => tensor_mask(input, *background),
        FieldTransformation::FillEnclosed { background, fill } => {
            fill_enclosed(input, *background, *fill)
        }
        FieldTransformation::PeriodicTile {
            period_rows,
            period_columns,
        } => periodic_tile(input, *period_rows, *period_columns),
        FieldTransformation::RobustPeriodicRepair { noise } => {
            robust_periodic_repair(input, *noise)
        }
        FieldTransformation::UniquePanel { background } => unique_panel(input, *background),
        FieldTransformation::MarkerRayBroadcast { background } => {
            marker_ray_broadcast(input, *background)
        }
        FieldTransformation::RotationalCompletion { background } => {
            rotational_completion(input, *background)
        }
        FieldTransformation::TileDiagonalHalo { background, halo } => {
            tile_diagonal_halo(input, *background, *halo)
        }
        FieldTransformation::SymbolNeighborhood { background, rules } => {
            symbol_neighborhood(input, *background, rules)
        }
        FieldTransformation::DiagonalPeriodic { background } => {
            diagonal_periodic(input, *background)
        }
        FieldTransformation::Gravity {
            background,
            direction,
        } => gravity(input, *background, *direction),
        FieldTransformation::FillSpans {
            background,
            horizontal,
            vertical,
        } => fill_spans(input, *background, *horizontal, *vertical),
        FieldTransformation::ComponentSizeRank { background } => {
            component_size_rank(input, *background)
        }
        FieldTransformation::Attract {
            background,
            mover,
            anchor,
        } => attract(input, *background, *mover, *anchor),
        FieldTransformation::PanelSpanCompletion { background } => {
            panel_span_completion(input, *background)
        }
        FieldTransformation::BoundaryAnchoredShear { background } => {
            boundary_anchored_shear(input, *background)
        }
        FieldTransformation::TemplateRayRepeat { background } => {
            template_ray_repeat(input, *background)
        }
        FieldTransformation::PanelIntersection {
            background,
            separator,
            fill,
            vertical_split,
        } => panel_intersection(input, *background, *separator, *fill, *vertical_split),
        FieldTransformation::PeriodicRows {
            added_rows,
            from,
            to,
        } => periodic_rows(input, *added_rows, *from, *to),
    }
}

fn tensor_mask(input: &FiniteField, background: i64) -> Result<FiniteField, FieldSchemaError> {
    let (rows, columns) = dimensions(input)?;
    let output_rows = rows.checked_mul(rows).ok_or(FieldSchemaError::Capacity)?;
    let output_columns = columns
        .checked_mul(columns)
        .ok_or(FieldSchemaError::Capacity)?;
    let mut output = vec![background; output_rows * output_columns];
    for control_row in 0..rows {
        for control_column in 0..columns {
            if get(input, control_row, control_column)? == background {
                continue;
            }
            for row in 0..rows {
                for column in 0..columns {
                    output[(control_row * rows + row) * output_columns
                        + control_column * columns
                        + column] = get(input, row, column)?;
                }
            }
        }
    }
    field(output_rows, output_columns, output)
}

fn categorical_map(
    input: &FiniteField,
    mappings: &[(i64, i64)],
) -> Result<FiniteField, FieldSchemaError> {
    let mapping = mappings.iter().copied().collect::<BTreeMap<_, _>>();
    let mut output = input.clone();
    for value in &mut output.values {
        *value = *mapping.get(value).ok_or(FieldSchemaError::NoTheory)?;
    }
    Ok(output)
}

fn robust_periodic_repair(
    input: &FiniteField,
    noise: i64,
) -> Result<FiniteField, FieldSchemaError> {
    let (rows, columns) = dimensions(input)?;
    for area in 1..=rows.saturating_mul(columns) {
        for period_rows in 1..=rows {
            if area % period_rows != 0 {
                continue;
            }
            let period_columns = area / period_rows;
            if period_columns == 0 || period_columns > columns {
                continue;
            }
            let mut phases = BTreeMap::<(usize, usize), i64>::new();
            let mut conflict = false;
            for row in 0..rows {
                for column in 0..columns {
                    let value = get(input, row, column)?;
                    if value == noise {
                        continue;
                    }
                    let phase = (row % period_rows, column % period_columns);
                    if phases.insert(phase, value).is_some_and(|old| old != value) {
                        conflict = true;
                        break;
                    }
                }
            }
            if conflict || phases.len() != area {
                continue;
            }
            let mut values = Vec::with_capacity(rows * columns);
            for row in 0..rows {
                for column in 0..columns {
                    values.push(phases[&(row % period_rows, column % period_columns)]);
                }
            }
            return field(rows, columns, values);
        }
    }
    Err(FieldSchemaError::NoTheory)
}

fn unique_panel(input: &FiniteField, background: i64) -> Result<FiniteField, FieldSchemaError> {
    let (rows, columns) = dimensions(input)?;
    let separator_rows = (0..rows)
        .filter(|row| uniform_row(input, *row) == Ok(Some(background)))
        .collect::<Vec<_>>();
    let separator_columns = (0..columns)
        .filter(|column| uniform_column(input, *column) == Ok(Some(background)))
        .collect::<Vec<_>>();
    let row_ranges = ranges_between(rows, &separator_rows);
    let column_ranges = ranges_between(columns, &separator_columns);
    if row_ranges.len().saturating_mul(column_ranges.len()) < 2 {
        return Err(FieldSchemaError::NoTheory);
    }
    let mut panels = Vec::new();
    let mut signature_counts = BTreeMap::<BTreeSet<i64>, usize>::new();
    for row_range in &row_ranges {
        for column_range in &column_ranges {
            let signature = row_range
                .clone()
                .flat_map(|row| {
                    column_range
                        .clone()
                        .map(move |column| input.values[row * columns + column])
                })
                .filter(|value| *value != background)
                .collect::<BTreeSet<_>>();
            if !signature.is_empty() {
                *signature_counts.entry(signature.clone()).or_default() += 1;
                panels.push((row_range.clone(), column_range.clone(), signature));
            }
        }
    }
    let unique = panels
        .into_iter()
        .filter(|(_, _, signature)| signature_counts.get(signature) == Some(&1))
        .collect::<Vec<_>>();
    if unique.len() != 1 {
        return Err(FieldSchemaError::NoTheory);
    }
    let (row_range, column_range, _) = &unique[0];
    let values = row_range
        .clone()
        .flat_map(|row| {
            column_range
                .clone()
                .map(move |column| input.values[row * columns + column])
        })
        .collect();
    field(row_range.len(), column_range.len(), values)
}

fn marker_ray_broadcast(
    input: &FiniteField,
    background: i64,
) -> Result<FiniteField, FieldSchemaError> {
    let (rows, columns) = dimensions(input)?;
    let markers = input
        .values
        .iter()
        .enumerate()
        .filter_map(|(index, value)| {
            (*value != background).then_some((index / columns, index % columns, *value))
        })
        .collect::<Vec<_>>();
    if markers.len() != 2 || markers[0].2 == markers[1].2 {
        return Err(FieldSchemaError::NoTheory);
    }
    let row_distance = markers[0].0.abs_diff(markers[1].0);
    let column_distance = markers[0].1.abs_diff(markers[1].1);
    let horizontal = column_distance > 0 && (row_distance == 0 || column_distance < row_distance);
    let mut ordered = markers;
    ordered.sort_by_key(|marker| if horizontal { marker.1 } else { marker.0 });
    let start = if horizontal {
        ordered[0].1
    } else {
        ordered[0].0
    };
    let step = if horizontal {
        ordered[1].1 - ordered[0].1
    } else {
        ordered[1].0 - ordered[0].0
    };
    if step == 0 {
        return Err(FieldSchemaError::NoTheory);
    }
    let mut output = input.clone();
    output.values.fill(background);
    let extent = if horizontal { columns } else { rows };
    for (phase, coordinate) in (start..extent).step_by(step).enumerate() {
        let value = ordered[phase % 2].2;
        if horizontal {
            for row in 0..rows {
                output.values[row * columns + coordinate] = value;
            }
        } else {
            for column in 0..columns {
                output.values[coordinate * columns + column] = value;
            }
        }
    }
    Ok(output)
}

fn rotational_completion(
    input: &FiniteField,
    background: i64,
) -> Result<FiniteField, FieldSchemaError> {
    let (rows, columns) = dimensions(input)?;
    let cells = input
        .values
        .iter()
        .enumerate()
        .filter_map(|(index, value)| {
            (*value != background).then_some((index / columns, index % columns, *value))
        })
        .collect::<Vec<_>>();
    if cells.is_empty() {
        return Err(FieldSchemaError::NoTheory);
    }
    let min_row = cells.iter().map(|cell| cell.0).min().expect("nonempty");
    let max_row = cells.iter().map(|cell| cell.0).max().expect("nonempty");
    let min_column = cells.iter().map(|cell| cell.1).min().expect("nonempty");
    let max_column = cells.iter().map(|cell| cell.1).max().expect("nonempty");
    if max_row - min_row != max_column - min_column {
        return Err(FieldSchemaError::NoTheory);
    }
    let center_row_twice = min_row + max_row;
    let center_column_twice = min_column + max_column;
    let mut output = input.clone();
    for (row, column, value) in cells {
        let mut row_twice =
            2_isize * isize::try_from(row).map_err(|_| FieldSchemaError::Capacity)?;
        let mut column_twice =
            2_isize * isize::try_from(column).map_err(|_| FieldSchemaError::Capacity)?;
        let center_row =
            isize::try_from(center_row_twice).map_err(|_| FieldSchemaError::Capacity)?;
        let center_column =
            isize::try_from(center_column_twice).map_err(|_| FieldSchemaError::Capacity)?;
        for _ in 0..3 {
            let relative_row = row_twice - center_row;
            let relative_column = column_twice - center_column;
            row_twice = center_row + relative_column;
            column_twice = center_column - relative_row;
            if row_twice % 2 != 0 || column_twice % 2 != 0 {
                return Err(FieldSchemaError::NoTheory);
            }
            let target_row =
                usize::try_from(row_twice / 2).map_err(|_| FieldSchemaError::NoTheory)?;
            let target_column =
                usize::try_from(column_twice / 2).map_err(|_| FieldSchemaError::NoTheory)?;
            if target_row >= rows || target_column >= columns {
                return Err(FieldSchemaError::NoTheory);
            }
            output.values[target_row * columns + target_column] = value;
        }
    }
    Ok(output)
}

fn tile_diagonal_halo(
    input: &FiniteField,
    background: i64,
    halo: i64,
) -> Result<FiniteField, FieldSchemaError> {
    let (rows, columns) = dimensions(input)?;
    let output_rows = rows.checked_mul(2).ok_or(FieldSchemaError::Capacity)?;
    let output_columns = columns.checked_mul(2).ok_or(FieldSchemaError::Capacity)?;
    let mut source = vec![background; output_rows * output_columns];
    for row in 0..output_rows {
        for column in 0..output_columns {
            source[row * output_columns + column] = get(input, row % rows, column % columns)?;
        }
    }
    let mut output = source.clone();
    for row in 0..output_rows {
        for column in 0..output_columns {
            if source[row * output_columns + column] == background {
                continue;
            }
            for (row_offset, column_offset) in [(-1_isize, -1_isize), (-1, 1), (1, -1), (1, 1)] {
                if let (Some(target_row), Some(target_column)) = (
                    row.checked_add_signed(row_offset),
                    column.checked_add_signed(column_offset),
                ) {
                    if target_row < output_rows
                        && target_column < output_columns
                        && source[target_row * output_columns + target_column] == background
                    {
                        output[target_row * output_columns + target_column] = halo;
                    }
                }
            }
        }
    }
    field(output_rows, output_columns, output)
}

fn symbol_neighborhood(
    input: &FiniteField,
    background: i64,
    rules: &[SymbolNeighborhoodRule],
) -> Result<FiniteField, FieldSchemaError> {
    let (rows, columns) = dimensions(input)?;
    let mut output = input.clone();
    for (trigger, paint, offsets) in rules {
        for (index, value) in input.values.iter().enumerate() {
            if value != trigger {
                continue;
            }
            let origin = (index / columns, index % columns);
            for (row_offset, column_offset) in offsets {
                if let (Some(row), Some(column)) = (
                    origin.0.checked_add_signed(*row_offset),
                    origin.1.checked_add_signed(*column_offset),
                ) {
                    if row < rows
                        && column < columns
                        && output.values[row * columns + column] == background
                    {
                        output.values[row * columns + column] = *paint;
                    }
                }
            }
        }
    }
    Ok(output)
}

fn fill_enclosed(
    input: &FiniteField,
    background: i64,
    fill: i64,
) -> Result<FiniteField, FieldSchemaError> {
    let (rows, columns) = dimensions(input)?;
    let mut output = input.clone();
    let mut unseen = (0..rows * columns)
        .filter(|index| input.values[*index] == background)
        .collect::<BTreeSet<_>>();
    while let Some(seed) = unseen.pop_first() {
        let mut component = vec![seed];
        let mut queue = VecDeque::from([seed]);
        let mut boundary = false;
        while let Some(index) = queue.pop_front() {
            let row = index / columns;
            let column = index % columns;
            boundary |= row == 0 || column == 0 || row + 1 == rows || column + 1 == columns;
            for neighbor in neighbors(row, column, rows, columns) {
                let next = neighbor.0 * columns + neighbor.1;
                if unseen.remove(&next) {
                    component.push(next);
                    queue.push_back(next);
                }
            }
        }
        if !boundary {
            for index in component {
                output.values[index] = fill;
            }
        }
    }
    Ok(output)
}

fn periodic_tile(
    input: &FiniteField,
    period_rows: usize,
    period_columns: usize,
) -> Result<FiniteField, FieldSchemaError> {
    let (rows, columns) = dimensions(input)?;
    if period_rows == 0 || period_columns == 0 || period_rows > rows || period_columns > columns {
        return Err(FieldSchemaError::NoTheory);
    }
    let mut output = input.clone();
    for row in 0..rows {
        for column in 0..columns {
            output.values[row * columns + column] =
                get(input, row % period_rows, column % period_columns)?;
        }
    }
    Ok(output)
}

fn fill_spans(
    input: &FiniteField,
    background: i64,
    horizontal: bool,
    vertical: bool,
) -> Result<FiniteField, FieldSchemaError> {
    let (rows, columns) = dimensions(input)?;
    let source = input.clone();
    let mut output = input.clone();
    if horizontal {
        for row in 0..rows {
            let mut positions = BTreeMap::<i64, Vec<usize>>::new();
            for column in 0..columns {
                let value = get(&source, row, column)?;
                if value != background {
                    positions.entry(value).or_default().push(column);
                }
            }
            for (value, columns_with_value) in positions {
                if columns_with_value.len() >= 2 {
                    for column in
                        columns_with_value[0]..=*columns_with_value.last().expect("nonempty")
                    {
                        if output.values[row * columns + column] == background {
                            output.values[row * columns + column] = value;
                        }
                    }
                }
            }
        }
    }
    if vertical {
        for column in 0..columns {
            let mut positions = BTreeMap::<i64, Vec<usize>>::new();
            for row in 0..rows {
                let value = get(&source, row, column)?;
                if value != background {
                    positions.entry(value).or_default().push(row);
                }
            }
            for (value, rows_with_value) in positions {
                if rows_with_value.len() >= 2 {
                    for row in rows_with_value[0]..=*rows_with_value.last().expect("nonempty") {
                        if output.values[row * columns + column] == background {
                            output.values[row * columns + column] = value;
                        }
                    }
                }
            }
        }
    }
    Ok(output)
}

fn component_size_rank(
    input: &FiniteField,
    background: i64,
) -> Result<FiniteField, FieldSchemaError> {
    let mut components = components(input, background)?;
    components.sort_by_key(|component| (std::cmp::Reverse(component.len()), component[0]));
    let mut output = input.clone();
    for (rank, component) in components.iter().enumerate() {
        let value = i64::try_from(rank + 1).map_err(|_| FieldSchemaError::Capacity)?;
        for index in component {
            output.values[*index] = value;
        }
    }
    Ok(output)
}

fn diagonal_periodic(
    input: &FiniteField,
    background: i64,
) -> Result<FiniteField, FieldSchemaError> {
    let (rows, columns) = dimensions(input)?;
    for period in 2..=rows.max(columns) {
        let mut phases = BTreeMap::<usize, i64>::new();
        let mut conflict = false;
        for row in 0..rows {
            for column in 0..columns {
                let value = get(input, row, column)?;
                if value == background {
                    continue;
                }
                let phase = (row + column) % period;
                if phases.insert(phase, value).is_some_and(|old| old != value) {
                    conflict = true;
                    break;
                }
            }
        }
        if conflict || phases.len() != period {
            continue;
        }
        let mut output = input.clone();
        for row in 0..rows {
            for column in 0..columns {
                output.values[row * columns + column] = phases[&((row + column) % period)];
            }
        }
        return Ok(output);
    }
    Err(FieldSchemaError::NoTheory)
}

fn attract(
    input: &FiniteField,
    background: i64,
    mover: i64,
    anchor: i64,
) -> Result<FiniteField, FieldSchemaError> {
    let (rows, columns) = dimensions(input)?;
    let movers = components_for_value(input, mover)?;
    let anchors = components_for_value(input, anchor)?;
    if movers.is_empty() || anchors.is_empty() {
        return Err(FieldSchemaError::NoTheory);
    }
    let anchor_cells = anchors.into_iter().flatten().collect::<BTreeSet<_>>();
    let mut output = input.clone();
    for mover_component in movers {
        let direction = attraction_direction(&mover_component, &anchor_cells, columns)?;
        let mut cells = mover_component
            .iter()
            .map(|index| (*index / columns, *index % columns))
            .collect::<Vec<_>>();
        for (row, column) in &cells {
            output.values[*row * columns + *column] = background;
        }
        loop {
            let Some(next) = cells
                .iter()
                .map(|(row, column)| shifted_coordinate(*row, *column, direction))
                .collect::<Option<Vec<_>>>()
            else {
                break;
            };
            if next.iter().any(|(row, column)| {
                *row >= rows
                    || *column >= columns
                    || anchor_cells.contains(&(row * columns + column))
            }) {
                break;
            }
            cells = next;
        }
        for (row, column) in cells {
            output.values[row * columns + column] = mover;
        }
    }
    Ok(output)
}

fn panel_span_completion(
    input: &FiniteField,
    background: i64,
) -> Result<FiniteField, FieldSchemaError> {
    let (rows, columns) = dimensions(input)?;
    let separator_rows = (0..rows)
        .filter(|row| {
            uniform_row(input, *row).is_ok_and(|value| value.is_some_and(|item| item != background))
        })
        .collect::<Vec<_>>();
    let separator_columns = (0..columns)
        .filter(|column| {
            uniform_column(input, *column)
                .is_ok_and(|value| value.is_some_and(|item| item != background))
        })
        .collect::<Vec<_>>();
    if separator_rows.is_empty() || separator_columns.is_empty() {
        return Err(FieldSchemaError::NoTheory);
    }
    let row_ranges = ranges_between(rows, &separator_rows);
    let column_ranges = ranges_between(columns, &separator_columns);
    if row_ranges.len() < 2 || column_ranges.len() < 2 {
        return Err(FieldSchemaError::NoTheory);
    }
    let mut marks = vec![vec![None; column_ranges.len()]; row_ranges.len()];
    for (panel_row, row_range) in row_ranges.iter().enumerate() {
        for (panel_column, column_range) in column_ranges.iter().enumerate() {
            let values = row_range
                .clone()
                .flat_map(|row| {
                    column_range
                        .clone()
                        .map(move |column| input.values[row * columns + column])
                })
                .filter(|value| *value != background)
                .collect::<BTreeSet<_>>();
            if values.len() == 1 {
                marks[panel_row][panel_column] = values.first().copied();
            }
        }
    }
    let mut output = input.clone();
    for panel_row in 0..row_ranges.len() {
        complete_mark_spans(&marks[panel_row], |panel_column, value| {
            paint_panel(
                &mut output,
                &row_ranges[panel_row],
                &column_ranges[panel_column],
                background,
                value,
            );
        });
    }
    for panel_column in 0..column_ranges.len() {
        let column_marks = marks
            .iter()
            .map(|row| row[panel_column])
            .collect::<Vec<_>>();
        complete_mark_spans(&column_marks, |panel_row, value| {
            paint_panel(
                &mut output,
                &row_ranges[panel_row],
                &column_ranges[panel_column],
                background,
                value,
            );
        });
    }
    Ok(output)
}

fn boundary_anchored_shear(
    input: &FiniteField,
    background: i64,
) -> Result<FiniteField, FieldSchemaError> {
    let (_, columns) = dimensions(input)?;
    let objects = components(input, background)?;
    let mut output = input.clone();
    output.values.fill(background);
    for object in objects {
        let rows = object
            .iter()
            .map(|index| index / columns)
            .collect::<Vec<_>>();
        let object_columns = object
            .iter()
            .map(|index| index % columns)
            .collect::<Vec<_>>();
        let (_, maximum_row, _, maximum_column) = bounds(&rows, &object_columns)?;
        for index in object {
            let row = index / columns;
            let column = index % columns;
            let target_column = if row != maximum_row && column != maximum_column {
                column.checked_add(1).ok_or(FieldSchemaError::Capacity)?
            } else {
                column
            };
            if target_column >= columns {
                return Err(FieldSchemaError::NoTheory);
            }
            output.values[row * columns + target_column] = input.values[index];
        }
    }
    Ok(output)
}

#[allow(clippy::too_many_lines)] // Kept linear so every geometric safety check remains auditable.
fn template_ray_repeat(
    input: &FiniteField,
    background: i64,
) -> Result<FiniteField, FieldSchemaError> {
    let (rows, columns) = dimensions(input)?;
    let mut objects = components(input, background)?;
    if objects.len() < 2 {
        return Err(FieldSchemaError::NoTheory);
    }
    objects.sort_by_key(|component| (std::cmp::Reverse(component.len()), component[0]));
    if objects.len() > 1 && objects[0].len() == objects[1].len() {
        return Err(FieldSchemaError::NoTheory);
    }
    let template = objects.remove(0);
    let template_rows = template
        .iter()
        .map(|index| index / columns)
        .collect::<Vec<_>>();
    let template_columns = template
        .iter()
        .map(|index| index % columns)
        .collect::<Vec<_>>();
    let template_box = bounds(&template_rows, &template_columns)?;
    let height = template_box.1 - template_box.0 + 1;
    let width = template_box.3 - template_box.2 + 1;
    let shape = template
        .iter()
        .map(|index| {
            (
                index / columns - template_box.0,
                index % columns - template_box.2,
            )
        })
        .collect::<BTreeSet<_>>();
    let mut output = input.clone();
    for control in objects {
        let control_rows = control
            .iter()
            .map(|index| index / columns)
            .collect::<Vec<_>>();
        let control_columns = control
            .iter()
            .map(|index| index % columns)
            .collect::<Vec<_>>();
        let control_box = bounds(&control_rows, &control_columns)?;
        let row_step = ray_step(
            template_box.0,
            template_box.1,
            control_box.0,
            control_box.1,
            height,
        )?;
        let column_step = ray_step(
            template_box.2,
            template_box.3,
            control_box.2,
            control_box.3,
            width,
        )?;
        if row_step == 0 && column_step == 0 {
            return Err(FieldSchemaError::NoTheory);
        }
        let first_origin = (
            isize::try_from(template_box.0).map_err(|_| FieldSchemaError::Capacity)? + row_step,
            isize::try_from(template_box.2).map_err(|_| FieldSchemaError::Capacity)? + column_step,
        );
        if !control.iter().all(|index| {
            let row = isize::try_from(index / columns).ok();
            let column = isize::try_from(index % columns).ok();
            row.zip(column).is_some_and(|(row, column)| {
                let relative_row = row - first_origin.0;
                let relative_column = column - first_origin.1;
                usize::try_from(relative_row)
                    .ok()
                    .zip(usize::try_from(relative_column).ok())
                    .is_some_and(|relative| shape.contains(&relative))
            })
        }) {
            return Err(FieldSchemaError::NoTheory);
        }
        let value = input.values[control[0]];
        let maximum_steps = rows.saturating_add(columns).saturating_add(1);
        for multiplier in 1..=maximum_steps {
            let multiplier = isize::try_from(multiplier).map_err(|_| FieldSchemaError::Capacity)?;
            let origin_row = isize::try_from(template_box.0)
                .map_err(|_| FieldSchemaError::Capacity)?
                + row_step * multiplier;
            let origin_column = isize::try_from(template_box.2)
                .map_err(|_| FieldSchemaError::Capacity)?
                + column_step * multiplier;
            let mut in_bounds = false;
            for (relative_row, relative_column) in &shape {
                let row = origin_row
                    + isize::try_from(*relative_row).map_err(|_| FieldSchemaError::Capacity)?;
                let column = origin_column
                    + isize::try_from(*relative_column).map_err(|_| FieldSchemaError::Capacity)?;
                if let (Ok(row), Ok(column)) = (usize::try_from(row), usize::try_from(column)) {
                    if row < rows && column < columns {
                        output.values[row * columns + column] = value;
                        in_bounds = true;
                    }
                }
            }
            if !in_bounds {
                break;
            }
        }
    }
    Ok(output)
}

fn ray_step(
    template_minimum: usize,
    template_maximum: usize,
    control_minimum: usize,
    control_maximum: usize,
    extent: usize,
) -> Result<isize, FieldSchemaError> {
    if control_maximum < template_minimum {
        let gap = template_minimum - control_maximum - 1;
        isize::try_from(extent + gap)
            .map(|step| -step)
            .map_err(|_| FieldSchemaError::Capacity)
    } else if control_minimum > template_maximum {
        let gap = control_minimum - template_maximum - 1;
        isize::try_from(extent + gap).map_err(|_| FieldSchemaError::Capacity)
    } else {
        Ok(0)
    }
}

fn gravity(
    input: &FiniteField,
    background: i64,
    direction: Direction,
) -> Result<FiniteField, FieldSchemaError> {
    let (rows, columns) = dimensions(input)?;
    let mut objects = components(input, background)?;
    objects.sort_by_key(|component| gravity_order(component, columns, direction));
    let mut output = input.clone();
    for component in objects {
        let cells = component
            .iter()
            .map(|index| (*index / columns, *index % columns, input.values[*index]))
            .collect::<Vec<_>>();
        for (row, column, _) in &cells {
            output.values[*row * columns + *column] = background;
        }
        let mut shifted = cells;
        loop {
            let next = shifted
                .iter()
                .map(|(row, column, value)| {
                    shifted_coordinate(*row, *column, direction)
                        .map(|(row, column)| (row, column, *value))
                })
                .collect::<Option<Vec<_>>>();
            let Some(next) = next else { break };
            if next.iter().any(|(row, column, _)| {
                *row >= rows
                    || *column >= columns
                    || output.values[*row * columns + *column] != background
            }) {
                break;
            }
            shifted = next;
        }
        for (row, column, value) in shifted {
            output.values[row * columns + column] = value;
        }
    }
    Ok(output)
}

fn panel_intersection(
    input: &FiniteField,
    background: i64,
    separator: i64,
    fill: i64,
    vertical: bool,
) -> Result<FiniteField, FieldSchemaError> {
    let (rows, columns) = dimensions(input)?;
    if vertical {
        let middle = columns / 2;
        if uniform_column(input, middle)? != Some(separator) {
            return Err(FieldSchemaError::NoTheory);
        }
        let values = (0..rows)
            .flat_map(|row| {
                (0..middle).map(move |column| {
                    let left = input.values[row * columns + column];
                    let right = input.values[row * columns + middle + 1 + column];
                    if left != background && right != background {
                        fill
                    } else {
                        background
                    }
                })
            })
            .collect();
        field(rows, middle, values)
    } else {
        let middle = rows / 2;
        if uniform_row(input, middle)? != Some(separator) {
            return Err(FieldSchemaError::NoTheory);
        }
        let mut values = Vec::with_capacity(middle * columns);
        for row in 0..middle {
            for column in 0..columns {
                let top = get(input, row, column)?;
                let bottom = get(input, middle + 1 + row, column)?;
                values.push(if top != background && bottom != background {
                    fill
                } else {
                    background
                });
            }
        }
        field(middle, columns, values)
    }
}

fn periodic_rows(
    input: &FiniteField,
    added_rows: usize,
    from: i64,
    to: i64,
) -> Result<FiniteField, FieldSchemaError> {
    let (rows, columns) = dimensions(input)?;
    let period = (1..=rows)
        .find(|period| {
            (0..rows).all(|row| {
                (0..columns).all(|column| {
                    input.values[row * columns + column]
                        == input.values[(row % period) * columns + column]
                })
            })
        })
        .ok_or(FieldSchemaError::NoTheory)?;
    let output_rows = rows
        .checked_add(added_rows)
        .ok_or(FieldSchemaError::Capacity)?;
    let mut values = Vec::with_capacity(output_rows * columns);
    for row in 0..output_rows {
        for column in 0..columns {
            let value = get(input, row % period, column)?;
            values.push(if value == from { to } else { value });
        }
    }
    field(output_rows, columns, values)
}

fn components_for_value(
    input: &FiniteField,
    target: i64,
) -> Result<Vec<Vec<usize>>, FieldSchemaError> {
    let (rows, columns) = dimensions(input)?;
    let mut unseen = input
        .values
        .iter()
        .enumerate()
        .filter(|(_, value)| **value == target)
        .map(|(index, _)| index)
        .collect::<BTreeSet<_>>();
    let mut output = Vec::new();
    while let Some(seed) = unseen.pop_first() {
        let mut component = vec![seed];
        let mut queue = VecDeque::from([seed]);
        while let Some(index) = queue.pop_front() {
            for (row, column) in neighbors8(index / columns, index % columns, rows, columns) {
                let next = row * columns + column;
                if unseen.remove(&next) {
                    component.push(next);
                    queue.push_back(next);
                }
            }
        }
        output.push(component);
    }
    Ok(output)
}

fn attraction_direction(
    mover: &[usize],
    anchors: &BTreeSet<usize>,
    columns: usize,
) -> Result<Direction, FieldSchemaError> {
    let mover_rows = mover
        .iter()
        .map(|index| index / columns)
        .collect::<Vec<_>>();
    let mover_columns = mover
        .iter()
        .map(|index| index % columns)
        .collect::<Vec<_>>();
    let anchor_rows = anchors
        .iter()
        .map(|index| index / columns)
        .collect::<Vec<_>>();
    let anchor_columns = anchors
        .iter()
        .map(|index| index % columns)
        .collect::<Vec<_>>();
    let mover_box = bounds(&mover_rows, &mover_columns)?;
    let anchor_box = bounds(&anchor_rows, &anchor_columns)?;
    let row_overlap = mover_box.0 <= anchor_box.1 && anchor_box.0 <= mover_box.1;
    let column_overlap = mover_box.2 <= anchor_box.3 && anchor_box.2 <= mover_box.3;
    if column_overlap || !row_overlap {
        if mover_box.1 < anchor_box.0 {
            Ok(Direction::Down)
        } else if anchor_box.1 < mover_box.0 {
            Ok(Direction::Up)
        } else if mover_box.3 < anchor_box.2 {
            Ok(Direction::Right)
        } else {
            Ok(Direction::Left)
        }
    } else if mover_box.3 < anchor_box.2 {
        Ok(Direction::Right)
    } else {
        Ok(Direction::Left)
    }
}

fn bounds(
    rows: &[usize],
    columns: &[usize],
) -> Result<(usize, usize, usize, usize), FieldSchemaError> {
    Ok((
        *rows.iter().min().ok_or(FieldSchemaError::NoTheory)?,
        *rows.iter().max().ok_or(FieldSchemaError::NoTheory)?,
        *columns.iter().min().ok_or(FieldSchemaError::NoTheory)?,
        *columns.iter().max().ok_or(FieldSchemaError::NoTheory)?,
    ))
}

fn ranges_between(length: usize, separators: &[usize]) -> Vec<std::ops::Range<usize>> {
    let mut output = Vec::new();
    let mut start = 0;
    for separator in separators {
        if start < *separator {
            output.push(start..*separator);
        }
        start = separator + 1;
    }
    if start < length {
        output.push(start..length);
    }
    output
}

fn complete_mark_spans(marks: &[Option<i64>], mut paint: impl FnMut(usize, i64)) {
    let mut positions = BTreeMap::<i64, Vec<usize>>::new();
    for (index, mark) in marks.iter().enumerate() {
        if let Some(value) = mark {
            positions.entry(*value).or_default().push(index);
        }
    }
    for (value, indices) in positions {
        if indices.len() >= 2 {
            for (index, mark) in marks
                .iter()
                .enumerate()
                .take(*indices.last().expect("nonempty") + 1)
                .skip(indices[0])
            {
                if mark.is_none() {
                    paint(index, value);
                }
            }
        }
    }
}

fn paint_panel(
    field: &mut FiniteField,
    rows: &std::ops::Range<usize>,
    columns: &std::ops::Range<usize>,
    background: i64,
    value: i64,
) {
    let width = field.shape[1] as usize;
    for row in rows.clone() {
        for column in columns.clone() {
            let index = row * width + column;
            if field.values[index] == background {
                field.values[index] = value;
            }
        }
    }
}

fn components(input: &FiniteField, background: i64) -> Result<Vec<Vec<usize>>, FieldSchemaError> {
    let (rows, columns) = dimensions(input)?;
    let mut unseen = (0..input.values.len())
        .filter(|index| input.values[*index] != background)
        .collect::<BTreeSet<_>>();
    let mut output = Vec::new();
    while let Some(seed) = unseen.pop_first() {
        let mut component = vec![seed];
        let mut queue = VecDeque::from([seed]);
        while let Some(index) = queue.pop_front() {
            let row = index / columns;
            let column = index % columns;
            for (next_row, next_column) in neighbors8(row, column, rows, columns) {
                let next = next_row * columns + next_column;
                if unseen.remove(&next) {
                    component.push(next);
                    queue.push_back(next);
                }
            }
        }
        component.sort_unstable();
        output.push(component);
    }
    Ok(output)
}

fn values(field: &FiniteField) -> BTreeSet<i64> {
    field.values.iter().copied().collect()
}

fn dimensions(field: &FiniteField) -> Result<(usize, usize), FieldSchemaError> {
    require_2d(field)?;
    Ok((
        usize::try_from(field.shape[0]).map_err(|_| FieldSchemaError::Capacity)?,
        usize::try_from(field.shape[1]).map_err(|_| FieldSchemaError::Capacity)?,
    ))
}

fn require_2d(field: &FiniteField) -> Result<(), FieldSchemaError> {
    if field.shape.len() == 2 {
        Ok(())
    } else {
        Err(FieldSchemaError::NoTheory)
    }
}

fn get(field: &FiniteField, row: usize, column: usize) -> Result<i64, FieldSchemaError> {
    let (_, columns) = dimensions(field)?;
    field
        .values
        .get(row * columns + column)
        .copied()
        .ok_or(FieldSchemaError::Capacity)
}

fn field(rows: usize, columns: usize, values: Vec<i64>) -> Result<FiniteField, FieldSchemaError> {
    Ok(FiniteField {
        shape: vec![
            u32::try_from(rows).map_err(|_| FieldSchemaError::Capacity)?,
            u32::try_from(columns).map_err(|_| FieldSchemaError::Capacity)?,
        ],
        values,
    })
}

fn uniform_column(field: &FiniteField, column: usize) -> Result<Option<i64>, FieldSchemaError> {
    let (rows, _) = dimensions(field)?;
    let first = get(field, 0, column)?;
    Ok((1..rows)
        .all(|row| get(field, row, column) == Ok(first))
        .then_some(first))
}

fn uniform_row(field: &FiniteField, row: usize) -> Result<Option<i64>, FieldSchemaError> {
    let (_, columns) = dimensions(field)?;
    let first = get(field, row, 0)?;
    Ok((1..columns)
        .all(|column| get(field, row, column) == Ok(first))
        .then_some(first))
}

fn neighbors(row: usize, column: usize, rows: usize, columns: usize) -> Vec<(usize, usize)> {
    let mut output = Vec::with_capacity(4);
    if row > 0 {
        output.push((row - 1, column));
    }
    if row + 1 < rows {
        output.push((row + 1, column));
    }
    if column > 0 {
        output.push((row, column - 1));
    }
    if column + 1 < columns {
        output.push((row, column + 1));
    }
    output
}

fn neighbors8(row: usize, column: usize, rows: usize, columns: usize) -> Vec<(usize, usize)> {
    let mut output = Vec::with_capacity(8);
    for row_offset in [-1_isize, 0, 1] {
        for column_offset in [-1_isize, 0, 1] {
            if row_offset == 0 && column_offset == 0 {
                continue;
            }
            let Some(next_row) = row.checked_add_signed(row_offset) else {
                continue;
            };
            let Some(next_column) = column.checked_add_signed(column_offset) else {
                continue;
            };
            if next_row < rows && next_column < columns {
                output.push((next_row, next_column));
            }
        }
    }
    output
}

fn gravity_order(component: &[usize], columns: usize, direction: Direction) -> (usize, usize) {
    let rows = component.iter().map(|index| index / columns);
    let cols = component.iter().map(|index| index % columns);
    match direction {
        Direction::Down => (usize::MAX - rows.max().unwrap_or(0), component[0]),
        Direction::Up => (rows.min().unwrap_or(0), component[0]),
        Direction::Right => (usize::MAX - cols.max().unwrap_or(0), component[0]),
        Direction::Left => (cols.min().unwrap_or(0), component[0]),
    }
}

fn shifted_coordinate(row: usize, column: usize, direction: Direction) -> Option<(usize, usize)> {
    match direction {
        Direction::Up => row.checked_sub(1).map(|row| (row, column)),
        Direction::Down => row.checked_add(1).map(|row| (row, column)),
        Direction::Left => column.checked_sub(1).map(|column| (row, column)),
        Direction::Right => column.checked_add(1).map(|column| (row, column)),
    }
}
