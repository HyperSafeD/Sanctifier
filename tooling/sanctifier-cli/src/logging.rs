use tracing_subscriber::EnvFilter;

pub enum LogOutput {
    Text,
    Json,
}

pub fn init(output: LogOutput) -> anyhow::Result<()> {
    let env_filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new("warn"))
        .map_err(|err| anyhow::anyhow!("failed to configure log filter: {err}"))?;

    match output {
        LogOutput::Text => {
            tracing_subscriber::fmt()
                .with_env_filter(env_filter)
                .with_writer(std::io::stderr)
                .with_target(false)
                .without_time()
                .init();
        }
        LogOutput::Json => {
            tracing_subscriber::fmt()
                .json()
                .with_env_filter(env_filter)
                .with_writer(std::io::stderr)
                .with_current_span(false)
                .with_span_list(false)
                .init();
        }
    }

    Ok(())
}
