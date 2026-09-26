//! Offline utilities: seed inspection and golden-hash checks.
//!
//! Blocking IO only; never on the frame path.

#![forbid(unsafe_code)]

/// Salt for the golden-hash check, distinguishing tool output from gameplay.
const GOLDEN_SALT: u64 = 0x6F6C_6465_6E5F_3137;

#[global_allocator]
static GLOBAL_ALLOCATOR: mimalloc::MiMalloc = mimalloc::MiMalloc;

/// Print usage to standard error.
fn usage() {
    eprintln!("usage: universe-tools seed <u64>");
}

/// Inspect a seed: print its golden hash in hexadecimal.
///
/// # Errors
///
/// Returns an error when the subcommand or seed is invalid.
fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    if let Some("seed") = args.next().as_deref() {
        let raw = args.next().unwrap_or_default();
        let seed: u64 = raw
            .parse()
            .map_err(|error| anyhow::anyhow!("invalid seed {raw:?}: {error}"))?;
        println!("{:016x}", engine::generation::mix_seed(seed, GOLDEN_SALT));
        Ok(())
    } else {
        usage();
        Err(anyhow::anyhow!("expected subcommand: seed"))
    }
}
