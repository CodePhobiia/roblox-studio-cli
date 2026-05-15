mod bridge;
mod cli;
mod error;
mod protocol;

use crate::error::{AppError, AppResult};
use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "rs", version, about = "Roblox Studio CLI + local bridge")]
struct Args {
    #[arg(long, env = "RS_BRIDGE_PORT", default_value_t = 7878, global = true)]
    port: u16,

    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// List connected Roblox Studio sessions.
    List {
        /// Emit JSON instead of a table.
        #[arg(long)]
        json: bool,
    },

    /// Execute Luau in a Studio session.
    Exec {
        /// Studio name, substring, or UUID. Optional if exactly one Studio is connected.
        #[arg(long)]
        studio: Option<String>,

        /// Luau source to execute. Use `return ...` to send a value back.
        #[arg(long)]
        lua: String,
    },

    /// Read a rich instance tree from a Studio session.
    Read {
        /// Studio name, substring, or UUID. Optional if exactly one Studio is connected.
        #[arg(long)]
        studio: Option<String>,

        /// Dot path such as Workspace or ReplicatedStorage.Modules.
        #[arg(long)]
        path: String,

        /// Descendant depth to include.
        #[arg(long, default_value_t = 3)]
        depth: u32,
    },

    /// Transfer an instance tree between two Studio sessions.
    Transfer {
        /// Source in `studio:path` form.
        #[arg(long)]
        from: String,

        /// Destination parent in `studio:parentPath` form.
        #[arg(long)]
        to: String,
    },

    /// Export Studio instances into individual local files.
    Export {
        /// Studio name, substring, or UUID. Optional if exactly one Studio is connected.
        #[arg(long)]
        studio: Option<String>,

        /// Dot path such as ServerStorage.SniperSkins or ReplicatedStorage.Modules.
        #[arg(long)]
        path: String,

        /// Output directory to write files into.
        #[arg(long)]
        out: std::path::PathBuf,

        /// Optional descendant depth. Omit to export the whole subtree.
        #[arg(long)]
        depth: Option<u32>,

        /// Overwrite existing files.
        #[arg(long)]
        overwrite: bool,
    },

    /// Manage the local bridge daemon.
    Bridge {
        #[command(subcommand)]
        command: BridgeCommand,
    },
}

#[derive(Debug, Subcommand)]
enum BridgeCommand {
    /// Run the bridge daemon in the foreground.
    Serve,

    /// Stop a running bridge daemon.
    Stop,

    /// Show bridge liveness and connected Studios.
    Status {
        /// Emit JSON instead of a table.
        #[arg(long)]
        json: bool,
    },
}

fn main() {
    if let Err(err) = run() {
        eprintln!("{err}");
        std::process::exit(err.exit_code());
    }
}

fn run() -> AppResult<()> {
    let args = Args::parse();

    match args.command {
        Command::List { json } => cli::list::run(args.port, json),
        Command::Exec { studio, lua } => cli::exec::run(args.port, studio, lua),
        Command::Read {
            studio,
            path,
            depth,
        } => cli::read::run(args.port, studio, path, depth),
        Command::Transfer { from, to } => cli::transfer::run(args.port, from, to),
        Command::Export {
            studio,
            path,
            out,
            depth,
            overwrite,
        } => cli::export::run(args.port, studio, path, out, depth, overwrite),
        Command::Bridge { command } => match command {
            BridgeCommand::Serve => {
                let runtime = tokio::runtime::Builder::new_multi_thread()
                    .enable_all()
                    .build()
                    .map_err(AppError::Io)?;
                runtime.block_on(bridge::server::serve(args.port))
            }
            BridgeCommand::Stop => cli::bridge::stop(args.port),
            BridgeCommand::Status { json } => cli::bridge::status(args.port, json),
        },
    }
}
