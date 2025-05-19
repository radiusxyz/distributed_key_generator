

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().with_target(false).init();
    dkg_cli::run()?;
    Ok(())
}


