mod bridge;
mod cli;
mod error;
mod protocol;

use crate::error::{AppError, AppResult};
use clap::{Parser, Subcommand, ValueEnum};

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

    /// Import a local mesh file into Studio as welded MeshParts.
    ImportAsset {
        /// Studio name, substring, or UUID. Optional if exactly one Studio is connected.
        #[arg(long)]
        studio: Option<String>,

        /// Local mesh file to convert. OBJ/STL/glTF/GLB are native; other formats use Blender if available.
        #[arg(long)]
        file: std::path::PathBuf,

        /// Dot path of the Studio parent to insert under.
        #[arg(long, default_value = "Workspace")]
        to: String,

        /// Imported model name. Defaults to the local file stem.
        #[arg(long)]
        name: Option<String>,

        /// Multiplier applied to imported vertex positions.
        #[arg(long, default_value_t = 1.0)]
        scale: f32,

        /// Anchor generated MeshParts.
        #[arg(long)]
        anchored: bool,

        /// Do not add WeldConstraints between generated MeshParts.
        #[arg(long)]
        no_weld: bool,
    },

    /// Import a local PNG as Studio UI.
    ImportImage {
        /// Studio name, substring, or UUID. Optional if exactly one Studio is connected.
        #[arg(long)]
        studio: Option<String>,

        /// Local .png file to import.
        #[arg(long)]
        file: std::path::PathBuf,

        /// Dot path of the Studio UI parent to insert under.
        #[arg(long, default_value = "StarterGui")]
        to: String,

        /// Imported GUI object name. Defaults to the local file stem.
        #[arg(long)]
        name: Option<String>,

        /// UI object kind to create.
        #[arg(long, value_enum, default_value_t = ImageKind::Image)]
        kind: ImageKind,

        /// UI size in pixels, like 64x64. Defaults to the PNG size after import scaling.
        #[arg(long)]
        size: Option<String>,

        /// UI position in pixels, like 0,0.
        #[arg(long, default_value = "0,0")]
        position: String,
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

#[derive(Debug, Clone, ValueEnum)]
enum ImageKind {
    /// Non-clickable ImageLabel.
    Image,

    /// Clickable ImageButton.
    Button,

    /// Non-clickable icon-sized ImageLabel.
    Icon,
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
        Command::ImportAsset {
            studio,
            file,
            to,
            name,
            scale,
            anchored,
            no_weld,
        } => cli::import_asset::run(args.port, studio, file, to, name, scale, anchored, !no_weld),
        Command::ImportImage {
            studio,
            file,
            to,
            name,
            kind,
            size,
            position,
        } => cli::import_image::run(
            args.port,
            studio,
            file,
            to,
            name,
            kind.as_str().to_string(),
            size,
            position,
        ),
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

impl ImageKind {
    fn as_str(&self) -> &'static str {
        match self {
            ImageKind::Image => "image",
            ImageKind::Button => "button",
            ImageKind::Icon => "icon",
        }
    }
}
