use clap::Parser;

#[derive(Debug, Clone, clap::Parser)]
#[clap(version, about)]
#[clap(propagate_version = true)]
struct Cli {
    #[clap(subcommand)]
    commands: Commands,
}

#[derive(Debug, Clone, clap::Subcommand)]
enum Commands {
    /// Interact with the nextest runner.
    /// Will run the tests by default.
    #[clap(visible_alias = "t")]
    Test {
        /// The arguments to pass to nextest.
        args: Option<String>,
    },
    /// Debug
    #[clap(visible_alias = "lldb")]
    DebugLLDB {
        /// The arguments to pass to nextest.
        args: Option<String>,
    },
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Cli::parse();
    let mut _cargo_cmd = std::process::Command::new(
        std::env::var("XTASK_CARGO_CMD").unwrap_or_else(|_| "cargo".into()),
    );
    match args.commands {
        Commands::Test { .. } => {
            println!("TODO: nextest support");
            // cargo_cmd.args([
            //     "nextest",
            //     args.as_deref().unwrap_or("run"),
            // ]);
            // cargo_cmd.output()?;
        }
        Commands::DebugLLDB { .. } => {
            println!("TODO: lldb support");
        }
    }
    Ok(())
}
