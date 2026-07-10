use cairn_cas::{Cas, CasLimits};
use cairn_core::{decode_canonical, CanonicalEncode, CanonicalLimits, CanonicalValue};
use std::fs;
use std::hint::black_box;
use std::path::PathBuf;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

const ROUND_TRIPS: u64 = 10_000_000;
const BLOB_BYTES: usize = 1024 * 1024;
const READ_ITERATIONS: u64 = 512;
const ROUND_TRIPS_F64: f64 = 10_000_000.0;
const TOTAL_READ_BYTES_F64: f64 = 536_870_912.0;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = benchmark_root();
    let result = run(&root);
    let cleanup = fs::remove_dir_all(&root);
    result?;
    cleanup?;
    Ok(())
}

fn run(root: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let value = CanonicalValue::Array(vec![
        CanonicalValue::Unsigned(42),
        CanonicalValue::Text("stable".to_owned()),
    ]);
    let expected = value.encode_canonical()?;
    let started = Instant::now();
    let mut mismatches = 0_u64;
    for _ in 0..ROUND_TRIPS {
        let encoded = value.encode_canonical()?;
        let decoded = decode_canonical(&encoded, CanonicalLimits::default())?;
        let replay = decoded.encode_canonical()?;
        if replay != expected {
            mismatches += 1;
        }
        black_box(replay);
    }
    let round_trip_elapsed = started.elapsed();

    let cas = Cas::open(
        root,
        CasLimits {
            maximum_blob_bytes: 1_048_576,
            maximum_recovery_entries: 1024,
        },
    )?;
    let blob = deterministic_blob();
    let id = cas.put(&blob)?.id;
    for _ in 0..16 {
        black_box(cas.get(id)?);
    }
    let started = Instant::now();
    for _ in 0..READ_ITERATIONS {
        let loaded = cas.get(id)?;
        black_box(loaded);
    }
    let read_elapsed = started.elapsed();
    let throughput = TOTAL_READ_BYTES_F64 / read_elapsed.as_secs_f64();

    println!(
        concat!(
            "{{",
            "\"schema_version\":1,",
            "\"round_trips\":{},",
            "\"round_trip_mismatches\":{},",
            "\"round_trip_elapsed_ns\":{},",
            "\"round_trips_per_second\":{:.3},",
            "\"blob_bytes\":{},",
            "\"read_iterations\":{},",
            "\"read_elapsed_ns\":{},",
            "\"cached_read_bytes_per_second\":{:.3}",
            "}}"
        ),
        ROUND_TRIPS,
        mismatches,
        round_trip_elapsed.as_nanos(),
        ROUND_TRIPS_F64 / round_trip_elapsed.as_secs_f64(),
        BLOB_BYTES,
        READ_ITERATIONS,
        read_elapsed.as_nanos(),
        throughput,
    );
    Ok(())
}

fn deterministic_blob() -> Vec<u8> {
    let mut state = 0x243f_6a88_85a3_08d3_u64;
    (0..BLOB_BYTES)
        .map(|_| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state.to_le_bytes()[0]
        })
        .collect()
}

fn benchmark_root() -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_nanos());
    std::env::temp_dir().join(format!("cairn-wp01-bench-{}-{nonce}", std::process::id()))
}
