use cairn_arc::{parse_task, ArcGrid};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::path::Path;

fn main() {
    let Some(report_path) = env::args_os().nth(1) else {
        eprintln!("usage: cairn-arc-failure-audit REPORT.json CORPUS_DIRECTORY");
        std::process::exit(2);
    };
    let Some(corpus) = env::args_os().nth(2) else {
        eprintln!("usage: cairn-arc-failure-audit REPORT.json CORPUS_DIRECTORY");
        std::process::exit(2);
    };
    let report: Value =
        serde_json::from_slice(&fs::read(report_path).expect("report read")).expect("report JSON");
    let mut clusters = BTreeMap::<String, Vec<String>>::new();
    for item in report["reports"].as_array().expect("reports") {
        if item["status"].as_str() != Some("resource_limit") {
            continue;
        }
        let task = item["task"].as_str().expect("task");
        let path = Path::new(&corpus).join(format!("{task}.json"));
        let parsed = parse_task(&fs::read(path).expect("task read")).expect("task parse");
        clusters
            .entry(signature(
                &parsed
                    .train
                    .iter()
                    .map(|pair| (&pair.input, &pair.output))
                    .collect::<Vec<_>>(),
            ))
            .or_default()
            .push(task.to_owned());
    }
    let mut ranked = clusters.into_iter().collect::<Vec<_>>();
    ranked.sort_by_key(|(signature, tasks)| (std::cmp::Reverse(tasks.len()), signature.clone()));
    for (signature, tasks) in ranked {
        println!(
            "{}\t{}\t{}",
            tasks.len(),
            signature,
            tasks.iter().take(12).cloned().collect::<Vec<_>>().join(",")
        );
    }
}

fn signature(pairs: &[(&ArcGrid, &ArcGrid)]) -> String {
    let shapes = pairs
        .iter()
        .map(|(input, output)| {
            let input_cells = cells(input);
            let output_cells = cells(output);
            if dimensions(input) == dimensions(output) {
                "same"
            } else if output_cells < input_cells {
                "shrink"
            } else if output_cells > input_cells {
                "expand"
            } else {
                "reshape"
            }
        })
        .collect::<BTreeSet<_>>();
    let palette = if pairs
        .iter()
        .all(|(input, output)| values(output).is_subset(&values(input)))
    {
        "palette_subset"
    } else {
        "new_values"
    };
    let separators = if pairs.iter().all(|(input, _)| has_separator(input)) {
        "partitioned"
    } else {
        "unpartitioned"
    };
    let partition_relation = pairs
        .iter()
        .map(|(input, output)| partition_output_relation(input, output))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>()
        .join("+");
    let selection = panel_selection_rules(pairs);
    let density = if shapes == BTreeSet::from(["same"]) {
        let changed = pairs
            .iter()
            .map(|(input, output)| {
                input
                    .rows
                    .iter()
                    .flatten()
                    .zip(output.rows.iter().flatten())
                    .filter(|(left, right)| left != right)
                    .count()
            })
            .sum::<usize>();
        let total = pairs.iter().map(|(input, _)| cells(input)).sum::<usize>();
        if changed * 10 <= total {
            "sparse"
        } else if changed * 2 <= total {
            "medium"
        } else {
            "dense"
        }
    } else {
        "geometry"
    };
    format!(
        "shape={};{palette};{separators};partition_output={partition_relation};selection={selection};change={density}",
        shapes.into_iter().collect::<Vec<_>>().join("+")
    )
}

fn panel_selection_rules(pairs: &[(&ArcGrid, &ArcGrid)]) -> String {
    let analyses = pairs
        .iter()
        .map(|(input, output)| selected_panel_analysis(input, output))
        .collect::<Option<Vec<_>>>();
    let Some(analyses) = analyses else {
        return "none".to_owned();
    };
    let mut rules = Vec::new();
    let indices = analyses
        .iter()
        .map(|analysis| analysis.selected)
        .collect::<BTreeSet<_>>();
    if indices.len() == 1 {
        rules.push(format!("fixed_{}", analyses[0].selected));
    }
    for (name, predicate) in [
        (
            "min_occupied",
            analyses
                .iter()
                .all(|a| a.occupied[a.selected] == *a.occupied.iter().min().expect("panels")),
        ),
        (
            "max_occupied",
            analyses
                .iter()
                .all(|a| a.occupied[a.selected] == *a.occupied.iter().max().expect("panels")),
        ),
        (
            "min_distinct",
            analyses
                .iter()
                .all(|a| a.distinct[a.selected] == *a.distinct.iter().min().expect("panels")),
        ),
        (
            "max_distinct",
            analyses
                .iter()
                .all(|a| a.distinct[a.selected] == *a.distinct.iter().max().expect("panels")),
        ),
        (
            "omits_shared",
            analyses
                .iter()
                .all(|a| a.omitted_shared == Some(a.selected)),
        ),
    ] {
        if predicate {
            rules.push(name.to_owned());
        }
    }
    if rules.is_empty() {
        "unexplained".to_owned()
    } else {
        rules.join("+")
    }
}

struct PanelAnalysis {
    selected: usize,
    occupied: Vec<usize>,
    distinct: Vec<usize>,
    omitted_shared: Option<usize>,
}

fn selected_panel_analysis(input: &ArcGrid, output: &ArcGrid) -> Option<PanelAnalysis> {
    let (rows, columns) = dimensions(input);
    let separator_rows = (0..rows)
        .filter(|row| {
            input.rows[*row]
                .iter()
                .all(|value| *value == input.rows[*row][0])
        })
        .collect::<BTreeSet<_>>();
    let separator_columns = (0..columns)
        .filter(|column| (0..rows).all(|row| input.rows[row][*column] == input.rows[0][*column]))
        .collect::<BTreeSet<_>>();
    let row_ranges = nonseparator_ranges(rows, &separator_rows);
    let column_ranges = nonseparator_ranges(columns, &separator_columns);
    let panel_rows = equal_width(&row_ranges)?;
    let panel_columns = equal_width(&column_ranges)?;
    if dimensions(output) != (panel_rows, panel_columns) {
        return None;
    }
    let panels = row_ranges
        .iter()
        .flat_map(|(row_start, row_end)| {
            column_ranges.iter().map(move |(column_start, column_end)| {
                (*row_start..*row_end)
                    .flat_map(|row| {
                        (*column_start..*column_end).map(move |column| input.rows[row][column])
                    })
                    .collect::<Vec<_>>()
            })
        })
        .collect::<Vec<_>>();
    let output_values = output.rows.iter().flatten().copied().collect::<Vec<_>>();
    let matches = panels
        .iter()
        .enumerate()
        .filter_map(|(index, panel)| (panel == &output_values).then_some(index))
        .collect::<Vec<_>>();
    if matches.len() != 1 {
        return None;
    }
    let background = modal(input);
    let occupied = panels
        .iter()
        .map(|panel| panel.iter().filter(|value| **value != background).count())
        .collect();
    let distinct = panels
        .iter()
        .map(|panel| panel.iter().copied().collect::<BTreeSet<_>>().len())
        .collect();
    let all_values = panels.iter().flatten().copied().collect::<BTreeSet<_>>();
    let missing = all_values
        .iter()
        .filter_map(|value| {
            let absent = panels
                .iter()
                .enumerate()
                .filter_map(|(index, panel)| (!panel.contains(value)).then_some(index))
                .collect::<Vec<_>>();
            (absent.len() == 1).then(|| absent[0])
        })
        .collect::<BTreeSet<_>>();
    Some(PanelAnalysis {
        selected: matches[0],
        occupied,
        distinct,
        omitted_shared: (missing.len() == 1).then(|| *missing.first().expect("one")),
    })
}

fn modal(grid: &ArcGrid) -> u8 {
    let mut counts = BTreeMap::new();
    for value in grid.rows.iter().flatten() {
        *counts.entry(*value).or_insert(0_usize) += 1;
    }
    counts
        .into_iter()
        .max_by_key(|(value, count)| (*count, std::cmp::Reverse(*value)))
        .map_or(0, |(value, _)| value)
}

fn partition_output_relation(input: &ArcGrid, output: &ArcGrid) -> &'static str {
    let (rows, columns) = dimensions(input);
    let separator_rows = (0..rows)
        .filter(|row| {
            input.rows[*row]
                .iter()
                .all(|value| *value == input.rows[*row][0])
        })
        .collect::<BTreeSet<_>>();
    let separator_columns = (0..columns)
        .filter(|column| (0..rows).all(|row| input.rows[row][*column] == input.rows[0][*column]))
        .collect::<BTreeSet<_>>();
    let row_ranges = nonseparator_ranges(rows, &separator_rows);
    let column_ranges = nonseparator_ranges(columns, &separator_columns);
    let Some(panel_rows) = equal_width(&row_ranges) else {
        return "irregular";
    };
    let Some(panel_columns) = equal_width(&column_ranges) else {
        return "irregular";
    };
    let output_shape = dimensions(output);
    if output_shape == (panel_rows, panel_columns) {
        "one_panel"
    } else if output_shape == (row_ranges.len(), column_ranges.len()) {
        "panel_summary"
    } else if output_shape == (rows, columns) {
        "same_field"
    } else {
        "other"
    }
}

fn nonseparator_ranges(extent: usize, separators: &BTreeSet<usize>) -> Vec<(usize, usize)> {
    let mut ranges = Vec::new();
    let mut start = None;
    for index in 0..=extent {
        if index < extent && !separators.contains(&index) {
            start.get_or_insert(index);
        } else if let Some(start) = start.take() {
            ranges.push((start, index));
        }
    }
    ranges
}

fn equal_width(ranges: &[(usize, usize)]) -> Option<usize> {
    let width = ranges.first().map(|(start, end)| end - start)?;
    (!ranges.is_empty() && width > 0 && ranges.iter().all(|(start, end)| end - start == width))
        .then_some(width)
}

fn dimensions(grid: &ArcGrid) -> (usize, usize) {
    (grid.rows.len(), grid.rows.first().map_or(0, Vec::len))
}

fn cells(grid: &ArcGrid) -> usize {
    dimensions(grid).0 * dimensions(grid).1
}

fn values(grid: &ArcGrid) -> BTreeSet<u8> {
    grid.rows.iter().flatten().copied().collect()
}

fn has_separator(grid: &ArcGrid) -> bool {
    let (rows, columns) = dimensions(grid);
    (0..rows).any(|row| {
        grid.rows[row]
            .iter()
            .all(|value| *value == grid.rows[row][0])
    }) || (0..columns)
        .any(|column| (0..rows).all(|row| grid.rows[row][column] == grid.rows[0][column]))
}
