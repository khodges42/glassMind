use anyhow::{Result, anyhow};
use tracing_subscriber::{EnvFilter, fmt};

pub fn init(debug: bool) -> Result<()> {
    let default_level = if debug {
        "glassmind=debug"
    } else {
        "glassmind=info"
    };
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(default_level));

    fmt()
        .with_env_filter(filter)
        .with_target(debug)
        .with_file(debug)
        .with_line_number(debug)
        .compact()
        .try_init()
        .map_err(|err| anyhow!("failed to initialize logging: {err}"))?;

    Ok(())
}
