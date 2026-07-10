use cairn_cas::{
    Blake3ContentHasher, Cas, CasError, CasLimits, FaultInjector, PublicationPoint, PutStatus,
};
use std::path::PathBuf;
use std::sync::Arc;

const PAYLOAD: &[u8] = &[0x42; 4096];

#[derive(Debug)]
struct AbortAfterBlob;

impl FaultInjector for AbortAfterBlob {
    fn checkpoint(&self, point: PublicationPoint) -> Result<(), CasError> {
        if point == PublicationPoint::BlobPublished {
            std::process::abort();
        }
        Ok(())
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut arguments = std::env::args_os().skip(1);
    let root = arguments
        .next()
        .map(PathBuf::from)
        .ok_or("missing CAS root")?;
    let mode = arguments.next().ok_or("missing worker mode")?;
    if arguments.next().is_some() {
        return Err("unexpected worker argument".into());
    }
    let cas = if mode == "crash-after-blob" {
        Cas::open_with_components(
            root,
            CasLimits::default(),
            Arc::new(Blake3ContentHasher),
            Arc::new(AbortAfterBlob),
        )?
    } else if mode == "put" {
        Cas::open(root, CasLimits::default())?
    } else {
        return Err("unknown worker mode".into());
    };
    let outcome = cas.put(PAYLOAD)?;
    println!(
        "{} {}",
        match outcome.status {
            PutStatus::Stored => "STORED",
            PutStatus::AlreadyPresent => "ALREADY_PRESENT",
        },
        outcome.id
    );
    Ok(())
}
