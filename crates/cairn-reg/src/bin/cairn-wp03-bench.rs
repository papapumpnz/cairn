use cairn_core::{CanonicalEncode, Hash32};
use cairn_reg::{
    DerivationKind, DerivationRecord, EvidenceWeight, NodeConstraint, NodeKind, QueryPlan,
    RegLimits, RelGraphBuilder, RelationDefinition, RelationPattern, Term,
};
use std::hint::black_box;
use std::time::Instant;

const NODE_COUNT: usize = 2_000;
const EDGES_PER_NODE: usize = 5;
const PROBE_ITERATIONS: usize = 10_000_000;
const QUERY_ITERATIONS: usize = 5_000;

fn main() {
    let mut builder = RelGraphBuilder::new(RegLimits::default());
    let observed = builder
        .add_derivation(DerivationRecord {
            kind: DerivationKind::Observed,
            operation: Hash32::digest(b"wp03-benchmark-observation"),
            inputs: Vec::new(),
            parameters: Vec::new(),
            semantics_version: "wp03-benchmark/1".to_owned(),
        })
        .expect("derivation");
    let linked = builder
        .declare_relation(RelationDefinition {
            name: "linked".to_owned(),
            arguments: vec![
                NodeConstraint::Kind(NodeKind::EntityHypothesis),
                NodeConstraint::Kind(NodeKind::EntityHypothesis),
            ],
        })
        .expect("relation");
    let nodes = (0..NODE_COUNT)
        .map(|_| {
            builder
                .add_node(NodeKind::EntityHypothesis, Vec::new(), observed)
                .expect("node")
        })
        .collect::<Vec<_>>();
    for from in 0..NODE_COUNT {
        for offset in 1..=EDGES_PER_NODE {
            builder
                .add_fact(
                    linked,
                    vec![nodes[from], nodes[(from + offset) % NODE_COUNT]],
                    Vec::new(),
                    EvidenceWeight::Exact,
                    observed,
                )
                .expect("fact");
        }
    }
    let snapshot = builder.publish();
    let graph = snapshot.graph();
    let index = graph.relation_index(linked).expect("index");

    let probe_start = Instant::now();
    let mut checksum = 0_usize;
    for iteration in 0..PROBE_ITERATIONS {
        checksum = checksum.wrapping_add(index.at(0, nodes[iteration % NODE_COUNT]).len());
    }
    black_box(checksum);
    let probe_elapsed = probe_start.elapsed();
    let probes_per_second = 10_000_000.0 / probe_elapsed.as_secs_f64();

    let plan = QueryPlan {
        variable_count: 2,
        patterns: vec![
            RelationPattern {
                relation: linked,
                arguments: vec![Term::Node(nodes[0]), Term::Variable(0)],
            },
            RelationPattern {
                relation: linked,
                arguments: vec![Term::Variable(0), Term::Variable(1)],
            },
        ],
        attribute_filters: Vec::new(),
        path_constraints: Vec::new(),
        result_variables: vec![0, 1],
    };
    let mut query_times = Vec::with_capacity(QUERY_ITERATIONS);
    let mut row_checksum = 0_usize;
    for _ in 0..QUERY_ITERATIONS {
        let start = Instant::now();
        let result = plan.execute(graph).expect("query");
        query_times.push(start.elapsed().as_nanos());
        row_checksum = row_checksum.wrapping_add(result.rows.len());
    }
    black_box(row_checksum);
    query_times.sort_unstable();
    let median_query_ns = query_times[QUERY_ITERATIONS / 2];
    let p95_query_ns = query_times[QUERY_ITERATIONS * 95 / 100];
    let canonical_bytes = snapshot.encode_canonical().expect("canonical graph").len();
    let rss_kib = current_rss_kib().unwrap_or(0);

    println!(
        "{{\"nodes\":{},\"facts\":{},\"probe_iterations\":{},\"probe_elapsed_ns\":{},\"indexed_probes_per_second\":{:.3},\"query_iterations\":{},\"median_two_join_ns\":{},\"p95_two_join_ns\":{},\"canonical_bytes\":{},\"rss_kib\":{},\"checksum\":{}}}",
        graph.node_count(),
        graph.fact_count(),
        PROBE_ITERATIONS,
        probe_elapsed.as_nanos(),
        probes_per_second,
        QUERY_ITERATIONS,
        median_query_ns,
        p95_query_ns,
        canonical_bytes,
        rss_kib,
        checksum ^ row_checksum
    );
}

fn current_rss_kib() -> Option<u64> {
    let status = std::fs::read_to_string("/proc/self/status").ok()?;
    let line = status.lines().find(|line| line.starts_with("VmRSS:"))?;
    line.split_whitespace().nth(1)?.parse().ok()
}
