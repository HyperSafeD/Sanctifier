use clap::Args;
use serde_json::json;

#[derive(Args, Debug)]
pub struct VersionArgs {
    /// Output format: text (default) | json
    #[arg(long, default_value = "text")]
    pub format: String,
}

pub fn exec(args: VersionArgs) -> anyhow::Result<()> {
    let version = env!("CARGO_PKG_VERSION");
    let git_sha = env!("VERGEN_GIT_SHA");
    let build_date = env!("VERGEN_BUILD_DATE");

    if args.format == "json" {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "version": version,
                "commit": &git_sha[..12],
                "build_date": build_date,
            }))?
        );
    } else {
        println!("sanctifier {}", version);
        println!("commit: {}", &git_sha[..12]);
        println!("build date: {}", build_date);
    }

    Ok(())
}
