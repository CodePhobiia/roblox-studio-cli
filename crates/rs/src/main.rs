mod bridge;
mod cli;
mod error;
#[cfg(test)]
mod plugin_static_tests;
mod protocol;

use crate::error::{AppError, AppResult};
use clap::{Args as ClapArgs, Parser, Subcommand, ValueEnum};
use std::io::Write;

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

    /// Diagnose bridge, plugin install, and Studio plugin protocol health.
    Doctor {
        /// Copy the repo-built plugin into the local Roblox Plugins folder and start the bridge if needed.
        #[arg(long)]
        fix: bool,

        /// Emit JSON instead of text.
        #[arg(long)]
        json: bool,

        /// Output format.
        #[arg(long, value_enum)]
        format: Option<OutputFormat>,
    },

    /// Build and install the Studio plugin bundle.
    InstallPlugin {
        /// Keep rebuilding and reinstalling when plugin source files change.
        #[arg(long)]
        watch: bool,

        /// Emit JSON instead of text.
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

        /// Allow arbitrary Luau execution in Studio for this invocation.
        #[arg(long)]
        allow_dangerous_exec: bool,
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

        /// Serialize and conflict-check without mutating the destination.
        #[arg(long)]
        dry_run: bool,

        /// Replace an existing destination child with the transferred root name.
        #[arg(long)]
        replace: bool,

        /// Ask the plugin to restore replaced content if deserialize fails.
        #[arg(long)]
        rollback_on_error: bool,

        /// Allow transferred welds/constraints to omit refs that point outside the selected source root.
        #[arg(long)]
        allow_external_refs: bool,

        #[command(flatten)]
        image_rehost: ImageRehostArgs,
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

        /// Folder containing texture files or rs-textures.json remapping texture names to Roblox asset URIs.
        #[arg(long)]
        texture_root: Option<std::path::PathBuf>,
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

    /// Import a folder or manifest of PNG UI elements as a ScreenGui.
    ImportUiPack {
        /// Studio name, substring, or UUID. Optional if exactly one Studio is connected.
        #[arg(long)]
        studio: Option<String>,

        /// Folder of PNG files. Use this or --manifest.
        #[arg(long)]
        folder: Option<std::path::PathBuf>,

        /// UI pack manifest JSON. Use this or --folder.
        #[arg(long)]
        manifest: Option<std::path::PathBuf>,

        /// Dot path of the Studio UI parent to insert under.
        #[arg(long)]
        to: Option<String>,

        /// ScreenGui/container name. Defaults to the folder or manifest name.
        #[arg(long)]
        name: Option<String>,

        /// Default UI kind for folder mode.
        #[arg(long, value_enum, default_value_t = ImageKind::Image)]
        kind: ImageKind,

        /// Emit JSON instead of text.
        #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
        format: OutputFormat,
    },

    /// Import audio asset IDs as Sound instances.
    ImportAudio {
        /// Studio name, substring, or UUID. Optional if exactly one Studio is connected.
        #[arg(long)]
        studio: Option<String>,

        /// Local audio file used for naming/validation. Requires --asset-id.
        #[arg(long)]
        file: Option<std::path::PathBuf>,

        /// Audio manifest JSON.
        #[arg(long)]
        manifest: Option<std::path::PathBuf>,

        /// Dot path of the Studio parent to insert under.
        #[arg(long)]
        to: Option<String>,

        /// Imported Sound name. Defaults to the local file stem.
        #[arg(long)]
        name: Option<String>,

        /// Roblox audio asset id or rbxassetid:// URI.
        #[arg(long)]
        asset_id: Option<String>,

        /// Sound.Volume.
        #[arg(long)]
        volume: Option<f32>,

        /// Sound.PlaybackSpeed.
        #[arg(long)]
        playback_speed: Option<f32>,

        /// Set Sound.Looped.
        #[arg(long)]
        looped: bool,

        /// Emit JSON instead of text.
        #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
        format: OutputFormat,
    },

    /// Upload local image, audio, or model files through Roblox Open Cloud.
    Upload {
        #[command(subcommand)]
        command: UploadCommand,
    },

    /// Import an already uploaded Roblox asset ID into Studio.
    ImportUploaded {
        #[command(subcommand)]
        command: ImportUploadedCommand,
    },

    /// Manage local rs credentials and Open Cloud profiles.
    Auth {
        #[command(subcommand)]
        command: AuthCommand,
    },

    /// Validate a Studio subtree for broken refs, Tool readiness, and asset issues.
    Validate {
        /// Studio name, substring, or UUID. Optional if exactly one Studio is connected.
        #[arg(long)]
        studio: Option<String>,

        /// Dot path to validate.
        #[arg(long)]
        path: String,

        /// Comma-separated rule groups such as tool,welds,refs,assets.
        #[arg(long, value_delimiter = ',')]
        rules: Vec<String>,

        /// Output format.
        #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
        format: OutputFormat,

        /// Apply safe fixes, then rerun validation and report before/after.
        #[arg(long)]
        fix: bool,
    },

    /// Repair a Tool so its parts are welded to the Handle and equip-ready.
    RepairTool(RepairToolArgs),

    /// Alias for repair-tool, aimed at asset-pipeline usage.
    WireTool(RepairToolArgs),

    /// Generate a compact inventory of a Studio subtree.
    Snapshot {
        /// Studio name, substring, or UUID. Optional if exactly one Studio is connected.
        #[arg(long)]
        studio: Option<String>,

        /// Dot path to snapshot.
        #[arg(long)]
        path: String,

        /// Include every visited path in JSON output.
        #[arg(long)]
        include_paths: bool,

        /// Write JSON snapshot to a file.
        #[arg(long)]
        out: Option<std::path::PathBuf>,

        /// Output format.
        #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
        format: OutputFormat,
    },

    /// Run repeatable live Studio smoke fixtures.
    Smoke {
        #[command(subcommand)]
        command: SmokeCommand,
    },

    /// Create an Instance with optional typed properties.
    Create {
        /// Studio name, substring, or UUID. Optional if exactly one Studio is connected.
        #[arg(long)]
        studio: Option<String>,

        /// ClassName to create.
        #[arg(long = "class")]
        class_name: Option<String>,

        /// Dot path of parent.
        #[arg(long)]
        to: Option<String>,

        /// Instance name.
        #[arg(long)]
        name: Option<String>,

        /// Property assignment such as Anchored=true or Size=8,1,8.
        #[arg(long = "property")]
        properties: Vec<String>,

        /// JSON CreateInstanceRequest file.
        #[arg(long)]
        json: Option<std::path::PathBuf>,

        /// Output format.
        #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
        format: OutputFormat,
    },

    /// Diff two Studio/export sources.
    Diff {
        /// Left Studio source.
        #[arg(long)]
        studio: Option<String>,

        /// Left Studio path.
        #[arg(long)]
        path: Option<String>,

        /// Left export folder.
        #[arg(long = "export")]
        export_path: Option<std::path::PathBuf>,

        /// Right Studio source.
        #[arg(long)]
        against_studio: Option<String>,

        /// Right Studio path.
        #[arg(long)]
        against_path: Option<String>,

        /// Right export folder.
        #[arg(long)]
        against_export: Option<std::path::PathBuf>,

        /// Descendant depth for Studio reads.
        #[arg(long, default_value_t = 999)]
        depth: u32,

        /// Ignore script Source changes.
        #[arg(long)]
        ignore_scripts: bool,

        /// Ignore asset URI property changes.
        #[arg(long)]
        ignore_assets: bool,

        /// Emit a safe mutation plan derived from the diff.
        #[arg(long)]
        fix_plan: bool,

        /// Output format.
        #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
        format: OutputFormat,
    },

    /// Apply safe operations from an rs diff --fix-plan JSON file.
    ApplyPlan {
        /// Studio name, substring, or UUID. Optional if exactly one Studio is connected.
        #[arg(long)]
        studio: Option<String>,

        /// Studio root path that the fix-plan's relative paths apply under.
        #[arg(long)]
        root: String,

        /// Fix-plan JSON file produced by rs diff --fix-plan --format json.
        #[arg(long)]
        file: std::path::PathBuf,

        /// Report intended mutations without changing Studio.
        #[arg(long)]
        dry_run: bool,

        /// Approve non-dry-run mutation.
        #[arg(long)]
        yes: bool,

        /// Apply only these change kinds, such as added,modified.
        #[arg(long, value_enum, value_delimiter = ',')]
        only: Vec<PlanChangeKind>,

        /// Exclude classes or groups such as Scripts. Comma-separated values are accepted.
        #[arg(long, value_delimiter = ',')]
        exclude: Vec<String>,

        /// Permit overwriting user-owned/manual instances.
        #[arg(long)]
        force: bool,

        /// Output format.
        #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
        format: OutputFormat,
    },

    /// Plan, preview, apply, and report verified Studio feature changes.
    Autopilot {
        #[command(subcommand)]
        command: AutopilotCommand,
    },

    /// Sync between Studio and disk.
    Sync {
        #[command(subcommand)]
        command: SyncCommand,
    },

    /// Sync local scripts/assets into Studio once or in watch mode.
    SyncFolder {
        /// Studio name, substring, or UUID. Optional if exactly one Studio is connected.
        #[arg(long)]
        studio: Option<String>,

        /// Local folder to sync.
        #[arg(long)]
        folder: Option<std::path::PathBuf>,

        /// Dot path of parent.
        #[arg(long)]
        to: Option<String>,

        /// Sync manifest JSON.
        #[arg(long)]
        manifest: Option<std::path::PathBuf>,

        /// Watch files and resync on changes.
        #[arg(long)]
        watch: bool,

        /// Report changes without mutating Studio where supported.
        #[arg(long)]
        dry_run: bool,

        /// Delete previously synced script instances missing locally.
        #[arg(long)]
        delete: bool,

        /// Permit overwriting user-owned/manual script instances.
        #[arg(long)]
        force: bool,
    },

    /// Run multiple rs operations from a manifest.
    Batch {
        /// Batch manifest JSON.
        #[arg(long)]
        file: std::path::PathBuf,

        /// Print planned mutating work where supported.
        #[arg(long)]
        dry_run: bool,

        /// Continue executing after a failed step.
        #[arg(long)]
        continue_on_error: bool,
    },

    /// Create, inspect, or import portable rs packages.
    Package {
        /// Studio name, substring, or UUID. Optional if exactly one Studio is connected.
        #[arg(long)]
        studio: Option<String>,

        /// Dot path to package. Used when no package subcommand is supplied.
        #[arg(long)]
        path: Option<String>,

        /// Output package folder. Used when no package subcommand is supplied.
        #[arg(long)]
        out: Option<std::path::PathBuf>,

        /// Optional descendant depth for the human-readable export tree.
        #[arg(long)]
        depth: Option<u32>,

        /// Overwrite package files when they already exist.
        #[arg(long)]
        overwrite: bool,

        #[command(subcommand)]
        command: Option<PackageCommand>,
    },

    /// Snapshot or restore mutating Studio work.
    Transaction {
        #[command(subcommand)]
        command: TransactionCommand,
    },

    /// Show Studio-side rs command history.
    History {
        /// Studio name, substring, or UUID. Optional if exactly one Studio is connected.
        #[arg(long)]
        studio: Option<String>,

        #[command(subcommand)]
        command: Option<HistoryCommand>,

        /// Output format.
        #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
        format: OutputFormat,
    },

    /// Undo a previous rs command when its history entry has a rollback snapshot.
    Undo {
        /// Studio name, substring, or UUID. Optional if exactly one Studio is connected.
        #[arg(long)]
        studio: Option<String>,

        /// Command ID from rs history.
        id: String,

        /// Approve the undo mutation.
        #[arg(long)]
        yes: bool,

        /// Output format.
        #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
        format: OutputFormat,
    },

    /// Inspect asset, script, and remote dependencies under a Studio path.
    Deps {
        /// Studio name, substring, or UUID. Optional if exactly one Studio is connected.
        #[arg(long)]
        studio: Option<String>,

        /// Dot path such as Workspace.Tool.
        #[arg(long)]
        path: String,

        /// Write dependency JSON to a file.
        #[arg(long)]
        out: Option<std::path::PathBuf>,

        /// Output format.
        #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
        format: OutputFormat,
    },

    /// Preflight a Studio subtree or package before sharing/publishing.
    PublishCheck {
        /// Studio name, substring, or UUID. Optional if exactly one Studio is connected.
        #[arg(long)]
        studio: Option<String>,

        /// Dot path to check.
        #[arg(long)]
        path: String,

        /// Optional package folder whose checksums should be verified.
        #[arg(long = "package")]
        package_path: Option<std::path::PathBuf>,

        /// Output format.
        #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
        format: OutputFormat,
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

#[derive(Debug, Subcommand)]
enum AutopilotCommand {
    /// List built-in deterministic Autopilot recipes.
    Recipes(AutopilotRecipesArgs),

    /// Write an AI-readable atlas of verified Autopilot capabilities.
    Capabilities(AutopilotCapabilitiesArgs),

    /// Compose several recipes into one reviewable starter-game plan.
    Compose(AutopilotComposeArgs),

    /// Generate an explicit tuned compose manifest from creator intent.
    Tune(AutopilotTuneArgs),

    /// Recommend the next safe agent action for an Autopilot plan or run folder.
    Coach(AutopilotCoachArgs),

    /// Write an agent-ready JSON and Markdown handoff for an Autopilot run.
    Handoff(AutopilotHandoffArgs),

    /// List Autopilot run folders with status, blockers, and next commands.
    Runs(AutopilotRunsArgs),

    /// Write a project-level AI mission packet from prior runs and an optional prompt.
    Mission(AutopilotMissionArgs),

    /// Write a compact project memory ledger from prior Autopilot runs.
    Memory(AutopilotMemoryArgs),

    /// Learn durable creator preferences from prior runs, decisions, and feedback.
    Preferences(AutopilotPreferencesArgs),

    /// Write a cross-run game bible for project canon, style, systems, and continuity.
    GameBible(AutopilotGameBibleArgs),

    /// Write a cross-run AI operating playbook from memory, canon, and retrospectives.
    Playbook(AutopilotPlaybookArgs),

    /// Write a canon-aware creative director slate for the next ambitious build bets.
    Director(AutopilotDirectorArgs),

    /// Execute a selected creative director bet through safe offline handlers.
    Pursue(AutopilotPursueArgs),

    /// Write a durable AI work agenda from the current cockpit command queue.
    Agenda(AutopilotAgendaArgs),

    /// Run a bounded safe agenda sprint through internal offline handlers.
    Sprint(AutopilotSprintArgs),

    /// Summarize recent AI work into lessons, claims, and next commands.
    Retrospect(AutopilotRetrospectArgs),

    /// Write one AI mission-control packet from memory, next, roadmap, and evidence.
    Control(AutopilotControlArgs),

    /// Write a creator-safe AI status brief with allowed and forbidden claims.
    Brief(AutopilotBriefArgs),

    /// Route a creator message into the safest next Autopilot command.
    Inbox(AutopilotInboxArgs),

    /// Handle a creator message through the safe offline Autopilot path.
    Handle(AutopilotHandleArgs),

    /// Write durable creator/AI conversation state for a run.
    Conversation(AutopilotConversationArgs),

    /// Handle creator chat through safe offline Autopilot steps and prepare an honest reply.
    Chat(AutopilotChatArgs),

    /// Turn a creator request into assumptions, questions, acceptance criteria, and first commands.
    Intake(AutopilotIntakeArgs),

    /// Run the offline AI startup flow from intake through review-pack bootstrap.
    Start(AutopilotStartArgs),

    /// Offer ranked deterministic build directions before choosing one to drive.
    Pitch(AutopilotPitchArgs),

    /// Write a player-facing storyboard for a prompt or Autopilot run.
    Storyboard(AutopilotStoryboardArgs),

    /// Write a creator-facing proposal from pitch, storyboard, and safe next commands.
    Proposal(AutopilotProposalArgs),

    /// Write one AI companion packet from proposal and setup readiness.
    Companion(AutopilotCompanionArgs),

    /// Record which proposal candidate the creator chose before driving it.
    Select(AutopilotSelectArgs),

    /// Drive the selected proposal candidate through the safe offline boundary.
    Launch(AutopilotLaunchArgs),

    /// Safely bootstrap or resume an Autopilot run until the live mutation boundary.
    Drive(AutopilotDriveArgs),

    /// Write one AI cockpit dashboard from mission control, brief, memory, roadmap, and proof.
    Cockpit(AutopilotCockpitArgs),

    /// Write a copy/paste-safe AI continuation capsule from the current cockpit.
    Capsule(AutopilotCapsuleArgs),

    /// Write one cold-start orientation packet for a fresh AI session.
    Orient(AutopilotOrientArgs),

    /// Write one redacted model-ready context pack for AI resume.
    ModelPack(AutopilotModelPackArgs),

    /// Write one copy/paste-ready agent task packet from model context and work order.
    TaskPack(AutopilotTaskPackArgs),

    /// Write one AI best-friend launch packet from memory, context, and task evidence.
    BestFriend(AutopilotBestFriendArgs),

    /// Audit whether a fresh AI can safely start from the best-friend launch packet.
    BestFriendCheck(AutopilotBestFriendCheckArgs),

    /// Recover a blocked AI best-friend session with checked repair guidance.
    BestFriendRescue(AutopilotBestFriendRescueArgs),

    /// Write coaching guidance for the AI before it speaks, acts, or recovers.
    BestFriendMentor(AutopilotBestFriendMentorArgs),

    /// Coach, run one protected companion action, and prepare a checked reply.
    BestFriendPilot(AutopilotBestFriendPilotArgs),

    /// Decide whether the AI should pilot, rescue, publish, or speak next.
    BestFriendControl(AutopilotBestFriendControlArgs),

    /// Execute one best-friend control-selected offline branch and stop.
    BestFriendOperate(AutopilotBestFriendOperateArgs),

    /// Run bounded best-friend operator steps until a real boundary is reached.
    BestFriendRunner(AutopilotBestFriendRunnerArgs),

    /// Execute one protected best-friend first action and refresh the launch packet.
    FirstTurn(AutopilotFirstTurnArgs),

    /// Run bounded protected best-friend turns until stopped by evidence or safety.
    BestFriendLoop(AutopilotBestFriendLoopArgs),

    /// Draft and self-check the creator-facing reply after best-friend work.
    BestFriendReply(AutopilotBestFriendReplyArgs),

    /// Handle one safe AI best-friend operating turn from message to checked reply.
    BestFriendTurn(AutopilotBestFriendTurnArgs),

    /// Bootstrap or resume one AI best-friend session from first contact to checked reply.
    BestFriendSession(AutopilotBestFriendSessionArgs),

    /// Bootstrap or resume a run and prepare a proof-bound wow demo packet.
    WowSession(AutopilotWowSessionArgs),

    /// Run the whole best-friend creator arc from prompt to demo reaction receipt.
    BestFriendArc(AutopilotBestFriendArcArgs),

    /// Write a multi-agent assignment board from opportunities and task context.
    SquadPack(AutopilotSquadPackArgs),

    /// Review squad assignments, evidence, ownership conflicts, and integration next steps.
    SquadReview(AutopilotSquadReviewArgs),

    /// Write a safe creative wow-factor plan from run artifacts and creator intent.
    WowPlan(AutopilotWowPlanArgs),

    /// Write an executable agent packet for the selected wow-plan player moment.
    MomentPack(AutopilotMomentPackArgs),

    /// Execute the selected wow moment as a safe offline candidate sprint.
    MomentSprint(AutopilotMomentSprintArgs),

    /// Decide whether the reviewed wow candidate is the best offline continuation.
    MomentDecision(AutopilotMomentDecisionArgs),

    /// Write a proof-bound creator demo packet for the recommended wow run.
    CreatorDemo(AutopilotCreatorDemoArgs),

    /// Route a creator's post-demo response into the next safe artifact path.
    DemoResponse(AutopilotDemoResponseArgs),

    /// Package a post-demo response route into the next AI handoff loop.
    DemoLoop(AutopilotDemoLoopArgs),

    /// Route a post-demo response, check reply wording, and refresh remembered context.
    DemoSession(AutopilotDemoSessionArgs),

    /// Audit whether the post-demo follow-up is actually handled before replying.
    DemoCheck(AutopilotDemoCheckArgs),

    /// Compose an evidence-checked creator reply from demo-check state.
    DemoReply(AutopilotDemoReplyArgs),

    /// Distill reusable learning signals from the checked post-demo conversation.
    DemoLearn(AutopilotDemoLearnArgs),

    /// Consolidate post-demo learning into durable project memory and AI context.
    Remember(AutopilotRememberArgs),

    /// Write one creator/AI review packet from approval, proof, privacy, and evidence.
    ReviewPack(AutopilotReviewPackArgs),

    /// Publish an offline Autopilot review packet into the Studio review panel.
    PublishReview(AutopilotPublishReviewArgs),

    /// Write a Roblox publish-prep dossier from run proof, policy, and showcase artifacts.
    PublishPrep(AutopilotPublishPrepArgs),

    /// Turn creator review notes into an AI-ready patch triage packet.
    Feedback(AutopilotFeedbackArgs),

    /// Turn feedback triage into a strict AI patch work order.
    FeedbackPatch(AutopilotFeedbackPatchArgs),

    /// Check proposed creator-facing claims against run evidence.
    ClaimCheck(AutopilotClaimCheckArgs),

    /// Compose a creator-facing response only after claims pass evidence checks.
    Respond(AutopilotRespondArgs),

    /// Record creator decisions, constraints, rejections, and notes without live approval.
    Decision(AutopilotDecisionArgs),

    /// Check a plan against recorded creator decisions before the next AI continues.
    Align(AutopilotAlignArgs),

    /// Record AI work notes, attempted commands, and continuation evidence for a run.
    Journal(AutopilotJournalArgs),

    /// Write an artifact-backed proof ledger for an Autopilot run.
    Proof(AutopilotProofArgs),

    /// Write a creator-intent acceptance scorecard for an Autopilot run.
    Acceptance(AutopilotAcceptanceArgs),

    /// Write a creator-promise fulfillment checklist with concrete evidence and gaps.
    Fulfillment(AutopilotFulfillmentArgs),

    /// Audit whether the creator objective is actually complete from artifacts.
    CompletionAudit(AutopilotCompletionAuditArgs),

    /// Write the creator-facing delivery message from completion-audit evidence.
    Deliver(AutopilotDeliverArgs),

    /// Turn missing creator-promise recipe gaps into an offline patch run.
    Satisfy(AutopilotSatisfyArgs),

    /// Repeatedly create offline patch runs until creator-promise recipes are covered.
    PromiseLoop(AutopilotPromiseLoopArgs),

    /// Write a prompt-to-artifact traceability matrix for an Autopilot run.
    Trace(AutopilotTraceArgs),

    /// Refresh derived offline review artifacts for an existing Autopilot run.
    Refresh(AutopilotRefreshArgs),

    /// Write an artifact-backed rollback readiness packet for an applied run.
    Rollback(AutopilotRollbackArgs),

    /// Write a creator approval packet before live Autopilot apply.
    Approval(AutopilotApprovalArgs),

    /// Scan Autopilot run artifacts for unredacted secrets or private credentials.
    Privacy(AutopilotPrivacyArgs),

    /// Choose the next best Autopilot move for an AI agent.
    Next(AutopilotNextArgs),

    /// Rank evidence-backed build, repair, proof, and continuity opportunities for an AI agent.
    Opportunities(AutopilotOpportunitiesArgs),

    /// Turn one ranked opportunity into an exact AI execution work order.
    WorkOrder(AutopilotWorkOrderArgs),

    /// Check whether a work order's expected artifacts are actually present.
    WorkCheck(AutopilotWorkCheckArgs),

    /// Run one offline AI cycle from opportunity through safe response routing.
    Cycle(AutopilotCycleArgs),

    /// Diagnose why an offline AI cycle or command is stuck.
    Diagnose(AutopilotDiagnoseArgs),

    /// Validate an AI-proposed Autopilot command sequence before execution.
    CommandGuard(AutopilotCommandGuardArgs),

    /// Check proposed AI claims and commands before speaking or acting.
    SelfCheck(AutopilotSelfCheckArgs),

    /// Write a reviewed execution runbook from a guarded AI command queue.
    Runbook(AutopilotRunbookArgs),

    /// Record the run's command, gate, evidence, and claim history for AI resume.
    FlightRecorder(AutopilotFlightRecorderArgs),

    /// Write one concise AI operating card from orientation, runbook, and recorder.
    Navigator(AutopilotNavigatorArgs),

    /// Execute one navigator-selected safe offline act action and refresh navigation.
    Advance(AutopilotAdvanceArgs),

    /// Execute the next safe offline Autopilot action and refresh loop evidence.
    Act(AutopilotActArgs),

    /// Run a guarded offline cycle/action loop until report-ready or blocked.
    Loop(AutopilotLoopArgs),

    /// Write a multi-step Autopilot execution roadmap for AI agents.
    Roadmap(AutopilotRoadmapArgs),

    /// Judge whether an Autopilot run is demo-ready, live-ready, or blocked.
    Judge(AutopilotJudgeArgs),

    /// Critique a planned gameplay slice and recommend the next design-safe step.
    Critique(AutopilotCritiqueArgs),

    /// Write a recipe-aware live playtest checklist for an Autopilot run.
    Playtest(AutopilotPlaytestArgs),

    /// Simulate the static player journey implied by an Autopilot plan.
    Simulate(AutopilotSimulateArgs),

    /// Write a feature graph connecting recipes, scripts, remotes, UI, and gates.
    Graph(AutopilotGraphArgs),

    /// Analyze currencies, rewards, prices, and first-purchase pacing.
    Balance(AutopilotBalanceArgs),

    /// Map services, scripts, remotes, cloud surfaces, and mutation blast radius.
    Impact(AutopilotImpactArgs),

    /// Map RemoteEvent and RemoteFunction contracts across generated scripts.
    Contracts(AutopilotContractsArgs),

    /// Audit server authority and exploit-sensitive generated gameplay surfaces.
    Authority(AutopilotAuthorityArgs),

    /// Audit generated UI, player affordances, and feedback loops.
    Ux(AutopilotUxArgs),

    /// Extract generated player-facing copy for review and localization.
    CopyDeck(AutopilotCopyDeckArgs),

    /// Audit generated slice performance budget before live apply.
    Performance(AutopilotPerformanceArgs),

    /// Audit generated UI and input accessibility before live apply.
    Accessibility(AutopilotAccessibilityArgs),

    /// Audit generated Roblox policy and safety-sensitive surfaces.
    Policy(AutopilotPolicyArgs),

    /// Write an art, audio, UI, and VFX asset production brief.
    AssetBrief(AutopilotAssetBriefArgs),

    /// Write a durable style bible for generated Roblox game assets and UI.
    StyleGuide(AutopilotStyleGuideArgs),

    /// Write a spatial world blueprint with zones, routes, and interaction anchors.
    WorldBlueprint(AutopilotWorldBlueprintArgs),

    /// Write a first-session onboarding plan for the generated gameplay loop.
    Onboarding(AutopilotOnboardingArgs),

    /// Write screenshot, trailer, thumbnail, and demo proof guidance for the generated run.
    Showcase(AutopilotShowcaseArgs),

    /// Write analytics, funnel, and retention measurement guidance for the generated run.
    Telemetry(AutopilotTelemetryArgs),

    /// Write ethical monetization offers, commerce surfaces, and price-test guidance.
    Monetization(AutopilotMonetizationArgs),

    /// Write social loops, friend moments, and Roblox growth proof guidance.
    Social(AutopilotSocialArgs),

    /// Write live update cadence, event hooks, and operational proof guidance.
    Liveops(AutopilotLiveopsArgs),

    /// Write DataStore schema, save/load proof, and migration guidance.
    Persistence(AutopilotPersistenceArgs),

    /// Write a live-proof evidence collection kit for an Autopilot run.
    Evidence(AutopilotEvidenceArgs),

    /// Record live playtest evidence after an approved Autopilot apply.
    RecordPlaytest(AutopilotRecordPlaytestArgs),

    /// Review live playtest evidence and route the next AI repair or proof step.
    EvidenceReview(AutopilotEvidenceReviewArgs),

    /// Decide whether an applied Autopilot run is actually healthy.
    Health(AutopilotHealthArgs),

    /// Convert failed playtest evidence into an AI-ready repair plan.
    RepairPlan(AutopilotRepairPlanArgs),

    /// Create a deterministic patch run from gameplay critique gaps.
    Improve(AutopilotImproveArgs),

    /// Compare two Autopilot runs and choose the safer continuation.
    Compare(AutopilotCompareArgs),

    /// Repeatedly improve and compare a run until it is playable or blocked.
    Iterate(AutopilotIterateArgs),

    /// Write an ordered multi-run apply sequence for baseline and patch runs.
    Sequence(AutopilotSequenceArgs),

    /// Turn a creator prompt into a staged AI build architecture.
    Architect(AutopilotArchitectArgs),

    /// Create an offline architecture, composed plan, handoff, and certification packet.
    Kickoff(AutopilotKickoffArgs),

    /// Audit generated Luau sources for risky constructs before live apply.
    AuditSources(AutopilotAuditSourcesArgs),

    /// Write a provider-ready planning packet for an AI model.
    PlannerPack(AutopilotPlannerPackArgs),

    /// Adopt a strict AI-generated Autopilot plan into a certified run folder.
    AdoptPlan(AutopilotAdoptPlanArgs),

    /// Certify an Autopilot run with deterministic go/no-go gates.
    Certify(AutopilotCertifyArgs),

    /// Write a hashed handoff manifest for an Autopilot run folder.
    Bundle(AutopilotBundleArgs),

    /// Verify artifact hashes from an Autopilot bundle manifest.
    VerifyBundle(AutopilotVerifyBundleArgs),

    /// Write an AI-readable setup packet for bridge, Studio, and plugin readiness.
    Setup(AutopilotSetupArgs),

    /// Wait until a Studio session is ready for live Autopilot preview/apply.
    Ready(AutopilotReadyArgs),

    /// Produce a final go/no-go gate before any live Autopilot apply.
    LiveGate(AutopilotLiveGateArgs),

    /// Write an AI-safe live demo rehearsal runbook without mutating Studio.
    Rehearsal(AutopilotRehearsalArgs),

    /// Produce an honest done/not-done closeout verdict for a run.
    Closeout(AutopilotCloseoutArgs),

    /// Write a black-box timeline and resume command for an Autopilot run.
    Timeline(AutopilotTimelineArgs),

    /// Plan and apply a deterministic Autopilot recipe in one approved run.
    Run(AutopilotRunArgs),

    /// Explain risk, blockers, artifacts, and next commands for an Autopilot plan.
    Explain(AutopilotExplainArgs),

    /// Create a deterministic Autopilot plan and artifact folder.
    Plan(AutopilotPlanArgs),

    /// Capture a redacted AI-agent context bundle from Studio.
    Context(AutopilotContextArgs),

    /// Write an AI-readable place survey from live Studio or a saved context bundle.
    Survey(AutopilotSurveyArgs),

    /// Compare an Autopilot run with saved Studio survey/context evidence.
    Reconcile(AutopilotReconcileArgs),

    /// Combine a creator request and place survey into the next AI build move.
    Scout(AutopilotScoutArgs),

    /// Bootstrap a full offline AI work session from scout or survey evidence.
    Session(AutopilotSessionArgs),

    /// Preview an existing Autopilot plan without mutating Studio.
    Preview(AutopilotPreviewArgs),

    /// Apply an approved Autopilot plan.
    Apply(AutopilotApplyArgs),

    /// Print a prior Autopilot run report.
    Report(AutopilotReportArgs),
}

#[derive(Debug, ClapArgs)]
struct AutopilotRecipesArgs {
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotCapabilitiesArgs {
    /// Root folder containing Autopilot run directories.
    #[arg(long, default_value = ".rs\\autopilot\\runs")]
    root: std::path::PathBuf,

    /// Maximum number of run folders to reference in suggested commands.
    #[arg(long, default_value_t = 20)]
    limit: usize,

    /// JSON capability atlas path. Defaults to .rs/autopilot/capability-atlas.json.
    #[arg(long)]
    out: Option<std::path::PathBuf>,

    /// Markdown capability atlas path. Defaults to .rs/autopilot/capability-atlas.md.
    #[arg(long)]
    markdown: Option<std::path::PathBuf>,

    /// Skip writing the Markdown companion file.
    #[arg(long)]
    no_markdown: bool,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotComposeArgs {
    /// Natural-language request. Quote multi-word prompts.
    #[arg()]
    prompt: Vec<String>,

    /// Studio name, substring, or UUID. Stored in the plan for later apply.
    #[arg(long)]
    studio: Option<String>,

    /// Studio path used as the operation root.
    #[arg(long, default_value = "game")]
    scope: String,

    /// Artifact output folder.
    #[arg(long)]
    out: Option<std::path::PathBuf>,

    /// Built-in composition preset, such as fullStarterGame or monetizedPrototype.
    #[arg(long)]
    preset: Option<String>,

    /// Built-in deterministic recipes to include. May be repeated or comma-separated.
    #[arg(long, value_delimiter = ',')]
    recipe: Vec<String>,

    /// Structured composition manifest JSON.
    #[arg(long)]
    from_manifest: Option<std::path::PathBuf>,

    /// Optional smoke suite to append to the composed plan.
    #[arg(long)]
    smoke: Option<String>,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotTuneArgs {
    /// Natural-language request. Quote multi-word prompts.
    #[arg()]
    prompt: Vec<String>,

    /// Built-in composition preset, such as fullStarterGame or tycoonPrototype.
    #[arg(long)]
    preset: Option<String>,

    /// Built-in deterministic recipes to include. May be repeated or comma-separated.
    #[arg(long, value_delimiter = ',')]
    recipe: Vec<String>,

    /// Optional smoke suite to include in the tuned compose manifest.
    #[arg(long)]
    smoke: Option<String>,

    /// Manifest output path. Defaults under .rs/autopilot/manifests/.
    #[arg(long)]
    out: Option<std::path::PathBuf>,

    /// Markdown summary path. Defaults beside the manifest.
    #[arg(long)]
    markdown: Option<std::path::PathBuf>,

    /// Do not write a Markdown summary.
    #[arg(long)]
    no_markdown: bool,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotCoachArgs {
    /// Autopilot run directory containing plan.json.
    #[arg(long)]
    run_dir: Option<std::path::PathBuf>,

    /// Autopilot plan JSON. Defaults the run directory to the plan's parent folder.
    #[arg(long)]
    plan: Option<std::path::PathBuf>,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotHandoffArgs {
    /// Autopilot run directory containing plan.json.
    run_dir: std::path::PathBuf,

    /// JSON handoff path. Defaults to <run-dir>/handoff.json.
    #[arg(long)]
    out: Option<std::path::PathBuf>,

    /// Markdown handoff path. Defaults to <run-dir>/handoff.md.
    #[arg(long)]
    markdown: Option<std::path::PathBuf>,

    /// Skip writing the Markdown companion file.
    #[arg(long)]
    no_markdown: bool,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotRunsArgs {
    /// Root folder containing Autopilot run directories.
    #[arg(long, default_value = ".rs\\autopilot\\runs")]
    root: std::path::PathBuf,

    /// Maximum number of run folders to return.
    #[arg(long, default_value_t = 20)]
    limit: usize,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotMissionArgs {
    /// Optional creator request to map onto safe recipe/compose commands.
    #[arg()]
    prompt: Vec<String>,

    /// Root folder containing Autopilot run directories.
    #[arg(long, default_value = ".rs\\autopilot\\runs")]
    root: std::path::PathBuf,

    /// Maximum number of run folders to include.
    #[arg(long, default_value_t = 5)]
    limit: usize,

    /// JSON mission path. Defaults to .rs/autopilot/mission.json.
    #[arg(long)]
    out: Option<std::path::PathBuf>,

    /// Markdown mission path. Defaults to .rs/autopilot/mission.md.
    #[arg(long)]
    markdown: Option<std::path::PathBuf>,

    /// Skip writing the Markdown companion file.
    #[arg(long)]
    no_markdown: bool,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotMemoryArgs {
    /// Root folder containing Autopilot run directories.
    #[arg(long, default_value = ".rs\\autopilot\\runs")]
    root: std::path::PathBuf,

    /// Maximum number of run folders to include.
    #[arg(long, default_value_t = 20)]
    limit: usize,

    /// JSON memory path. Defaults to .rs/autopilot/project-memory.json.
    #[arg(long)]
    out: Option<std::path::PathBuf>,

    /// Markdown memory path. Defaults to .rs/autopilot/project-memory.md.
    #[arg(long)]
    markdown: Option<std::path::PathBuf>,

    /// Skip writing the Markdown companion file.
    #[arg(long)]
    no_markdown: bool,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotPreferencesArgs {
    /// Root folder containing Autopilot run directories.
    #[arg(long, default_value = ".rs\\autopilot\\runs")]
    root: std::path::PathBuf,

    /// Maximum number of run folders to learn from.
    #[arg(long, default_value_t = 20)]
    limit: usize,

    /// JSON preferences path. Defaults to .rs/autopilot/creator-preferences.json.
    #[arg(long)]
    out: Option<std::path::PathBuf>,

    /// Markdown preferences path. Defaults to .rs/autopilot/creator-preferences.md.
    #[arg(long)]
    markdown: Option<std::path::PathBuf>,

    /// Skip writing the Markdown companion file.
    #[arg(long)]
    no_markdown: bool,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotGameBibleArgs {
    /// Root folder containing Autopilot run directories.
    #[arg(long, default_value = ".rs\\autopilot\\runs")]
    root: std::path::PathBuf,

    /// Maximum number of run folders to learn from.
    #[arg(long, default_value_t = 20)]
    limit: usize,

    /// JSON game bible path. Defaults to .rs/autopilot/game-bible.json.
    #[arg(long)]
    out: Option<std::path::PathBuf>,

    /// Markdown game bible path. Defaults to .rs/autopilot/game-bible.md.
    #[arg(long)]
    markdown: Option<std::path::PathBuf>,

    /// Skip writing the Markdown companion file.
    #[arg(long)]
    no_markdown: bool,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotDirectorArgs {
    /// Optional creator request or strategic focus.
    #[arg()]
    prompt: Vec<String>,

    /// Root folder containing Autopilot run directories.
    #[arg(long, default_value = ".rs\\autopilot\\runs")]
    root: std::path::PathBuf,

    /// Maximum number of run folders to learn from.
    #[arg(long, default_value_t = 20)]
    limit: usize,

    /// JSON director path. Defaults to .rs/autopilot/director.json.
    #[arg(long)]
    out: Option<std::path::PathBuf>,

    /// Markdown director path. Defaults to .rs/autopilot/director.md.
    #[arg(long)]
    markdown: Option<std::path::PathBuf>,

    /// Skip writing the Markdown companion file.
    #[arg(long)]
    no_markdown: bool,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotPlaybookArgs {
    /// Root folder containing Autopilot run directories.
    #[arg(long, default_value = ".rs\\autopilot\\runs")]
    root: std::path::PathBuf,

    /// Maximum number of run folders to learn from.
    #[arg(long, default_value_t = 20)]
    limit: usize,

    /// JSON playbook path. Defaults to .rs/autopilot/ai-playbook.json.
    #[arg(long)]
    out: Option<std::path::PathBuf>,

    /// Markdown playbook path. Defaults to .rs/autopilot/ai-playbook.md.
    #[arg(long)]
    markdown: Option<std::path::PathBuf>,

    /// Skip writing the Markdown companion file.
    #[arg(long)]
    no_markdown: bool,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotPursueArgs {
    /// Optional creator request or strategic focus used when refreshing director context.
    #[arg()]
    prompt: Vec<String>,

    /// Director bet id to execute. Defaults to the first supported safe offline bet.
    #[arg(long)]
    bet: Option<String>,

    /// Root folder containing Autopilot run directories.
    #[arg(long, default_value = ".rs\\autopilot\\runs")]
    root: std::path::PathBuf,

    /// Maximum number of run folders to learn from.
    #[arg(long, default_value_t = 20)]
    limit: usize,

    /// JSON pursuit path. Defaults to .rs/autopilot/pursuit.json.
    #[arg(long)]
    out: Option<std::path::PathBuf>,

    /// Markdown pursuit path. Defaults to .rs/autopilot/pursuit.md.
    #[arg(long)]
    markdown: Option<std::path::PathBuf>,

    /// Skip writing the Markdown companion file.
    #[arg(long)]
    no_markdown: bool,

    /// Select and report the bet without executing it.
    #[arg(long)]
    dry_run: bool,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotAgendaArgs {
    /// Optional creator request or focus for the AI agenda.
    #[arg()]
    prompt: Vec<String>,

    /// Inspect this run directly instead of selecting from --root.
    #[arg(long)]
    run_dir: Option<std::path::PathBuf>,

    /// Root folder containing Autopilot run directories.
    #[arg(long, default_value = ".rs\\autopilot\\runs")]
    root: std::path::PathBuf,

    /// Maximum number of run folders to learn from.
    #[arg(long, default_value_t = 20)]
    limit: usize,

    /// JSON agenda path. Defaults to .rs/autopilot/agenda.json.
    #[arg(long)]
    out: Option<std::path::PathBuf>,

    /// Markdown agenda path. Defaults to .rs/autopilot/agenda.md.
    #[arg(long)]
    markdown: Option<std::path::PathBuf>,

    /// Skip writing the Markdown companion file.
    #[arg(long)]
    no_markdown: bool,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotSprintArgs {
    /// Optional creator request or focus for the agenda sprint.
    #[arg()]
    prompt: Vec<String>,

    /// Source run directory. If omitted, sprint uses the agenda-selected run when available.
    #[arg(long)]
    run_dir: Option<std::path::PathBuf>,

    /// Root folder containing Autopilot run directories.
    #[arg(long, default_value = ".rs\\autopilot\\runs")]
    root: std::path::PathBuf,

    /// Maximum number of run folders to learn from.
    #[arg(long, default_value_t = 20)]
    limit: usize,

    /// Maximum number of safe agenda actions to execute.
    #[arg(long, default_value_t = 3)]
    max_steps: usize,

    /// Select actions and write the sprint report without executing them.
    #[arg(long)]
    dry_run: bool,

    /// JSON sprint path. Defaults to <run-dir>/sprint.json or .rs/autopilot/sprint.json.
    #[arg(long)]
    out: Option<std::path::PathBuf>,

    /// Markdown sprint path. Defaults next to the JSON path.
    #[arg(long)]
    markdown: Option<std::path::PathBuf>,

    /// Skip writing the Markdown companion file.
    #[arg(long)]
    no_markdown: bool,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotRetrospectArgs {
    /// Specific Autopilot run directory to summarize.
    run_dir: std::path::PathBuf,

    /// Root folder containing Autopilot run directories.
    #[arg(long, default_value = ".rs\\autopilot\\runs")]
    root: std::path::PathBuf,

    /// Maximum number of run folders to inspect for suggested next context.
    #[arg(long, default_value_t = 20)]
    limit: usize,

    /// JSON retrospective path. Defaults to <run-dir>/retrospective.json.
    #[arg(long)]
    out: Option<std::path::PathBuf>,

    /// Markdown retrospective path. Defaults next to the JSON path.
    #[arg(long)]
    markdown: Option<std::path::PathBuf>,

    /// Skip writing the Markdown companion file.
    #[arg(long)]
    no_markdown: bool,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotControlArgs {
    /// Optional creator request to include in recommendations.
    #[arg()]
    prompt: Vec<String>,

    /// Inspect this run directly instead of selecting from --root.
    #[arg(long)]
    run_dir: Option<std::path::PathBuf>,

    /// Root folder containing Autopilot run directories.
    #[arg(long, default_value = ".rs\\autopilot\\runs")]
    root: std::path::PathBuf,

    /// Maximum number of run folders to inspect.
    #[arg(long, default_value_t = 20)]
    limit: usize,

    /// JSON control packet path. Defaults to .rs/autopilot/control.json.
    #[arg(long)]
    out: Option<std::path::PathBuf>,

    /// Markdown control packet path. Defaults next to the JSON path.
    #[arg(long)]
    markdown: Option<std::path::PathBuf>,

    /// Skip writing the Markdown companion file.
    #[arg(long)]
    no_markdown: bool,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotBriefArgs {
    /// Optional creator request to include in the status brief.
    #[arg()]
    prompt: Vec<String>,

    /// Inspect this run directly instead of selecting from --root.
    #[arg(long)]
    run_dir: Option<std::path::PathBuf>,

    /// Root folder containing Autopilot run directories.
    #[arg(long, default_value = ".rs\\autopilot\\runs")]
    root: std::path::PathBuf,

    /// Maximum number of run folders to inspect.
    #[arg(long, default_value_t = 20)]
    limit: usize,

    /// JSON brief path. Defaults to <run-dir>/user-brief.json or .rs/autopilot/user-brief.json.
    #[arg(long)]
    out: Option<std::path::PathBuf>,

    /// Markdown brief path. Defaults next to the JSON path.
    #[arg(long)]
    markdown: Option<std::path::PathBuf>,

    /// Skip writing the Markdown companion file.
    #[arg(long)]
    no_markdown: bool,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotInboxArgs {
    /// Creator message to classify and route.
    #[arg()]
    message: Vec<String>,

    /// Existing run folder to use as context.
    #[arg(long)]
    run_dir: Option<std::path::PathBuf>,

    /// Root folder containing Autopilot run directories.
    #[arg(long, default_value = ".rs\\autopilot\\runs")]
    root: std::path::PathBuf,

    /// Maximum number of run folders to inspect when --run-dir is omitted.
    #[arg(long, default_value_t = 20)]
    limit: usize,

    /// Message source label.
    #[arg(long, default_value = "creator")]
    source: String,

    /// JSON inbox path. Defaults to <run-dir>/inbox.json or .rs/autopilot/inbox.json.
    #[arg(long)]
    out: Option<std::path::PathBuf>,

    /// Markdown inbox path. Defaults next to the JSON path.
    #[arg(long)]
    markdown: Option<std::path::PathBuf>,

    /// Skip writing the Markdown companion file.
    #[arg(long)]
    no_markdown: bool,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotHandleArgs {
    /// Creator message to route and safely handle.
    #[arg()]
    message: Vec<String>,

    /// Existing run folder to use as context.
    #[arg(long)]
    run_dir: Option<std::path::PathBuf>,

    /// Studio name, substring, or UUID to preserve in generated start requests.
    #[arg(long)]
    studio: Option<String>,

    /// Scope root for generated validation and apply operations when starting a new run.
    #[arg(long, default_value = "Workspace")]
    scope: String,

    /// Root folder containing Autopilot run directories.
    #[arg(long, default_value = ".rs\\autopilot\\runs")]
    root: std::path::PathBuf,

    /// Maximum number of run folders to inspect when --run-dir is omitted.
    #[arg(long, default_value_t = 20)]
    limit: usize,

    /// Message source label.
    #[arg(long, default_value = "creator")]
    source: String,

    /// Continue with explicit assumptions when a new-build intake wants clarification.
    #[arg(long)]
    assume: bool,

    /// Smoke suite to append when a new build request starts an offline run.
    #[arg(long)]
    smoke: Option<String>,

    /// Route the message but do not execute the selected offline route.
    #[arg(long)]
    dry_run: bool,

    /// JSON handle path. Defaults to <run-dir>/handle.json or .rs/autopilot/handle.json.
    #[arg(long)]
    out: Option<std::path::PathBuf>,

    /// Markdown handle path. Defaults next to the JSON path.
    #[arg(long)]
    markdown: Option<std::path::PathBuf>,

    /// Skip writing Markdown companion files.
    #[arg(long)]
    no_markdown: bool,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotConversationArgs {
    /// Optional creator message to include as the newest unhandled turn.
    #[arg()]
    message: Vec<String>,

    /// Existing run folder to use as context.
    #[arg(long)]
    run_dir: Option<std::path::PathBuf>,

    /// Root folder containing Autopilot run directories.
    #[arg(long, default_value = ".rs\\autopilot\\runs")]
    root: std::path::PathBuf,

    /// Maximum number of run folders to inspect when --run-dir is omitted.
    #[arg(long, default_value_t = 20)]
    limit: usize,

    /// Message source label.
    #[arg(long, default_value = "creator")]
    source: String,

    /// JSON conversation path. Defaults to <run-dir>/conversation.json or .rs/autopilot/conversation.json.
    #[arg(long)]
    out: Option<std::path::PathBuf>,

    /// Markdown conversation path. Defaults next to the JSON path.
    #[arg(long)]
    markdown: Option<std::path::PathBuf>,

    /// Skip writing the Markdown companion file.
    #[arg(long)]
    no_markdown: bool,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotChatArgs {
    /// Creator message to route, safely handle, and prepare a reply for.
    #[arg()]
    message: Vec<String>,

    /// Existing run folder to use as context.
    #[arg(long)]
    run_dir: Option<std::path::PathBuf>,

    /// Studio name, substring, or UUID to preserve in generated start requests.
    #[arg(long)]
    studio: Option<String>,

    /// Scope root for generated validation and apply operations when starting a new run.
    #[arg(long, default_value = "Workspace")]
    scope: String,

    /// Root folder containing Autopilot run directories.
    #[arg(long, default_value = ".rs\\autopilot\\runs")]
    root: std::path::PathBuf,

    /// Maximum number of run folders to inspect when --run-dir is omitted.
    #[arg(long, default_value_t = 20)]
    limit: usize,

    /// Message source label.
    #[arg(long, default_value = "creator")]
    source: String,

    /// Continue with explicit assumptions when a new-build intake wants clarification.
    #[arg(long)]
    assume: bool,

    /// Smoke suite to append when a new build request starts an offline run.
    #[arg(long)]
    smoke: Option<String>,

    /// Maximum safe offline loop steps to run after handling the message. Use 0 to skip.
    #[arg(long, default_value_t = 2)]
    max_steps: usize,

    /// Route and summarize the message without executing safe offline follow-up steps.
    #[arg(long)]
    dry_run: bool,

    /// JSON chat path. Defaults to <run-dir>/chat.json or .rs/autopilot/chat.json.
    #[arg(long)]
    out: Option<std::path::PathBuf>,

    /// Markdown chat path. Defaults next to the JSON path.
    #[arg(long)]
    markdown: Option<std::path::PathBuf>,

    /// Skip writing Markdown companion files.
    #[arg(long)]
    no_markdown: bool,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotIntakeArgs {
    /// Creator request to interpret before architecture or planning.
    #[arg()]
    prompt: Vec<String>,

    /// Root folder containing Autopilot run directories.
    #[arg(long, default_value = ".rs\\autopilot\\runs")]
    root: std::path::PathBuf,

    /// Maximum number of run folders to inspect for continuity warnings.
    #[arg(long, default_value_t = 20)]
    limit: usize,

    /// JSON intake path. Defaults to .rs/autopilot/intake.json.
    #[arg(long)]
    out: Option<std::path::PathBuf>,

    /// Markdown intake path. Defaults next to the JSON path.
    #[arg(long)]
    markdown: Option<std::path::PathBuf>,

    /// Skip writing the Markdown companion file.
    #[arg(long)]
    no_markdown: bool,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotStartArgs {
    /// Creator request to turn into an offline AI-safe run packet.
    #[arg()]
    prompt: Vec<String>,

    /// Studio name, substring, or UUID to preserve in generated plan requests.
    #[arg(long)]
    studio: Option<String>,

    /// Scope root for generated validation and apply operations.
    #[arg(long, default_value = "Workspace")]
    scope: String,

    /// Root folder containing Autopilot run directories.
    #[arg(long, default_value = ".rs\\autopilot\\runs")]
    root: std::path::PathBuf,

    /// Specific run folder to create when kickoff proceeds.
    #[arg(long)]
    run_dir: Option<std::path::PathBuf>,

    /// Maximum number of run folders to inspect for continuity warnings.
    #[arg(long, default_value_t = 20)]
    limit: usize,

    /// Continue with explicit assumptions even when intake wants clarification.
    #[arg(long)]
    assume: bool,

    /// Smoke suite to include in generated plan/certification artifacts.
    #[arg(long)]
    smoke: Option<String>,

    /// JSON start path. Defaults to .rs/autopilot/start.json.
    #[arg(long)]
    out: Option<std::path::PathBuf>,

    /// Markdown start path. Defaults next to the JSON path.
    #[arg(long)]
    markdown: Option<std::path::PathBuf>,

    /// Skip writing the Markdown companion file.
    #[arg(long)]
    no_markdown: bool,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotPitchArgs {
    /// Creator request to turn into ranked build directions.
    #[arg()]
    prompt: Vec<String>,

    /// Studio name, substring, or UUID to preserve in generated drive commands.
    #[arg(long)]
    studio: Option<String>,

    /// Scope root for generated validation and apply operations.
    #[arg(long, default_value = "Workspace")]
    scope: String,

    /// Root folder where selected pitch runs should be driven.
    #[arg(long, default_value = ".rs\\autopilot\\runs")]
    root: std::path::PathBuf,

    /// Maximum number of candidates to return.
    #[arg(long, default_value_t = 3)]
    max_candidates: usize,

    /// JSON pitch path. Defaults to .rs/autopilot/pitch.json.
    #[arg(long)]
    out: Option<std::path::PathBuf>,

    /// Markdown pitch path. Defaults next to the JSON path.
    #[arg(long)]
    markdown: Option<std::path::PathBuf>,

    /// Skip writing the Markdown companion file.
    #[arg(long)]
    no_markdown: bool,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotStoryboardArgs {
    /// Creator request to turn into a player-facing storyboard.
    #[arg()]
    prompt: Vec<String>,

    /// Existing Autopilot run directory to storyboard.
    #[arg(long)]
    run_dir: Option<std::path::PathBuf>,

    /// Root folder where a prompt-only storyboard's drive command should point.
    #[arg(long, default_value = ".rs\\autopilot\\runs")]
    root: std::path::PathBuf,

    /// JSON storyboard path. Defaults to <run-dir>/storyboard.json or .rs/autopilot/storyboard.json.
    #[arg(long)]
    out: Option<std::path::PathBuf>,

    /// Markdown storyboard path. Defaults next to the JSON path.
    #[arg(long)]
    markdown: Option<std::path::PathBuf>,

    /// Skip writing the Markdown companion file.
    #[arg(long)]
    no_markdown: bool,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotProposalArgs {
    /// Creator request to turn into a proposal packet.
    #[arg()]
    prompt: Vec<String>,

    /// Studio name, substring, or UUID to preserve in generated drive commands.
    #[arg(long)]
    studio: Option<String>,

    /// Scope root for generated validation and apply operations.
    #[arg(long, default_value = "Workspace")]
    scope: String,

    /// Root folder where selected proposal runs should be driven.
    #[arg(long, default_value = ".rs\\autopilot\\runs")]
    root: std::path::PathBuf,

    /// Maximum number of pitch candidates to include.
    #[arg(long, default_value_t = 3)]
    max_candidates: usize,

    /// JSON proposal path. Defaults to .rs/autopilot/proposal.json.
    #[arg(long)]
    out: Option<std::path::PathBuf>,

    /// Markdown proposal path. Defaults next to the JSON path.
    #[arg(long)]
    markdown: Option<std::path::PathBuf>,

    /// Skip writing the Markdown companion file.
    #[arg(long)]
    no_markdown: bool,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotCompanionArgs {
    /// Creator request to turn into a proposal plus setup packet.
    #[arg()]
    prompt: Vec<String>,

    /// Studio name, substring, or UUID to preserve in generated commands and readiness checks.
    #[arg(long)]
    studio: Option<String>,

    /// Scope root for generated validation and apply operations.
    #[arg(long, default_value = "Workspace")]
    scope: String,

    /// Root folder where selected proposal runs should be driven.
    #[arg(long, default_value = ".rs\\autopilot\\runs")]
    root: std::path::PathBuf,

    /// Maximum number of pitch candidates to include.
    #[arg(long, default_value_t = 3)]
    max_candidates: usize,

    /// Build/copy the current plugin bundle before setup readiness.
    #[arg(long)]
    fix: bool,

    /// Seconds to wait before returning a structured setup blocker report.
    #[arg(long, default_value_t = 10)]
    timeout: u64,

    /// Poll interval in milliseconds.
    #[arg(long, default_value_t = 1000)]
    poll_ms: u64,

    /// JSON companion path. Defaults to .rs/autopilot/companion.json.
    #[arg(long)]
    out: Option<std::path::PathBuf>,

    /// Markdown companion path. Defaults next to the JSON path.
    #[arg(long)]
    markdown: Option<std::path::PathBuf>,

    /// Skip writing the Markdown companion file.
    #[arg(long)]
    no_markdown: bool,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotSelectArgs {
    /// Proposal JSON to select from.
    #[arg()]
    proposal: std::path::PathBuf,

    /// Candidate id to select. Defaults to proposal.recommendedCandidate.
    #[arg(long)]
    candidate: Option<String>,

    /// JSON selection path. Defaults to <proposal-dir>/selection.json.
    #[arg(long)]
    out: Option<std::path::PathBuf>,

    /// Markdown selection path. Defaults next to the JSON path.
    #[arg(long)]
    markdown: Option<std::path::PathBuf>,

    /// Skip writing the Markdown companion file.
    #[arg(long)]
    no_markdown: bool,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotLaunchArgs {
    /// Selection JSON to launch from.
    #[arg()]
    selection: std::path::PathBuf,

    /// Maximum number of run folders to inspect for continuity warnings.
    #[arg(long, default_value_t = 20)]
    limit: usize,

    /// Continue with explicit assumptions when drive bootstraps intake/start.
    #[arg(long)]
    assume: bool,

    /// Optional smoke suite to thread into generated apply guidance.
    #[arg(long)]
    smoke: Option<String>,

    /// JSON launch path. Defaults to <selection-dir>/launch.json.
    #[arg(long)]
    out: Option<std::path::PathBuf>,

    /// Markdown launch path. Defaults next to the JSON path.
    #[arg(long)]
    markdown: Option<std::path::PathBuf>,

    /// Skip writing the Markdown companion file.
    #[arg(long)]
    no_markdown: bool,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotDriveArgs {
    /// Creator request to bootstrap when --run-dir has no plan yet.
    #[arg()]
    prompt: Vec<String>,

    /// Studio name, substring, or UUID to preserve in generated plan requests.
    #[arg(long)]
    studio: Option<String>,

    /// Scope root for generated validation and apply operations.
    #[arg(long, default_value = "Workspace")]
    scope: String,

    /// Root folder containing Autopilot run directories.
    #[arg(long, default_value = ".rs\\autopilot\\runs")]
    root: std::path::PathBuf,

    /// Existing or planned run folder to drive.
    #[arg(long)]
    run_dir: Option<std::path::PathBuf>,

    /// Maximum number of run folders to inspect for continuity warnings.
    #[arg(long, default_value_t = 20)]
    limit: usize,

    /// Continue with explicit assumptions even when intake wants clarification.
    #[arg(long)]
    assume: bool,

    /// Smoke suite to include in generated plan/certification artifacts.
    #[arg(long)]
    smoke: Option<String>,

    /// JSON drive path. Defaults to <run-dir>/drive.json or .rs/autopilot/drive.json.
    #[arg(long)]
    out: Option<std::path::PathBuf>,

    /// Markdown drive path. Defaults next to the JSON path.
    #[arg(long)]
    markdown: Option<std::path::PathBuf>,

    /// Skip writing the Markdown companion file.
    #[arg(long)]
    no_markdown: bool,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotCockpitArgs {
    /// Optional creator request to include in the session dashboard.
    #[arg()]
    prompt: Vec<String>,

    /// Inspect this run directly instead of selecting from --root.
    #[arg(long)]
    run_dir: Option<std::path::PathBuf>,

    /// Root folder containing Autopilot run directories.
    #[arg(long, default_value = ".rs\\autopilot\\runs")]
    root: std::path::PathBuf,

    /// Maximum number of run folders to inspect.
    #[arg(long, default_value_t = 20)]
    limit: usize,

    /// JSON cockpit path. Defaults to <run-dir>/cockpit.json or .rs/autopilot/cockpit.json.
    #[arg(long)]
    out: Option<std::path::PathBuf>,

    /// Markdown cockpit path. Defaults next to the JSON path.
    #[arg(long)]
    markdown: Option<std::path::PathBuf>,

    /// Skip writing the Markdown companion file.
    #[arg(long)]
    no_markdown: bool,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotCapsuleArgs {
    /// Optional creator request to include in the continuation capsule.
    #[arg()]
    prompt: Vec<String>,

    /// Inspect this run directly instead of selecting from --root.
    #[arg(long)]
    run_dir: Option<std::path::PathBuf>,

    /// Root folder containing Autopilot run directories.
    #[arg(long, default_value = ".rs\\autopilot\\runs")]
    root: std::path::PathBuf,

    /// Maximum number of run folders to inspect.
    #[arg(long, default_value_t = 20)]
    limit: usize,

    /// JSON capsule path. Defaults to <run-dir>/agent-capsule.json or .rs/autopilot/agent-capsule.json.
    #[arg(long)]
    out: Option<std::path::PathBuf>,

    /// Markdown capsule path. Defaults next to the JSON path.
    #[arg(long)]
    markdown: Option<std::path::PathBuf>,

    /// Skip writing the Markdown companion file.
    #[arg(long)]
    no_markdown: bool,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotOrientArgs {
    /// Optional creator request to include in the orientation packet.
    #[arg()]
    prompt: Vec<String>,

    /// Inspect this run directly instead of selecting from --root.
    #[arg(long)]
    run_dir: Option<std::path::PathBuf>,

    /// Root folder containing Autopilot run directories.
    #[arg(long, default_value = ".rs\\autopilot\\runs")]
    root: std::path::PathBuf,

    /// Maximum number of run folders to inspect.
    #[arg(long, default_value_t = 20)]
    limit: usize,

    /// JSON orientation path. Defaults to <run-dir>/orientation.json or .rs/autopilot/orientation.json.
    #[arg(long)]
    out: Option<std::path::PathBuf>,

    /// Markdown orientation path. Defaults next to the JSON path.
    #[arg(long)]
    markdown: Option<std::path::PathBuf>,

    /// Skip writing the Markdown companion file.
    #[arg(long)]
    no_markdown: bool,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotModelPackArgs {
    /// Autopilot run directory to package for model resume.
    run_dir: std::path::PathBuf,

    /// Optional creator request to include in the resume prompt.
    #[arg()]
    prompt: Vec<String>,

    /// Root folder containing Autopilot run directories.
    #[arg(long, default_value = ".rs\\autopilot\\runs")]
    root: std::path::PathBuf,

    /// Maximum number of run folders to inspect for project-level packets.
    #[arg(long, default_value_t = 20)]
    limit: usize,

    /// Maximum total snippet characters to embed in the model pack.
    #[arg(long, default_value_t = 24000)]
    max_chars: usize,

    /// JSON model-pack path. Defaults to <run-dir>/model-pack.json.
    #[arg(long)]
    out: Option<std::path::PathBuf>,

    /// Markdown model-pack path. Defaults to <run-dir>/model-pack.md.
    #[arg(long)]
    markdown: Option<std::path::PathBuf>,

    /// Skip writing the Markdown companion file.
    #[arg(long)]
    no_markdown: bool,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotTaskPackArgs {
    /// Autopilot run directory to package into an agent task.
    run_dir: std::path::PathBuf,

    /// Optional creator request to include in the task prompt.
    #[arg()]
    prompt: Vec<String>,

    /// Root folder containing Autopilot run directories.
    #[arg(long, default_value = ".rs\\autopilot\\runs")]
    root: std::path::PathBuf,

    /// Maximum number of run folders to inspect for project-level packets.
    #[arg(long, default_value_t = 20)]
    limit: usize,

    /// Opportunity id or title to turn into the task. Defaults to the top ranked opportunity.
    #[arg(long)]
    opportunity: Option<String>,

    /// Maximum total snippet characters to embed in the nested model pack.
    #[arg(long, default_value_t = 24000)]
    max_chars: usize,

    /// JSON task-pack path. Defaults to <run-dir>/task-pack.json.
    #[arg(long)]
    out: Option<std::path::PathBuf>,

    /// Markdown task-pack path. Defaults to <run-dir>/task-pack.md.
    #[arg(long)]
    markdown: Option<std::path::PathBuf>,

    /// Skip writing the Markdown companion file.
    #[arg(long)]
    no_markdown: bool,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotBestFriendArgs {
    /// Autopilot run directory to package into an AI best-friend launch surface.
    run_dir: std::path::PathBuf,

    /// Optional creator request to include in the opening prompt.
    #[arg()]
    prompt: Vec<String>,

    /// Root folder containing Autopilot run directories.
    #[arg(long, default_value = ".rs\\autopilot\\runs")]
    root: std::path::PathBuf,

    /// Maximum number of run folders to inspect for project-level packets.
    #[arg(long, default_value_t = 20)]
    limit: usize,

    /// Opportunity id or title to turn into the first safe task. Defaults to the top ranked opportunity.
    #[arg(long)]
    opportunity: Option<String>,

    /// Maximum total snippet characters to embed in the nested model pack.
    #[arg(long, default_value_t = 24000)]
    max_chars: usize,

    /// JSON best-friend path. Defaults to <run-dir>/best-friend.json.
    #[arg(long)]
    out: Option<std::path::PathBuf>,

    /// Markdown best-friend path. Defaults to <run-dir>/best-friend.md.
    #[arg(long)]
    markdown: Option<std::path::PathBuf>,

    /// Skip writing the Markdown companion file.
    #[arg(long)]
    no_markdown: bool,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotBestFriendCheckArgs {
    /// Autopilot run directory to audit before a fresh AI acts.
    run_dir: std::path::PathBuf,

    /// Optional creator request to include in refreshed launch context.
    #[arg()]
    prompt: Vec<String>,

    /// Root folder containing Autopilot run directories.
    #[arg(long, default_value = ".rs\\autopilot\\runs")]
    root: std::path::PathBuf,

    /// Maximum number of run folders to inspect for project-level packets.
    #[arg(long, default_value_t = 20)]
    limit: usize,

    /// Opportunity id or title to turn into the first safe task. Defaults to the top ranked opportunity.
    #[arg(long)]
    opportunity: Option<String>,

    /// Maximum total snippet characters to embed in nested context packs.
    #[arg(long, default_value_t = 24000)]
    max_chars: usize,

    /// JSON best-friend-check path. Defaults to <run-dir>/best-friend-check.json.
    #[arg(long)]
    out: Option<std::path::PathBuf>,

    /// Markdown best-friend-check path. Defaults to <run-dir>/best-friend-check.md.
    #[arg(long)]
    markdown: Option<std::path::PathBuf>,

    /// Skip writing Markdown companion files.
    #[arg(long)]
    no_markdown: bool,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotBestFriendRescueArgs {
    /// Autopilot run directory to recover.
    run_dir: std::path::PathBuf,

    /// Optional creator request or recovery context to include in refreshed launch context.
    #[arg()]
    prompt: Vec<String>,

    /// Root folder containing Autopilot run directories.
    #[arg(long, default_value = ".rs\\autopilot\\runs")]
    root: std::path::PathBuf,

    /// Maximum number of run folders to inspect for project-level packets.
    #[arg(long, default_value_t = 20)]
    limit: usize,

    /// Opportunity id or title to turn into the first safe task. Defaults to the top ranked opportunity.
    #[arg(long)]
    opportunity: Option<String>,

    /// Maximum total snippet characters to embed in nested context packs.
    #[arg(long, default_value_t = 24000)]
    max_chars: usize,

    /// Failed or suspicious command to diagnose.
    #[arg(long)]
    command: Option<String>,

    /// Command result or short outcome to diagnose.
    #[arg(long)]
    result: Option<String>,

    /// Error text to include in the diagnosis. May be supplied multiple times.
    #[arg(long)]
    error: Vec<String>,

    /// Evidence path or note to include in the diagnosis. May be supplied multiple times.
    #[arg(long)]
    evidence: Vec<String>,

    /// JSON best-friend-rescue path. Defaults to <run-dir>/best-friend-rescue.json.
    #[arg(long)]
    out: Option<std::path::PathBuf>,

    /// Markdown best-friend-rescue path. Defaults to <run-dir>/best-friend-rescue.md.
    #[arg(long)]
    markdown: Option<std::path::PathBuf>,

    /// Skip writing Markdown companion files.
    #[arg(long)]
    no_markdown: bool,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotBestFriendMentorArgs {
    /// Autopilot run directory to coach before a fresh AI speaks, acts, or recovers.
    run_dir: std::path::PathBuf,

    /// Optional creator request or current concern to include in the coaching prompt.
    #[arg()]
    prompt: Vec<String>,

    /// Root folder containing Autopilot run directories.
    #[arg(long, default_value = ".rs\\autopilot\\runs")]
    root: std::path::PathBuf,

    /// Maximum number of run folders to inspect for project-level packets.
    #[arg(long, default_value_t = 20)]
    limit: usize,

    /// Opportunity id or title to turn into the first safe task. Defaults to the top ranked opportunity.
    #[arg(long)]
    opportunity: Option<String>,

    /// Maximum total snippet characters to embed in nested context packs.
    #[arg(long, default_value_t = 24000)]
    max_chars: usize,

    /// JSON best-friend-mentor path. Defaults to <run-dir>/best-friend-mentor.json.
    #[arg(long)]
    out: Option<std::path::PathBuf>,

    /// Markdown best-friend-mentor path. Defaults to <run-dir>/best-friend-mentor.md.
    #[arg(long)]
    markdown: Option<std::path::PathBuf>,

    /// Skip writing Markdown companion files.
    #[arg(long)]
    no_markdown: bool,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotBestFriendPilotArgs {
    /// Autopilot run directory to coach and advance by one protected companion move.
    run_dir: std::path::PathBuf,

    /// Optional creator request or current concern to include in refreshed context.
    #[arg()]
    prompt: Vec<String>,

    /// Root folder containing Autopilot run directories.
    #[arg(long, default_value = ".rs\\autopilot\\runs")]
    root: std::path::PathBuf,

    /// Maximum number of run folders to inspect for project-level packets.
    #[arg(long, default_value_t = 20)]
    limit: usize,

    /// Opportunity id or title to turn into the protected first task. Defaults to the top ranked opportunity.
    #[arg(long)]
    opportunity: Option<String>,

    /// Maximum total snippet characters to embed in nested context packs.
    #[arg(long, default_value_t = 24000)]
    max_chars: usize,

    /// Select the protected move and checked reply path without executing the action.
    #[arg(long)]
    dry_run: bool,

    /// JSON best-friend-pilot path. Defaults to <run-dir>/best-friend-pilot.json.
    #[arg(long)]
    out: Option<std::path::PathBuf>,

    /// Markdown best-friend-pilot path. Defaults to <run-dir>/best-friend-pilot.md.
    #[arg(long)]
    markdown: Option<std::path::PathBuf>,

    /// Skip writing Markdown companion files.
    #[arg(long)]
    no_markdown: bool,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotBestFriendControlArgs {
    /// Autopilot run directory to route between pilot, rescue, Studio publish, or reply.
    run_dir: std::path::PathBuf,

    /// Optional creator request or current concern to include in refreshed launch control.
    #[arg()]
    prompt: Vec<String>,

    /// Root folder containing Autopilot run directories.
    #[arg(long, default_value = ".rs\\autopilot\\runs")]
    root: std::path::PathBuf,

    /// Maximum number of run folders to inspect for project-level packets.
    #[arg(long, default_value_t = 20)]
    limit: usize,

    /// Opportunity id or title to turn into the protected first task. Defaults to the top ranked opportunity.
    #[arg(long)]
    opportunity: Option<String>,

    /// Maximum total snippet characters to embed in nested context packs.
    #[arg(long, default_value_t = 24000)]
    max_chars: usize,

    /// JSON best-friend-control path. Defaults to <run-dir>/best-friend-control.json.
    #[arg(long)]
    out: Option<std::path::PathBuf>,

    /// Markdown best-friend-control path. Defaults to <run-dir>/best-friend-control.md.
    #[arg(long)]
    markdown: Option<std::path::PathBuf>,

    /// Skip writing Markdown companion files.
    #[arg(long)]
    no_markdown: bool,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotBestFriendOperateArgs {
    /// Autopilot run directory to advance by one control-selected offline branch.
    run_dir: std::path::PathBuf,

    /// Optional creator request or current concern to include in refreshed launch control.
    #[arg()]
    prompt: Vec<String>,

    /// Root folder containing Autopilot run directories.
    #[arg(long, default_value = ".rs\\autopilot\\runs")]
    root: std::path::PathBuf,

    /// Maximum number of run folders to inspect for project-level packets.
    #[arg(long, default_value_t = 20)]
    limit: usize,

    /// Opportunity id or title to turn into the protected first task. Defaults to the top ranked opportunity.
    #[arg(long)]
    opportunity: Option<String>,

    /// Maximum total snippet characters to embed in nested context packs.
    #[arg(long, default_value_t = 24000)]
    max_chars: usize,

    /// Select the branch and write receipts without executing the branch.
    #[arg(long)]
    dry_run: bool,

    /// JSON best-friend-operate path. Defaults to <run-dir>/best-friend-operate.json.
    #[arg(long)]
    out: Option<std::path::PathBuf>,

    /// Markdown best-friend-operate path. Defaults to <run-dir>/best-friend-operate.md.
    #[arg(long)]
    markdown: Option<std::path::PathBuf>,

    /// Skip writing Markdown companion files.
    #[arg(long)]
    no_markdown: bool,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotBestFriendRunnerArgs {
    /// Autopilot run directory to advance through bounded operator steps.
    run_dir: std::path::PathBuf,

    /// Optional creator request or current concern to include in refreshed launch control.
    #[arg()]
    prompt: Vec<String>,

    /// Root folder containing Autopilot run directories.
    #[arg(long, default_value = ".rs\\autopilot\\runs")]
    root: std::path::PathBuf,

    /// Maximum number of run folders to inspect for project-level packets.
    #[arg(long, default_value_t = 20)]
    limit: usize,

    /// Opportunity id or title to turn into the protected first task. Defaults to the top ranked opportunity.
    #[arg(long)]
    opportunity: Option<String>,

    /// Maximum total snippet characters to embed in nested context packs.
    #[arg(long, default_value_t = 24000)]
    max_chars: usize,

    /// Maximum operator steps to execute before stopping.
    #[arg(long, default_value_t = 3)]
    max_steps: usize,

    /// Select the first operator branch and write receipts without executing it.
    #[arg(long)]
    dry_run: bool,

    /// JSON best-friend-runner path. Defaults to <run-dir>/best-friend-runner.json.
    #[arg(long)]
    out: Option<std::path::PathBuf>,

    /// Markdown best-friend-runner path. Defaults to <run-dir>/best-friend-runner.md.
    #[arg(long)]
    markdown: Option<std::path::PathBuf>,

    /// Skip writing Markdown companion files.
    #[arg(long)]
    no_markdown: bool,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotFirstTurnArgs {
    /// Autopilot run directory to advance from the best-friend launch packet.
    run_dir: std::path::PathBuf,

    /// Optional creator request to include in refreshed launch context.
    #[arg()]
    prompt: Vec<String>,

    /// Root folder containing Autopilot run directories.
    #[arg(long, default_value = ".rs\\autopilot\\runs")]
    root: std::path::PathBuf,

    /// Maximum number of run folders to inspect for project-level packets.
    #[arg(long, default_value_t = 20)]
    limit: usize,

    /// Opportunity id or title to turn into the first safe task. Defaults to the top ranked opportunity.
    #[arg(long)]
    opportunity: Option<String>,

    /// Maximum total snippet characters to embed in nested context packs.
    #[arg(long, default_value_t = 24000)]
    max_chars: usize,

    /// Select the protected first action without executing it.
    #[arg(long)]
    dry_run: bool,

    /// JSON first-turn path. Defaults to <run-dir>/first-turn.json.
    #[arg(long)]
    out: Option<std::path::PathBuf>,

    /// Markdown first-turn path. Defaults to <run-dir>/first-turn.md.
    #[arg(long)]
    markdown: Option<std::path::PathBuf>,

    /// Skip writing Markdown companion files.
    #[arg(long)]
    no_markdown: bool,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotBestFriendLoopArgs {
    /// Autopilot run directory to advance through protected best-friend turns.
    run_dir: std::path::PathBuf,

    /// Optional creator request to include in refreshed launch context.
    #[arg()]
    prompt: Vec<String>,

    /// Root folder containing Autopilot run directories.
    #[arg(long, default_value = ".rs\\autopilot\\runs")]
    root: std::path::PathBuf,

    /// Maximum number of run folders to inspect for project-level packets.
    #[arg(long, default_value_t = 20)]
    limit: usize,

    /// Opportunity id or title to turn into the first safe task. Defaults to the top ranked opportunity.
    #[arg(long)]
    opportunity: Option<String>,

    /// Maximum total snippet characters to embed in nested context packs.
    #[arg(long, default_value_t = 24000)]
    max_chars: usize,

    /// Maximum protected turns to execute before stopping.
    #[arg(long, default_value_t = 3)]
    max_steps: usize,

    /// Select the first protected action without executing the loop.
    #[arg(long)]
    dry_run: bool,

    /// JSON best-friend-loop path. Defaults to <run-dir>/best-friend-loop.json.
    #[arg(long)]
    out: Option<std::path::PathBuf>,

    /// Markdown best-friend-loop path. Defaults to <run-dir>/best-friend-loop.md.
    #[arg(long)]
    markdown: Option<std::path::PathBuf>,

    /// Skip writing Markdown companion files.
    #[arg(long)]
    no_markdown: bool,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotBestFriendReplyArgs {
    /// Autopilot run directory with best-friend, first-turn, or best-friend-loop evidence.
    run_dir: std::path::PathBuf,

    /// Root folder containing Autopilot run directories.
    #[arg(long, default_value = ".rs\\autopilot\\runs")]
    root: std::path::PathBuf,

    /// Maximum number of run folders to inspect for project-level packets.
    #[arg(long, default_value_t = 20)]
    limit: usize,

    /// JSON best-friend-reply path. Defaults to <run-dir>/best-friend-reply.json.
    #[arg(long)]
    out: Option<std::path::PathBuf>,

    /// Markdown best-friend-reply path. Defaults to <run-dir>/best-friend-reply.md.
    #[arg(long)]
    markdown: Option<std::path::PathBuf>,

    /// Skip writing Markdown companion files.
    #[arg(long)]
    no_markdown: bool,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotBestFriendTurnArgs {
    /// Autopilot run directory to operate from.
    run_dir: std::path::PathBuf,

    /// Optional creator request to include in refreshed best-friend context.
    #[arg()]
    prompt: Vec<String>,

    /// Latest creator message to route before protected best-friend work.
    #[arg(long)]
    message: Option<String>,

    /// Message source label when --message is provided.
    #[arg(long, default_value = "creator")]
    source: String,

    /// Root folder containing Autopilot run directories.
    #[arg(long, default_value = ".rs\\autopilot\\runs")]
    root: std::path::PathBuf,

    /// Maximum number of run folders to inspect for project-level packets.
    #[arg(long, default_value_t = 20)]
    limit: usize,

    /// Opportunity id or title to turn into the first safe task. Defaults to the top ranked opportunity.
    #[arg(long)]
    opportunity: Option<String>,

    /// Maximum total snippet characters to embed in nested context packs.
    #[arg(long, default_value_t = 24000)]
    max_chars: usize,

    /// Maximum protected best-friend turns to execute before stopping.
    #[arg(long, default_value_t = 2)]
    max_steps: usize,

    /// Route and preview the turn without executing protected best-friend work or drafting the final reply.
    #[arg(long)]
    dry_run: bool,

    /// JSON best-friend-turn path. Defaults to <run-dir>/best-friend-turn.json.
    #[arg(long)]
    out: Option<std::path::PathBuf>,

    /// Markdown best-friend-turn path. Defaults to <run-dir>/best-friend-turn.md.
    #[arg(long)]
    markdown: Option<std::path::PathBuf>,

    /// Skip writing Markdown companion files.
    #[arg(long)]
    no_markdown: bool,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotBestFriendSessionArgs {
    /// Creator request to bootstrap, or creator message to route when --run-dir resumes a run.
    #[arg()]
    prompt: Vec<String>,

    /// Latest creator message to route after selecting or bootstrapping the run.
    #[arg(long)]
    message: Option<String>,

    /// Message source label when --message or resumed prompt text is routed.
    #[arg(long, default_value = "creator")]
    source: String,

    /// Existing scout.json to turn into a full offline session before the companion turn.
    #[arg(long)]
    scout: Option<std::path::PathBuf>,

    /// Existing survey.json to scout before bootstrapping.
    #[arg(long)]
    survey: Option<std::path::PathBuf>,

    /// Existing context.json to summarize and scout before bootstrapping.
    #[arg(long)]
    context: Option<std::path::PathBuf>,

    /// Existing run to resume, or explicit output folder for a bootstrapped run.
    #[arg(long)]
    run_dir: Option<std::path::PathBuf>,

    /// Root folder containing Autopilot run directories.
    #[arg(long, default_value = ".rs\\autopilot\\runs")]
    root: std::path::PathBuf,

    /// Override the selected planning scope for bootstrapped runs.
    #[arg(long)]
    scope: Option<String>,

    /// Maximum number of run folders to inspect for project-level packets.
    #[arg(long, default_value_t = 20)]
    limit: usize,

    /// Proceed through intake clarification with explicit assumptions.
    #[arg(long)]
    assume: bool,

    /// Optional smoke suite to append to the bootstrapped plan.
    #[arg(long)]
    smoke: Option<String>,

    /// Opportunity id or title to turn into the first safe task. Defaults to the top ranked opportunity.
    #[arg(long)]
    opportunity: Option<String>,

    /// Maximum total snippet characters to embed in nested context packs.
    #[arg(long, default_value_t = 24000)]
    max_chars: usize,

    /// Maximum protected best-friend turns to execute before stopping.
    #[arg(long, default_value_t = 2)]
    max_steps: usize,

    /// Bootstrap and preview the companion turn without executing protected best-friend work.
    #[arg(long)]
    dry_run: bool,

    /// JSON best-friend-session path. Defaults to <run-dir>/best-friend-session.json or .rs/autopilot/best-friend-session.json.
    #[arg(long)]
    out: Option<std::path::PathBuf>,

    /// Markdown best-friend-session path. Defaults next to the JSON path.
    #[arg(long)]
    markdown: Option<std::path::PathBuf>,

    /// Skip writing Markdown companion files.
    #[arg(long)]
    no_markdown: bool,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotWowSessionArgs {
    /// Creator request to bootstrap, or creative direction to use when --run-dir resumes a run.
    #[arg()]
    prompt: Vec<String>,

    /// Latest creator message to route before preparing the demo.
    #[arg(long)]
    message: Option<String>,

    /// Message source label when --message or resumed prompt text is routed.
    #[arg(long, default_value = "creator")]
    source: String,

    /// Existing scout.json to turn into a full offline session before the demo.
    #[arg(long)]
    scout: Option<std::path::PathBuf>,

    /// Existing survey.json to scout before bootstrapping.
    #[arg(long)]
    survey: Option<std::path::PathBuf>,

    /// Existing context.json to summarize and scout before bootstrapping.
    #[arg(long)]
    context: Option<std::path::PathBuf>,

    /// Existing run to resume, or explicit output folder for a bootstrapped run.
    #[arg(long)]
    run_dir: Option<std::path::PathBuf>,

    /// Root folder containing Autopilot run directories.
    #[arg(long, default_value = ".rs\\autopilot\\runs")]
    root: std::path::PathBuf,

    /// Override the selected planning scope for bootstrapped runs.
    #[arg(long)]
    scope: Option<String>,

    /// Maximum number of run folders to inspect for project-level packets.
    #[arg(long, default_value_t = 20)]
    limit: usize,

    /// Proceed through intake clarification with explicit assumptions.
    #[arg(long)]
    assume: bool,

    /// Optional smoke suite to append to the bootstrapped plan.
    #[arg(long)]
    smoke: Option<String>,

    /// Specific wow idea id or title to demo. Defaults to wow-plan selectedIdea.
    #[arg(long)]
    idea: Option<String>,

    /// Opportunity id or title to turn into the companion first safe task.
    #[arg(long)]
    opportunity: Option<String>,

    /// Maximum number of wow candidates to keep.
    #[arg(long, default_value_t = 5)]
    max_ideas: usize,

    /// Maximum total snippet characters to embed in nested context packs.
    #[arg(long, default_value_t = 24000)]
    max_chars: usize,

    /// Maximum protected best-friend turns to execute before the demo loop.
    #[arg(long, default_value_t = 2)]
    max_steps: usize,

    /// Bootstrap and preview the wow session without generating the offline candidate.
    #[arg(long)]
    dry_run: bool,

    /// JSON wow-session path. Defaults to <run-dir>/wow-session.json or .rs/autopilot/wow-session.json.
    #[arg(long)]
    out: Option<std::path::PathBuf>,

    /// Markdown wow-session path. Defaults next to the JSON path.
    #[arg(long)]
    markdown: Option<std::path::PathBuf>,

    /// Skip writing Markdown companion files.
    #[arg(long)]
    no_markdown: bool,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotBestFriendArcArgs {
    /// Creator request to bootstrap, or creative direction to use when --run-dir resumes a run.
    #[arg()]
    prompt: Vec<String>,

    /// Latest post-demo creator reaction to route after preparing the demo.
    #[arg(long)]
    message: Option<String>,

    /// Message source label when --message is routed.
    #[arg(long, default_value = "creator")]
    source: String,

    /// Existing scout.json to turn into a full offline session before the demo.
    #[arg(long)]
    scout: Option<std::path::PathBuf>,

    /// Existing survey.json to scout before bootstrapping.
    #[arg(long)]
    survey: Option<std::path::PathBuf>,

    /// Existing context.json to summarize and scout before bootstrapping.
    #[arg(long)]
    context: Option<std::path::PathBuf>,

    /// Existing run to resume, or explicit output folder for a bootstrapped run.
    #[arg(long)]
    run_dir: Option<std::path::PathBuf>,

    /// Root folder containing Autopilot run directories.
    #[arg(long, default_value = ".rs\\autopilot\\runs")]
    root: std::path::PathBuf,

    /// Override the selected planning scope for bootstrapped runs.
    #[arg(long)]
    scope: Option<String>,

    /// Maximum number of run folders to inspect for project-level packets.
    #[arg(long, default_value_t = 20)]
    limit: usize,

    /// Proceed through intake clarification with explicit assumptions.
    #[arg(long)]
    assume: bool,

    /// Optional smoke suite to append to the bootstrapped plan.
    #[arg(long)]
    smoke: Option<String>,

    /// Specific wow idea id or title to demo. Defaults to wow-plan selectedIdea.
    #[arg(long)]
    idea: Option<String>,

    /// Opportunity id or title to turn into the companion first safe task.
    #[arg(long)]
    opportunity: Option<String>,

    /// Maximum number of wow candidates to keep.
    #[arg(long, default_value_t = 5)]
    max_ideas: usize,

    /// Maximum total snippet characters to embed in nested context packs.
    #[arg(long, default_value_t = 24000)]
    max_chars: usize,

    /// Maximum protected best-friend turns to execute before the demo loop.
    #[arg(long, default_value_t = 2)]
    max_steps: usize,

    /// Bootstrap and preview the arc without generating the offline candidate.
    #[arg(long)]
    dry_run: bool,

    /// JSON best-friend-arc path. Defaults to <run-dir>/best-friend-arc.json or .rs/autopilot/best-friend-arc.json.
    #[arg(long)]
    out: Option<std::path::PathBuf>,

    /// Markdown best-friend-arc path. Defaults next to the JSON path.
    #[arg(long)]
    markdown: Option<std::path::PathBuf>,

    /// Skip writing Markdown companion files.
    #[arg(long)]
    no_markdown: bool,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotSquadPackArgs {
    /// Autopilot run directory to split into parallel agent assignments.
    run_dir: std::path::PathBuf,

    /// Optional creator request to include in the coordination prompt.
    #[arg()]
    prompt: Vec<String>,

    /// Root folder containing Autopilot run directories.
    #[arg(long, default_value = ".rs\\autopilot\\runs")]
    root: std::path::PathBuf,

    /// Maximum number of run folders to inspect for project-level packets.
    #[arg(long, default_value_t = 20)]
    limit: usize,

    /// Maximum number of agent assignments to prepare.
    #[arg(long, default_value_t = 4)]
    max_tasks: usize,

    /// Maximum total snippet characters to embed in the nested model pack.
    #[arg(long, default_value_t = 24000)]
    max_chars: usize,

    /// JSON squad-pack path. Defaults to <run-dir>/squad-pack.json.
    #[arg(long)]
    out: Option<std::path::PathBuf>,

    /// Markdown squad-pack path. Defaults to <run-dir>/squad-pack.md.
    #[arg(long)]
    markdown: Option<std::path::PathBuf>,

    /// Skip writing the Markdown companion file.
    #[arg(long)]
    no_markdown: bool,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotSquadReviewArgs {
    /// Autopilot run directory containing squad assignments to review.
    run_dir: std::path::PathBuf,

    /// Root folder containing Autopilot run directories.
    #[arg(long, default_value = ".rs\\autopilot\\runs")]
    root: std::path::PathBuf,

    /// Maximum number of run folders to inspect for project-level packets.
    #[arg(long, default_value_t = 20)]
    limit: usize,

    /// Maximum number of agent assignments to review or refresh.
    #[arg(long, default_value_t = 4)]
    max_tasks: usize,

    /// Maximum total snippet characters to embed in the nested model pack.
    #[arg(long, default_value_t = 24000)]
    max_chars: usize,

    /// JSON squad-review path. Defaults to <run-dir>/squad-review.json.
    #[arg(long)]
    out: Option<std::path::PathBuf>,

    /// Markdown squad-review path. Defaults to <run-dir>/squad-review.md.
    #[arg(long)]
    markdown: Option<std::path::PathBuf>,

    /// Skip writing the Markdown companion file.
    #[arg(long)]
    no_markdown: bool,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotWowPlanArgs {
    /// Autopilot run directory to inspect for wow-factor candidates.
    run_dir: std::path::PathBuf,

    /// Optional creator request or creative direction to include in candidate ranking.
    #[arg()]
    prompt: Vec<String>,

    /// Root folder containing Autopilot run directories.
    #[arg(long, default_value = ".rs\\autopilot\\runs")]
    root: std::path::PathBuf,

    /// Maximum number of run folders to inspect for project-level context.
    #[arg(long, default_value_t = 20)]
    limit: usize,

    /// Maximum number of wow candidates to keep.
    #[arg(long, default_value_t = 5)]
    max_ideas: usize,

    /// JSON wow-plan path. Defaults to <run-dir>/wow-plan.json.
    #[arg(long)]
    out: Option<std::path::PathBuf>,

    /// Markdown wow-plan path. Defaults to <run-dir>/wow-plan.md.
    #[arg(long)]
    markdown: Option<std::path::PathBuf>,

    /// Skip writing the Markdown companion file.
    #[arg(long)]
    no_markdown: bool,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotMomentPackArgs {
    /// Autopilot run directory whose wow-plan idea should become an agent packet.
    run_dir: std::path::PathBuf,

    /// Optional creator request or creative direction to include while refreshing wow-plan.
    #[arg()]
    prompt: Vec<String>,

    /// Specific wow idea id or title to implement. Defaults to wow-plan selectedIdea.
    #[arg(long)]
    idea: Option<String>,

    /// Root folder containing Autopilot run directories.
    #[arg(long, default_value = ".rs\\autopilot\\runs")]
    root: std::path::PathBuf,

    /// Maximum number of run folders to inspect for project-level context.
    #[arg(long, default_value_t = 20)]
    limit: usize,

    /// Maximum number of wow candidates to keep while refreshing wow-plan.
    #[arg(long, default_value_t = 5)]
    max_ideas: usize,

    /// Maximum task prompt characters to embed in the packet.
    #[arg(long, default_value_t = 24000)]
    max_chars: usize,

    /// JSON moment-pack path. Defaults to <run-dir>/moment-pack.json.
    #[arg(long)]
    out: Option<std::path::PathBuf>,

    /// Markdown moment-pack path. Defaults to <run-dir>/moment-pack.md.
    #[arg(long)]
    markdown: Option<std::path::PathBuf>,

    /// Skip writing the Markdown companion file.
    #[arg(long)]
    no_markdown: bool,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotMomentSprintArgs {
    /// Autopilot run directory whose selected wow moment should be executed offline.
    run_dir: std::path::PathBuf,

    /// Optional creator request or creative direction to include while refreshing moment-pack.
    #[arg()]
    prompt: Vec<String>,

    /// Specific wow idea id or title to sprint. Defaults to wow-plan selectedIdea.
    #[arg(long)]
    idea: Option<String>,

    /// Root folder containing Autopilot run directories.
    #[arg(long, default_value = ".rs\\autopilot\\runs")]
    root: std::path::PathBuf,

    /// Maximum number of run folders to inspect for project-level context.
    #[arg(long, default_value_t = 20)]
    limit: usize,

    /// Maximum number of wow candidates to keep while refreshing wow-plan.
    #[arg(long, default_value_t = 5)]
    max_ideas: usize,

    /// Maximum task prompt characters to embed in the nested moment-pack.
    #[arg(long, default_value_t = 24000)]
    max_chars: usize,

    /// Plan the safe offline sprint without generating the candidate run.
    #[arg(long)]
    dry_run: bool,

    /// JSON moment-sprint path. Defaults to <run-dir>/moment-sprint.json.
    #[arg(long)]
    out: Option<std::path::PathBuf>,

    /// Markdown moment-sprint path. Defaults to <run-dir>/moment-sprint.md.
    #[arg(long)]
    markdown: Option<std::path::PathBuf>,

    /// Skip writing the Markdown companion file.
    #[arg(long)]
    no_markdown: bool,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotMomentDecisionArgs {
    /// Autopilot source run directory whose wow candidate should be decided.
    run_dir: std::path::PathBuf,

    /// Optional creator request or creative direction to include while refreshing moment-sprint.
    #[arg()]
    prompt: Vec<String>,

    /// Specific wow idea id or title to decide. Defaults to wow-plan selectedIdea.
    #[arg(long)]
    idea: Option<String>,

    /// Root folder containing Autopilot run directories.
    #[arg(long, default_value = ".rs\\autopilot\\runs")]
    root: std::path::PathBuf,

    /// Maximum number of run folders to inspect for project-level context.
    #[arg(long, default_value_t = 20)]
    limit: usize,

    /// Maximum number of wow candidates to keep while refreshing wow-plan.
    #[arg(long, default_value_t = 5)]
    max_ideas: usize,

    /// Maximum task prompt characters to embed in the nested moment-pack.
    #[arg(long, default_value_t = 24000)]
    max_chars: usize,

    /// Plan the decision without generating or comparing the candidate run.
    #[arg(long)]
    dry_run: bool,

    /// JSON moment-decision path. Defaults to <run-dir>/moment-decision.json.
    #[arg(long)]
    out: Option<std::path::PathBuf>,

    /// Markdown moment-decision path. Defaults to <run-dir>/moment-decision.md.
    #[arg(long)]
    markdown: Option<std::path::PathBuf>,

    /// Skip writing the Markdown companion file.
    #[arg(long)]
    no_markdown: bool,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotCreatorDemoArgs {
    /// Autopilot source run directory whose recommended wow run should be presented.
    run_dir: std::path::PathBuf,

    /// Optional creator request or creative direction to include while refreshing moment-decision.
    #[arg()]
    prompt: Vec<String>,

    /// Specific wow idea id or title to demo. Defaults to wow-plan selectedIdea.
    #[arg(long)]
    idea: Option<String>,

    /// Root folder containing Autopilot run directories.
    #[arg(long, default_value = ".rs\\autopilot\\runs")]
    root: std::path::PathBuf,

    /// Maximum number of run folders to inspect for project-level context.
    #[arg(long, default_value_t = 20)]
    limit: usize,

    /// Maximum number of wow candidates to keep while refreshing wow-plan.
    #[arg(long, default_value_t = 5)]
    max_ideas: usize,

    /// Maximum task prompt characters to embed in the nested moment-pack.
    #[arg(long, default_value_t = 24000)]
    max_chars: usize,

    /// Plan the demo packet without generating or comparing the candidate run.
    #[arg(long)]
    dry_run: bool,

    /// JSON creator-demo path. Defaults to <run-dir>/creator-demo.json.
    #[arg(long)]
    out: Option<std::path::PathBuf>,

    /// Markdown creator-demo path. Defaults to <run-dir>/creator-demo.md.
    #[arg(long)]
    markdown: Option<std::path::PathBuf>,

    /// Skip writing the Markdown companion file.
    #[arg(long)]
    no_markdown: bool,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotDemoResponseArgs {
    /// Autopilot source run directory whose creator demo was shown.
    run_dir: std::path::PathBuf,

    /// Creator's post-demo response. Use --message for repeated notes.
    #[arg()]
    response: Vec<String>,

    /// Creator response note to route. Can be repeated.
    #[arg(long)]
    message: Vec<String>,

    /// Specific wow idea id or title to keep while refreshing creator-demo.
    #[arg(long)]
    idea: Option<String>,

    /// Root folder containing Autopilot run directories.
    #[arg(long, default_value = ".rs\\autopilot\\runs")]
    root: std::path::PathBuf,

    /// Maximum number of run folders to inspect for project-level context.
    #[arg(long, default_value_t = 20)]
    limit: usize,

    /// Maximum number of wow candidates to keep while refreshing wow-plan.
    #[arg(long, default_value_t = 5)]
    max_ideas: usize,

    /// Maximum task prompt characters to embed in the nested moment-pack.
    #[arg(long, default_value_t = 24000)]
    max_chars: usize,

    /// Plan response routing without generating or comparing the candidate run.
    #[arg(long)]
    dry_run: bool,

    /// JSON demo-response path. Defaults to <run-dir>/demo-response.json.
    #[arg(long)]
    out: Option<std::path::PathBuf>,

    /// Markdown demo-response path. Defaults to <run-dir>/demo-response.md.
    #[arg(long)]
    markdown: Option<std::path::PathBuf>,

    /// Skip writing the Markdown companion file.
    #[arg(long)]
    no_markdown: bool,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotDemoLoopArgs {
    /// Autopilot source run directory whose creator demo response should be turned into a handoff.
    run_dir: std::path::PathBuf,

    /// Creator's post-demo response. Use --message for repeated notes.
    #[arg()]
    response: Vec<String>,

    /// Creator response note to route. Can be repeated.
    #[arg(long)]
    message: Vec<String>,

    /// Specific wow idea id or title to keep while refreshing creator-demo.
    #[arg(long)]
    idea: Option<String>,

    /// Root folder containing Autopilot run directories.
    #[arg(long, default_value = ".rs\\autopilot\\runs")]
    root: std::path::PathBuf,

    /// Maximum number of run folders to inspect for project-level context.
    #[arg(long, default_value_t = 20)]
    limit: usize,

    /// Maximum number of wow candidates to keep while refreshing wow-plan.
    #[arg(long, default_value_t = 5)]
    max_ideas: usize,

    /// Maximum task prompt characters to embed in the nested moment-pack.
    #[arg(long, default_value_t = 24000)]
    max_chars: usize,

    /// Plan the loop handoff without generating or comparing the candidate run.
    #[arg(long)]
    dry_run: bool,

    /// JSON demo-loop path. Defaults to <run-dir>/demo-loop.json.
    #[arg(long)]
    out: Option<std::path::PathBuf>,

    /// Markdown demo-loop path. Defaults to <run-dir>/demo-loop.md.
    #[arg(long)]
    markdown: Option<std::path::PathBuf>,

    /// Skip writing the Markdown companion file.
    #[arg(long)]
    no_markdown: bool,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotDemoSessionArgs {
    /// Autopilot source run directory whose creator demo response should be handled end-to-end.
    run_dir: std::path::PathBuf,

    /// Creator's post-demo response. Use --message for repeated notes.
    #[arg()]
    response: Vec<String>,

    /// Creator response note to route. Can be repeated.
    #[arg(long)]
    message: Vec<String>,

    /// Specific wow idea id or title to keep while refreshing creator-demo.
    #[arg(long)]
    idea: Option<String>,

    /// Root folder containing Autopilot run directories.
    #[arg(long, default_value = ".rs\\autopilot\\runs")]
    root: std::path::PathBuf,

    /// Maximum number of run folders to inspect for project-level context.
    #[arg(long, default_value_t = 20)]
    limit: usize,

    /// Maximum number of wow candidates to keep while refreshing wow-plan.
    #[arg(long, default_value_t = 5)]
    max_ideas: usize,

    /// Maximum task prompt characters to embed in the nested moment-pack.
    #[arg(long, default_value_t = 24000)]
    max_chars: usize,

    /// Plan response handling without generating or comparing the candidate run.
    #[arg(long)]
    dry_run: bool,

    /// JSON demo-session path. Defaults to <run-dir>/demo-session.json.
    #[arg(long)]
    out: Option<std::path::PathBuf>,

    /// Markdown demo-session path. Defaults to <run-dir>/demo-session.md.
    #[arg(long)]
    markdown: Option<std::path::PathBuf>,

    /// Skip writing the Markdown companion file.
    #[arg(long)]
    no_markdown: bool,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotDemoCheckArgs {
    /// Autopilot source run directory whose post-demo follow-up should be audited.
    run_dir: std::path::PathBuf,

    /// JSON demo-check path. Defaults to <run-dir>/demo-check.json.
    #[arg(long)]
    out: Option<std::path::PathBuf>,

    /// Markdown demo-check path. Defaults to <run-dir>/demo-check.md.
    #[arg(long)]
    markdown: Option<std::path::PathBuf>,

    /// Skip writing the Markdown companion file.
    #[arg(long)]
    no_markdown: bool,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotDemoReplyArgs {
    /// Autopilot source run directory whose checked post-demo state should be turned into a reply.
    run_dir: std::path::PathBuf,

    /// JSON demo-reply path. Defaults to <run-dir>/demo-reply.json.
    #[arg(long)]
    out: Option<std::path::PathBuf>,

    /// Markdown demo-reply path. Defaults to <run-dir>/demo-reply.md.
    #[arg(long)]
    markdown: Option<std::path::PathBuf>,

    /// Skip writing the Markdown companion file.
    #[arg(long)]
    no_markdown: bool,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotDemoLearnArgs {
    /// Autopilot source run directory whose post-demo conversation should become learning signals.
    run_dir: std::path::PathBuf,

    /// JSON demo-learn path. Defaults to <run-dir>/demo-learn.json.
    #[arg(long)]
    out: Option<std::path::PathBuf>,

    /// Markdown demo-learn path. Defaults to <run-dir>/demo-learn.md.
    #[arg(long)]
    markdown: Option<std::path::PathBuf>,

    /// Skip writing the Markdown companion file.
    #[arg(long)]
    no_markdown: bool,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotRememberArgs {
    /// Autopilot source run directory whose post-demo learning should be consolidated.
    run_dir: std::path::PathBuf,

    /// Root folder containing Autopilot run directories.
    #[arg(long, default_value = ".rs\\autopilot\\runs")]
    root: std::path::PathBuf,

    /// Maximum number of run folders to learn from.
    #[arg(long, default_value_t = 20)]
    limit: usize,

    /// Also write best-friend.json from the refreshed memory context.
    #[arg(long)]
    best_friend: bool,

    /// Opportunity id or title to use when also writing best-friend.json.
    #[arg(long)]
    opportunity: Option<String>,

    /// Maximum total snippet characters to embed in the optional best-friend model pack.
    #[arg(long, default_value_t = 24000)]
    max_chars: usize,

    /// JSON remember path. Defaults to <run-dir>/remember.json.
    #[arg(long)]
    out: Option<std::path::PathBuf>,

    /// Markdown remember path. Defaults to <run-dir>/remember.md.
    #[arg(long)]
    markdown: Option<std::path::PathBuf>,

    /// Skip writing the Markdown companion file.
    #[arg(long)]
    no_markdown: bool,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotReviewPackArgs {
    /// Autopilot run directory to package for creator or AI review.
    run_dir: std::path::PathBuf,

    /// JSON review packet path. Defaults to <run-dir>/review-pack.json.
    #[arg(long)]
    out: Option<std::path::PathBuf>,

    /// Markdown review packet path. Defaults to <run-dir>/review-pack.md.
    #[arg(long)]
    markdown: Option<std::path::PathBuf>,

    /// Skip writing the Markdown companion file.
    #[arg(long)]
    no_markdown: bool,

    /// Do not create evidence subdirectories while refreshing the evidence kit.
    #[arg(long)]
    no_create_evidence_dirs: bool,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotPublishReviewArgs {
    /// Autopilot run directory containing plan.json.
    run_dir: std::path::PathBuf,

    /// Studio name, substring, or UUID to publish the panel state into.
    #[arg(long)]
    studio: Option<String>,

    /// Optional companion.json to merge into the Studio review panel packet.
    #[arg(long)]
    companion: Option<std::path::PathBuf>,

    /// Optional best-friend-arc.json to expose the wow moment and checked reply in Studio. Defaults to <run-dir>/best-friend-arc.json when present.
    #[arg(long)]
    arc: Option<std::path::PathBuf>,

    /// Optional best-friend.json to expose AI launch context in Studio. Defaults to <run-dir>/best-friend.json when present.
    #[arg(long)]
    best_friend: Option<std::path::PathBuf>,

    /// Optional best-friend-pilot.json to expose the one-move co-pilot result in Studio. Defaults to <run-dir>/best-friend-pilot.json when present.
    #[arg(long)]
    best_friend_pilot: Option<std::path::PathBuf>,

    /// Optional best-friend-runner.json to expose the bounded co-pilot supervisor in Studio. Defaults to <run-dir>/best-friend-runner.json when present.
    #[arg(long)]
    best_friend_runner: Option<std::path::PathBuf>,

    /// JSON publish receipt path. Defaults to <run-dir>/studio-review.json.
    #[arg(long)]
    out: Option<std::path::PathBuf>,

    /// Markdown publish receipt path. Defaults next to the JSON path.
    #[arg(long)]
    markdown: Option<std::path::PathBuf>,

    /// Skip writing the Markdown companion file.
    #[arg(long)]
    no_markdown: bool,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotPublishPrepArgs {
    /// Autopilot run directory containing plan.json.
    run_dir: std::path::PathBuf,

    /// JSON publish-prep path. Defaults to <run-dir>/publish-prep.json.
    #[arg(long)]
    out: Option<std::path::PathBuf>,

    /// Markdown publish-prep path. Defaults to <run-dir>/publish-prep.md.
    #[arg(long)]
    markdown: Option<std::path::PathBuf>,

    /// Skip writing the Markdown companion file.
    #[arg(long)]
    no_markdown: bool,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotFeedbackArgs {
    /// Autopilot run directory containing plan.json.
    run_dir: std::path::PathBuf,

    /// Creator, playtester, or AI review note. Repeat for multiple notes.
    #[arg(long)]
    note: Vec<String>,

    /// Feedback source label.
    #[arg(long, default_value = "creator")]
    source: String,

    /// JSON feedback triage path. Defaults to <run-dir>/feedback.json.
    #[arg(long)]
    out: Option<std::path::PathBuf>,

    /// Markdown feedback triage path. Defaults to <run-dir>/feedback.md.
    #[arg(long)]
    markdown: Option<std::path::PathBuf>,

    /// Skip writing the Markdown companion file.
    #[arg(long)]
    no_markdown: bool,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotFeedbackPatchArgs {
    /// Autopilot run directory containing plan.json and feedback.json.
    run_dir: std::path::PathBuf,

    /// Feedback triage JSON. Defaults to <run-dir>/feedback.json.
    #[arg(long)]
    feedback: Option<std::path::PathBuf>,

    /// JSON feedback patch work order path. Defaults to <run-dir>/feedback-patch.json.
    #[arg(long)]
    out: Option<std::path::PathBuf>,

    /// Markdown feedback patch work order path. Defaults to <run-dir>/feedback-patch.md.
    #[arg(long)]
    markdown: Option<std::path::PathBuf>,

    /// Feedback-specific planner-pack JSON path. Defaults to <run-dir>/feedback-planner-pack.json.
    #[arg(long)]
    planner_pack: Option<std::path::PathBuf>,

    /// Do not write the feedback-specific planner pack.
    #[arg(long)]
    no_planner_pack: bool,

    /// Skip writing Markdown companion files.
    #[arg(long)]
    no_markdown: bool,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotClaimCheckArgs {
    /// Autopilot run directory containing plan.json.
    run_dir: std::path::PathBuf,

    /// Proposed creator-facing claim to check. Repeat as needed.
    #[arg(long)]
    claim: Vec<String>,

    /// JSON claim-check path. Defaults to <run-dir>/claim-check.json.
    #[arg(long)]
    out: Option<std::path::PathBuf>,

    /// Markdown claim-check path. Defaults to <run-dir>/claim-check.md.
    #[arg(long)]
    markdown: Option<std::path::PathBuf>,

    /// Skip writing the Markdown companion file.
    #[arg(long)]
    no_markdown: bool,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotRespondArgs {
    /// Autopilot run directory containing plan.json.
    run_dir: std::path::PathBuf,

    /// Creator-facing claim to include in the response. Repeat as needed.
    #[arg(long)]
    claim: Vec<String>,

    /// JSON response path. Defaults to <run-dir>/response.json.
    #[arg(long)]
    out: Option<std::path::PathBuf>,

    /// Markdown response path. Defaults to <run-dir>/response.md.
    #[arg(long)]
    markdown: Option<std::path::PathBuf>,

    /// Skip writing Markdown companion files.
    #[arg(long)]
    no_markdown: bool,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotDecisionArgs {
    /// Autopilot run directory containing plan.json.
    run_dir: std::path::PathBuf,

    /// Source label for the entries.
    #[arg(long, default_value = "creator")]
    source: String,

    /// Creator decision to preserve. Repeat as needed.
    #[arg(long)]
    decision: Vec<String>,

    /// Creator constraint to preserve. Repeat as needed.
    #[arg(long)]
    constraint: Vec<String>,

    /// Rejected option or direction to preserve. Repeat as needed.
    #[arg(long)]
    rejection: Vec<String>,

    /// General creator note to preserve. Repeat as needed.
    #[arg(long)]
    note: Vec<String>,

    /// JSON decision ledger path. Defaults to <run-dir>/decisions.json.
    #[arg(long)]
    out: Option<std::path::PathBuf>,

    /// Markdown decision ledger path. Defaults to <run-dir>/decisions.md.
    #[arg(long)]
    markdown: Option<std::path::PathBuf>,

    /// Skip writing the Markdown companion file.
    #[arg(long)]
    no_markdown: bool,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotAlignArgs {
    /// Autopilot run directory containing plan.json.
    run_dir: std::path::PathBuf,

    /// Decision ledger path. Defaults to <run-dir>/decisions.json.
    #[arg(long)]
    decisions: Option<std::path::PathBuf>,

    /// JSON alignment report path. Defaults to <run-dir>/alignment.json.
    #[arg(long)]
    out: Option<std::path::PathBuf>,

    /// Markdown alignment report path. Defaults to <run-dir>/alignment.md.
    #[arg(long)]
    markdown: Option<std::path::PathBuf>,

    /// Skip writing the Markdown companion file.
    #[arg(long)]
    no_markdown: bool,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotJournalArgs {
    /// Autopilot run directory to annotate for AI continuation.
    run_dir: std::path::PathBuf,

    /// Source label for the journal entries.
    #[arg(long, default_value = "agent")]
    source: String,

    /// AI or human work note to preserve. Repeat as needed.
    #[arg(long)]
    entry: Vec<String>,

    /// Command attempted or recommended during the session. Repeat as needed.
    #[arg(long)]
    command: Vec<String>,

    /// Result of a command, check, or investigation. Repeat as needed.
    #[arg(long)]
    result: Vec<String>,

    /// Evidence path or artifact note to preserve. Repeat as needed.
    #[arg(long)]
    evidence: Vec<String>,

    /// JSON journal path. Defaults to <run-dir>/journal.json.
    #[arg(long)]
    out: Option<std::path::PathBuf>,

    /// Markdown journal path. Defaults to <run-dir>/journal.md.
    #[arg(long)]
    markdown: Option<std::path::PathBuf>,

    /// Skip writing the Markdown companion file.
    #[arg(long)]
    no_markdown: bool,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotProofArgs {
    /// Autopilot run directory to audit for claim-ready proof.
    run_dir: std::path::PathBuf,

    /// JSON proof path. Defaults to <run-dir>/proof.json.
    #[arg(long)]
    out: Option<std::path::PathBuf>,

    /// Markdown proof path. Defaults next to the JSON path.
    #[arg(long)]
    markdown: Option<std::path::PathBuf>,

    /// Skip writing the Markdown companion file.
    #[arg(long)]
    no_markdown: bool,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotAcceptanceArgs {
    /// Autopilot run directory to score against creator intent and proof.
    run_dir: std::path::PathBuf,

    /// Optional creator request override. Defaults to plan/request text.
    #[arg()]
    prompt: Vec<String>,

    /// JSON acceptance path. Defaults to <run-dir>/acceptance.json.
    #[arg(long)]
    out: Option<std::path::PathBuf>,

    /// Markdown acceptance path. Defaults next to the JSON path.
    #[arg(long)]
    markdown: Option<std::path::PathBuf>,

    /// Skip writing the Markdown companion file.
    #[arg(long)]
    no_markdown: bool,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotFulfillmentArgs {
    /// Autopilot run directory to check against creator promises.
    run_dir: std::path::PathBuf,

    /// Optional creator request override. Defaults to plan/request text.
    #[arg()]
    prompt: Vec<String>,

    /// JSON fulfillment path. Defaults to <run-dir>/fulfillment.json.
    #[arg(long)]
    out: Option<std::path::PathBuf>,

    /// Markdown fulfillment path. Defaults next to the JSON path.
    #[arg(long)]
    markdown: Option<std::path::PathBuf>,

    /// Skip writing the Markdown companion file.
    #[arg(long)]
    no_markdown: bool,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotCompletionAuditArgs {
    /// Autopilot run directory to audit before claiming completion.
    run_dir: std::path::PathBuf,

    /// Optional creator objective override. Defaults to plan/request text.
    #[arg()]
    prompt: Vec<String>,

    /// JSON completion-audit path. Defaults to <run-dir>/completion-audit.json.
    #[arg(long)]
    out: Option<std::path::PathBuf>,

    /// Markdown completion-audit path. Defaults next to the JSON path.
    #[arg(long)]
    markdown: Option<std::path::PathBuf>,

    /// Skip writing the Markdown companion file.
    #[arg(long)]
    no_markdown: bool,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotDeliverArgs {
    /// Autopilot run directory to summarize for creator delivery.
    run_dir: std::path::PathBuf,

    /// Optional creator objective override. Defaults to plan/request text.
    #[arg()]
    prompt: Vec<String>,

    /// JSON delivery path. Defaults to <run-dir>/delivery.json.
    #[arg(long)]
    out: Option<std::path::PathBuf>,

    /// Markdown delivery path. Defaults next to the JSON path.
    #[arg(long)]
    markdown: Option<std::path::PathBuf>,

    /// Skip writing the Markdown companion file.
    #[arg(long)]
    no_markdown: bool,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotSatisfyArgs {
    /// Autopilot run directory containing fulfillment gaps.
    run_dir: std::path::PathBuf,

    /// Optional creator request override. Defaults to plan/request text.
    #[arg()]
    prompt: Vec<String>,

    /// Patch run output folder. Defaults beside the source run.
    #[arg(long)]
    patch_run: Option<std::path::PathBuf>,

    /// JSON satisfy path. Defaults to <run-dir>/satisfy.json.
    #[arg(long)]
    out: Option<std::path::PathBuf>,

    /// Markdown satisfy path. Defaults next to the JSON path.
    #[arg(long)]
    markdown: Option<std::path::PathBuf>,

    /// Skip writing the Markdown companion file.
    #[arg(long)]
    no_markdown: bool,

    /// Maximum missing creator-promise recipes to include in the patch run.
    #[arg(long, default_value_t = 2)]
    max_recipes: usize,

    /// Optional smoke suite to append to the patch plan.
    #[arg(long)]
    smoke: Option<String>,

    /// Report the patch plan without writing a patch run.
    #[arg(long)]
    dry_run: bool,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotPromiseLoopArgs {
    /// Source Autopilot run directory to extend through safe offline patches.
    run_dir: std::path::PathBuf,

    /// Creator request to satisfy across the source run plus generated patches.
    #[arg()]
    prompt: Vec<String>,

    /// Promise-loop session folder. Defaults beside the source run.
    #[arg(long)]
    out: Option<std::path::PathBuf>,

    /// Markdown promise-loop report path. Defaults to <out>/promise-loop.md.
    #[arg(long)]
    markdown: Option<std::path::PathBuf>,

    /// Skip writing the Markdown companion file.
    #[arg(long)]
    no_markdown: bool,

    /// Maximum offline patch steps to create.
    #[arg(long, default_value_t = 3)]
    max_steps: usize,

    /// Maximum missing recipe promises per patch step.
    #[arg(long, default_value_t = 2)]
    max_recipes: usize,

    /// Optional smoke suite to append to generated patch plans.
    #[arg(long)]
    smoke: Option<String>,

    /// Plan the loop without writing patch runs.
    #[arg(long)]
    dry_run: bool,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotTraceArgs {
    /// Autopilot run directory to map from creator intent to artifacts.
    run_dir: std::path::PathBuf,

    /// Optional creator request override. Defaults to plan/request text.
    #[arg()]
    prompt: Vec<String>,

    /// JSON trace path. Defaults to <run-dir>/trace.json.
    #[arg(long)]
    out: Option<std::path::PathBuf>,

    /// Markdown trace path. Defaults next to the JSON path.
    #[arg(long)]
    markdown: Option<std::path::PathBuf>,

    /// Skip writing the Markdown companion file.
    #[arg(long)]
    no_markdown: bool,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotRefreshArgs {
    /// Autopilot run directory to refresh.
    run_dir: std::path::PathBuf,

    /// JSON refresh report path. Defaults to <run-dir>/refresh.json.
    #[arg(long)]
    out: Option<std::path::PathBuf>,

    /// Markdown refresh report path. Defaults next to the JSON path.
    #[arg(long)]
    markdown: Option<std::path::PathBuf>,

    /// Skip writing the Markdown companion file.
    #[arg(long)]
    no_markdown: bool,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotRollbackArgs {
    /// Autopilot run directory containing apply.json and rollback artifacts.
    run_dir: std::path::PathBuf,

    /// JSON rollback packet path. Defaults to <run-dir>/rollback.json.
    #[arg(long)]
    out: Option<std::path::PathBuf>,

    /// Markdown rollback packet path. Defaults next to the JSON path.
    #[arg(long)]
    markdown: Option<std::path::PathBuf>,

    /// Skip writing the Markdown companion file.
    #[arg(long)]
    no_markdown: bool,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotApprovalArgs {
    /// Autopilot run directory to prepare for creator approval.
    run_dir: std::path::PathBuf,

    /// JSON approval packet path. Defaults to <run-dir>/approval.json.
    #[arg(long)]
    out: Option<std::path::PathBuf>,

    /// Markdown approval packet path. Defaults next to the JSON path.
    #[arg(long)]
    markdown: Option<std::path::PathBuf>,

    /// Skip writing the Markdown companion file.
    #[arg(long)]
    no_markdown: bool,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotPrivacyArgs {
    /// Autopilot run directory to scan.
    run_dir: std::path::PathBuf,

    /// JSON privacy report path. Defaults to <run-dir>/privacy.json.
    #[arg(long)]
    out: Option<std::path::PathBuf>,

    /// Markdown privacy report path. Defaults next to the JSON path.
    #[arg(long)]
    markdown: Option<std::path::PathBuf>,

    /// Skip writing the Markdown companion file.
    #[arg(long)]
    no_markdown: bool,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotNextArgs {
    /// Optional creator request to consider if no active run needs attention.
    #[arg()]
    prompt: Vec<String>,

    /// Specific Autopilot run directory to navigate from.
    #[arg(long)]
    run_dir: Option<std::path::PathBuf>,

    /// Root folder containing Autopilot run directories.
    #[arg(long, default_value = ".rs\\autopilot\\runs")]
    root: std::path::PathBuf,

    /// Maximum number of run folders to inspect when --run-dir is omitted.
    #[arg(long, default_value_t = 20)]
    limit: usize,

    /// JSON navigation path. Defaults to .rs/autopilot/next.json.
    #[arg(long)]
    out: Option<std::path::PathBuf>,

    /// Markdown navigation path. Defaults next to the JSON path.
    #[arg(long)]
    markdown: Option<std::path::PathBuf>,

    /// Skip writing the Markdown companion file.
    #[arg(long)]
    no_markdown: bool,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotOpportunitiesArgs {
    /// Optional creator request to score alongside current run state.
    #[arg()]
    prompt: Vec<String>,

    /// Specific Autopilot run directory to score from.
    #[arg(long)]
    run_dir: Option<std::path::PathBuf>,

    /// Root folder containing Autopilot run directories.
    #[arg(long, default_value = ".rs\\autopilot\\runs")]
    root: std::path::PathBuf,

    /// Maximum number of run folders to inspect when --run-dir is omitted.
    #[arg(long, default_value_t = 20)]
    limit: usize,

    /// JSON opportunity map path. Defaults to <run-dir>/opportunities.json or .rs/autopilot/opportunities.json.
    #[arg(long)]
    out: Option<std::path::PathBuf>,

    /// Markdown opportunity map path. Defaults next to the JSON path.
    #[arg(long)]
    markdown: Option<std::path::PathBuf>,

    /// Skip writing the Markdown companion file.
    #[arg(long)]
    no_markdown: bool,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotWorkOrderArgs {
    /// Optional creator request to score alongside current run state.
    #[arg()]
    prompt: Vec<String>,

    /// Specific Autopilot run directory to create a work order for.
    #[arg(long)]
    run_dir: Option<std::path::PathBuf>,

    /// Opportunity id or title to select. Defaults to the top-ranked opportunity.
    #[arg(long)]
    opportunity: Option<String>,

    /// Root folder containing Autopilot run directories.
    #[arg(long, default_value = ".rs\\autopilot\\runs")]
    root: std::path::PathBuf,

    /// Maximum number of run folders to inspect when --run-dir is omitted.
    #[arg(long, default_value_t = 20)]
    limit: usize,

    /// JSON work order path. Defaults to <run-dir>/work-order.json or .rs/autopilot/work-order.json.
    #[arg(long)]
    out: Option<std::path::PathBuf>,

    /// Markdown work order path. Defaults next to the JSON path.
    #[arg(long)]
    markdown: Option<std::path::PathBuf>,

    /// Skip writing the Markdown companion file.
    #[arg(long)]
    no_markdown: bool,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotWorkCheckArgs {
    /// Specific Autopilot run directory to check.
    run_dir: std::path::PathBuf,

    /// Work-order JSON path. Defaults to <run-dir>/work-order.json.
    #[arg(long)]
    work_order: Option<std::path::PathBuf>,

    /// JSON work-check path. Defaults to <run-dir>/work-check.json.
    #[arg(long)]
    out: Option<std::path::PathBuf>,

    /// Markdown work-check path. Defaults next to the JSON path.
    #[arg(long)]
    markdown: Option<std::path::PathBuf>,

    /// Skip writing the Markdown companion file.
    #[arg(long)]
    no_markdown: bool,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotCycleArgs {
    /// Specific Autopilot run directory to cycle.
    run_dir: std::path::PathBuf,

    /// Optional creator request to score alongside current run state.
    #[arg(long)]
    prompt: Option<String>,

    /// Root folder containing Autopilot run directories.
    #[arg(long, default_value = ".rs\\autopilot\\runs")]
    root: std::path::PathBuf,

    /// Maximum number of run folders to inspect.
    #[arg(long, default_value_t = 20)]
    limit: usize,

    /// JSON cycle path. Defaults to <run-dir>/cycle.json.
    #[arg(long)]
    out: Option<std::path::PathBuf>,

    /// Markdown cycle path. Defaults next to the JSON path.
    #[arg(long)]
    markdown: Option<std::path::PathBuf>,

    /// Skip writing the Markdown companion file.
    #[arg(long)]
    no_markdown: bool,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotDiagnoseArgs {
    /// Specific Autopilot run directory to diagnose.
    run_dir: std::path::PathBuf,

    /// Command that failed or produced confusing output.
    #[arg(long)]
    command: Option<String>,

    /// Short result label such as failed, blocked, or timed out.
    #[arg(long)]
    result: Option<String>,

    /// Error text or diagnostic note. May be repeated.
    #[arg(long)]
    error: Vec<String>,

    /// Evidence path or note that supports the diagnosis. May be repeated.
    #[arg(long)]
    evidence: Vec<String>,

    /// JSON diagnosis path. Defaults to <run-dir>/diagnosis.json.
    #[arg(long)]
    out: Option<std::path::PathBuf>,

    /// Markdown diagnosis path. Defaults next to the JSON path.
    #[arg(long)]
    markdown: Option<std::path::PathBuf>,

    /// Skip writing the Markdown companion file.
    #[arg(long)]
    no_markdown: bool,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotCommandGuardArgs {
    /// Optional Autopilot run directory used to load a default command queue.
    #[arg(long)]
    run_dir: Option<std::path::PathBuf>,

    /// Root folder containing Autopilot run directories.
    #[arg(long, default_value = ".rs\\autopilot\\runs")]
    root: std::path::PathBuf,

    /// Maximum number of run folders to reference in refresh commands.
    #[arg(long, default_value_t = 20)]
    limit: usize,

    /// Command to validate. May be repeated.
    #[arg(long = "command")]
    command: Vec<String>,

    /// Optional text file containing one command per line.
    #[arg(long)]
    from_file: Option<std::path::PathBuf>,

    /// JSON command guard path. Defaults to <run-dir>/command-guard.json or .rs/autopilot/command-guard.json.
    #[arg(long)]
    out: Option<std::path::PathBuf>,

    /// Markdown command guard path. Defaults next to the JSON path.
    #[arg(long)]
    markdown: Option<std::path::PathBuf>,

    /// Skip writing the Markdown companion file.
    #[arg(long)]
    no_markdown: bool,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotSelfCheckArgs {
    /// Autopilot run directory used as evidence for claims and command discovery.
    run_dir: std::path::PathBuf,

    /// Root folder containing Autopilot run directories.
    #[arg(long, default_value = ".rs\\autopilot\\runs")]
    root: std::path::PathBuf,

    /// Maximum number of run folders to reference in refresh commands.
    #[arg(long, default_value_t = 20)]
    limit: usize,

    /// Proposed creator-facing claim to check. Repeat as needed.
    #[arg(long)]
    claim: Vec<String>,

    /// Proposed creator-facing sentence or short message to check. Repeat as needed.
    #[arg(long)]
    message: Vec<String>,

    /// Command to validate before the AI runs it. May be repeated.
    #[arg(long = "command")]
    command: Vec<String>,

    /// Optional text file containing one command per line.
    #[arg(long)]
    from_file: Option<std::path::PathBuf>,

    /// JSON self-check path. Defaults to <run-dir>/self-check.json.
    #[arg(long)]
    out: Option<std::path::PathBuf>,

    /// Markdown self-check path. Defaults to <run-dir>/self-check.md.
    #[arg(long)]
    markdown: Option<std::path::PathBuf>,

    /// Skip writing Markdown companion files.
    #[arg(long)]
    no_markdown: bool,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotRunbookArgs {
    /// Optional creator request used to seed a safe start command when no queue exists.
    prompt: Vec<String>,

    /// Optional Autopilot run directory used to load a default command queue.
    #[arg(long)]
    run_dir: Option<std::path::PathBuf>,

    /// Root folder containing Autopilot run directories.
    #[arg(long, default_value = ".rs\\autopilot\\runs")]
    root: std::path::PathBuf,

    /// Maximum number of run folders to reference in refresh commands.
    #[arg(long, default_value_t = 20)]
    limit: usize,

    /// Maximum number of queued commands to include in the runbook.
    #[arg(long, default_value_t = 6)]
    max_steps: usize,

    /// Command to include in the runbook. May be repeated.
    #[arg(long = "command")]
    command: Vec<String>,

    /// Optional text file containing one command per line.
    #[arg(long)]
    from_file: Option<std::path::PathBuf>,

    /// JSON runbook path. Defaults to <run-dir>/runbook.json or .rs/autopilot/runbook.json.
    #[arg(long)]
    out: Option<std::path::PathBuf>,

    /// Markdown runbook path. Defaults next to the JSON path.
    #[arg(long)]
    markdown: Option<std::path::PathBuf>,

    /// Skip writing Markdown companion files.
    #[arg(long)]
    no_markdown: bool,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotFlightRecorderArgs {
    /// Specific Autopilot run directory to summarize.
    run_dir: std::path::PathBuf,

    /// JSON flight-recorder path. Defaults to <run-dir>/flight-recorder.json.
    #[arg(long)]
    out: Option<std::path::PathBuf>,

    /// Markdown flight-recorder path. Defaults next to the JSON path.
    #[arg(long)]
    markdown: Option<std::path::PathBuf>,

    /// Skip writing the Markdown companion file.
    #[arg(long)]
    no_markdown: bool,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotNavigatorArgs {
    /// Optional creator request used when no run is selected yet.
    prompt: Vec<String>,

    /// Optional Autopilot run directory to navigate.
    #[arg(long)]
    run_dir: Option<std::path::PathBuf>,

    /// Root folder containing Autopilot run directories.
    #[arg(long, default_value = ".rs\\autopilot\\runs")]
    root: std::path::PathBuf,

    /// Maximum number of run folders to inspect for orientation.
    #[arg(long, default_value_t = 20)]
    limit: usize,

    /// JSON navigator path. Defaults to <run-dir>/navigator.json or .rs/autopilot/navigator.json.
    #[arg(long)]
    out: Option<std::path::PathBuf>,

    /// Markdown navigator path. Defaults next to the JSON path.
    #[arg(long)]
    markdown: Option<std::path::PathBuf>,

    /// Skip writing Markdown companion files.
    #[arg(long)]
    no_markdown: bool,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotAdvanceArgs {
    /// Specific Autopilot run directory to advance by one safe action.
    run_dir: std::path::PathBuf,

    /// Root folder containing Autopilot run directories.
    #[arg(long, default_value = ".rs\\autopilot\\runs")]
    root: std::path::PathBuf,

    /// Maximum number of run folders to inspect for navigator context.
    #[arg(long, default_value_t = 20)]
    limit: usize,

    /// Select the navigator action without executing it.
    #[arg(long)]
    dry_run: bool,

    /// JSON advance receipt path. Defaults to <run-dir>/advance.json.
    #[arg(long)]
    out: Option<std::path::PathBuf>,

    /// Markdown advance receipt path. Defaults next to the JSON path.
    #[arg(long)]
    markdown: Option<std::path::PathBuf>,

    /// Skip writing Markdown companion files.
    #[arg(long)]
    no_markdown: bool,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotActArgs {
    /// Specific Autopilot run directory to act on.
    run_dir: std::path::PathBuf,

    /// Explicit command to execute. Defaults to diagnosis, cycle, or work-order next action.
    #[arg(long)]
    command: Option<String>,

    /// Selection source: auto, diagnosis, cycle, work-order, or agenda.
    #[arg(long, default_value = "auto")]
    source: String,

    /// Root folder containing Autopilot run directories.
    #[arg(long, default_value = ".rs\\autopilot\\runs")]
    root: std::path::PathBuf,

    /// Maximum number of run folders to inspect for generated opportunity/cycle context.
    #[arg(long, default_value_t = 20)]
    limit: usize,

    /// Explain the selected action without executing it.
    #[arg(long)]
    dry_run: bool,

    /// JSON action receipt path. Defaults to <run-dir>/act.json.
    #[arg(long)]
    out: Option<std::path::PathBuf>,

    /// Markdown action receipt path. Defaults next to the JSON path.
    #[arg(long)]
    markdown: Option<std::path::PathBuf>,

    /// Skip writing the Markdown companion file.
    #[arg(long)]
    no_markdown: bool,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotLoopArgs {
    /// Specific Autopilot run directory to advance.
    run_dir: std::path::PathBuf,

    /// Root folder containing Autopilot run directories.
    #[arg(long, default_value = ".rs\\autopilot\\runs")]
    root: std::path::PathBuf,

    /// Maximum number of run folders to inspect for generated opportunity/cycle context.
    #[arg(long, default_value_t = 20)]
    limit: usize,

    /// Maximum safe offline actions to execute before stopping.
    #[arg(long, default_value_t = 3)]
    max_steps: usize,

    /// Select the next action without executing it.
    #[arg(long)]
    dry_run: bool,

    /// JSON loop receipt path. Defaults to <run-dir>/loop.json.
    #[arg(long)]
    out: Option<std::path::PathBuf>,

    /// Markdown loop receipt path. Defaults next to the JSON path.
    #[arg(long)]
    markdown: Option<std::path::PathBuf>,

    /// Skip writing the Markdown companion file.
    #[arg(long)]
    no_markdown: bool,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotRoadmapArgs {
    /// Optional creator request to turn into backlog items.
    #[arg()]
    prompt: Vec<String>,

    /// Root folder containing Autopilot run directories.
    #[arg(long, default_value = ".rs\\autopilot\\runs")]
    root: std::path::PathBuf,

    /// Maximum number of run folders to inspect.
    #[arg(long, default_value_t = 20)]
    limit: usize,

    /// JSON roadmap path. Defaults to .rs/autopilot/roadmap.json.
    #[arg(long)]
    out: Option<std::path::PathBuf>,

    /// Markdown roadmap path. Defaults next to the JSON path.
    #[arg(long)]
    markdown: Option<std::path::PathBuf>,

    /// Skip writing the Markdown companion file.
    #[arg(long)]
    no_markdown: bool,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotJudgeArgs {
    /// Autopilot run directory to judge.
    run_dir: std::path::PathBuf,

    /// JSON judgment path. Defaults to <run-dir>/judgment.json.
    #[arg(long)]
    out: Option<std::path::PathBuf>,

    /// Markdown judgment path. Defaults to <run-dir>/judgment.md.
    #[arg(long)]
    markdown: Option<std::path::PathBuf>,

    /// Skip writing the Markdown companion file.
    #[arg(long)]
    no_markdown: bool,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotCritiqueArgs {
    /// Autopilot run directory containing plan.json.
    #[arg(long)]
    run_dir: Option<std::path::PathBuf>,

    /// Autopilot plan JSON. Defaults the run directory to the plan's parent folder.
    #[arg(long)]
    plan: Option<std::path::PathBuf>,

    /// JSON critique path. Defaults to <run-dir>/gameplay-critique.json.
    #[arg(long)]
    out: Option<std::path::PathBuf>,

    /// Markdown critique path. Defaults to <run-dir>/gameplay-critique.md.
    #[arg(long)]
    markdown: Option<std::path::PathBuf>,

    /// Skip writing the Markdown companion file.
    #[arg(long)]
    no_markdown: bool,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotPlaytestArgs {
    /// Autopilot run directory containing plan.json.
    #[arg(long)]
    run_dir: Option<std::path::PathBuf>,

    /// Autopilot plan JSON. Defaults the run directory to the plan's parent folder.
    #[arg(long)]
    plan: Option<std::path::PathBuf>,

    /// JSON playtest path. Defaults to <run-dir>/playtest-plan.json.
    #[arg(long)]
    out: Option<std::path::PathBuf>,

    /// Markdown playtest path. Defaults to <run-dir>/playtest-plan.md.
    #[arg(long)]
    markdown: Option<std::path::PathBuf>,

    /// Skip writing the Markdown companion file.
    #[arg(long)]
    no_markdown: bool,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotSimulateArgs {
    /// Autopilot run directory containing plan.json.
    #[arg(long)]
    run_dir: Option<std::path::PathBuf>,

    /// Autopilot plan JSON. Defaults the run directory to the plan's parent folder.
    #[arg(long)]
    plan: Option<std::path::PathBuf>,

    /// JSON simulation path. Defaults to <run-dir>/simulation.json.
    #[arg(long)]
    out: Option<std::path::PathBuf>,

    /// Markdown simulation path. Defaults to <run-dir>/simulation.md.
    #[arg(long)]
    markdown: Option<std::path::PathBuf>,

    /// Skip writing the Markdown companion file.
    #[arg(long)]
    no_markdown: bool,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotGraphArgs {
    /// Autopilot run directory containing plan.json.
    #[arg(long)]
    run_dir: Option<std::path::PathBuf>,

    /// Autopilot plan JSON. Defaults the run directory to the plan's parent folder.
    #[arg(long)]
    plan: Option<std::path::PathBuf>,

    /// JSON graph path. Defaults to <run-dir>/feature-graph.json.
    #[arg(long)]
    out: Option<std::path::PathBuf>,

    /// Markdown graph path. Defaults to <run-dir>/feature-graph.md.
    #[arg(long)]
    markdown: Option<std::path::PathBuf>,

    /// Skip writing the Markdown companion file.
    #[arg(long)]
    no_markdown: bool,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotBalanceArgs {
    /// Autopilot run directory containing plan.json.
    #[arg(long)]
    run_dir: Option<std::path::PathBuf>,

    /// Autopilot plan JSON. Defaults the run directory to the plan's parent folder.
    #[arg(long)]
    plan: Option<std::path::PathBuf>,

    /// JSON balance path. Defaults to <run-dir>/balance.json.
    #[arg(long)]
    out: Option<std::path::PathBuf>,

    /// Markdown balance path. Defaults to <run-dir>/balance.md.
    #[arg(long)]
    markdown: Option<std::path::PathBuf>,

    /// Skip writing the Markdown companion file.
    #[arg(long)]
    no_markdown: bool,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotImpactArgs {
    /// Autopilot run directory containing plan.json.
    run_dir: Option<std::path::PathBuf>,

    /// Autopilot plan JSON. Defaults the run directory to the plan's parent folder.
    #[arg(long)]
    plan: Option<std::path::PathBuf>,

    /// JSON impact report path. Defaults to <run-dir>/impact.json.
    #[arg(long)]
    out: Option<std::path::PathBuf>,

    /// Markdown impact report path. Defaults to <run-dir>/impact.md.
    #[arg(long)]
    markdown: Option<std::path::PathBuf>,

    /// Skip writing the Markdown companion file.
    #[arg(long)]
    no_markdown: bool,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotContractsArgs {
    /// Autopilot run directory containing plan.json.
    run_dir: Option<std::path::PathBuf>,

    /// Autopilot plan JSON. Defaults the run directory to the plan's parent folder.
    #[arg(long)]
    plan: Option<std::path::PathBuf>,

    /// JSON contracts path. Defaults to <run-dir>/contracts.json.
    #[arg(long)]
    out: Option<std::path::PathBuf>,

    /// Markdown contracts path. Defaults to <run-dir>/contracts.md.
    #[arg(long)]
    markdown: Option<std::path::PathBuf>,

    /// Skip writing the Markdown companion file.
    #[arg(long)]
    no_markdown: bool,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotAuthorityArgs {
    /// Autopilot run directory containing plan.json.
    run_dir: Option<std::path::PathBuf>,

    /// Autopilot plan JSON. Defaults the run directory to the plan's parent folder.
    #[arg(long)]
    plan: Option<std::path::PathBuf>,

    /// JSON authority audit path. Defaults to <run-dir>/authority.json.
    #[arg(long)]
    out: Option<std::path::PathBuf>,

    /// Markdown authority audit path. Defaults to <run-dir>/authority.md.
    #[arg(long)]
    markdown: Option<std::path::PathBuf>,

    /// Skip writing the Markdown companion file.
    #[arg(long)]
    no_markdown: bool,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotUxArgs {
    /// Autopilot run directory containing plan.json.
    run_dir: Option<std::path::PathBuf>,

    /// Autopilot plan JSON. Defaults the run directory to the plan's parent folder.
    #[arg(long)]
    plan: Option<std::path::PathBuf>,

    /// JSON UX audit path. Defaults to <run-dir>/ux.json.
    #[arg(long)]
    out: Option<std::path::PathBuf>,

    /// Markdown UX audit path. Defaults to <run-dir>/ux.md.
    #[arg(long)]
    markdown: Option<std::path::PathBuf>,

    /// Skip writing the Markdown companion file.
    #[arg(long)]
    no_markdown: bool,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotCopyDeckArgs {
    /// Autopilot run directory containing plan.json.
    run_dir: Option<std::path::PathBuf>,

    /// Autopilot plan JSON. Defaults the run directory to the plan's parent folder.
    #[arg(long)]
    plan: Option<std::path::PathBuf>,

    /// JSON copy deck path. Defaults to <run-dir>/copy-deck.json.
    #[arg(long)]
    out: Option<std::path::PathBuf>,

    /// Markdown copy deck path. Defaults to <run-dir>/copy-deck.md.
    #[arg(long)]
    markdown: Option<std::path::PathBuf>,

    /// Skip writing the Markdown companion file.
    #[arg(long)]
    no_markdown: bool,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotPerformanceArgs {
    /// Autopilot run directory containing plan.json.
    run_dir: Option<std::path::PathBuf>,

    /// Autopilot plan JSON. Defaults the run directory to the plan's parent folder.
    #[arg(long)]
    plan: Option<std::path::PathBuf>,

    /// JSON performance audit path. Defaults to <run-dir>/performance.json.
    #[arg(long)]
    out: Option<std::path::PathBuf>,

    /// Markdown performance audit path. Defaults to <run-dir>/performance.md.
    #[arg(long)]
    markdown: Option<std::path::PathBuf>,

    /// Skip writing the Markdown companion file.
    #[arg(long)]
    no_markdown: bool,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotAccessibilityArgs {
    /// Autopilot run directory containing plan.json.
    run_dir: Option<std::path::PathBuf>,

    /// Autopilot plan JSON. Defaults the run directory to the plan's parent folder.
    #[arg(long)]
    plan: Option<std::path::PathBuf>,

    /// JSON accessibility audit path. Defaults to <run-dir>/accessibility.json.
    #[arg(long)]
    out: Option<std::path::PathBuf>,

    /// Markdown accessibility audit path. Defaults to <run-dir>/accessibility.md.
    #[arg(long)]
    markdown: Option<std::path::PathBuf>,

    /// Skip writing the Markdown companion file.
    #[arg(long)]
    no_markdown: bool,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotPolicyArgs {
    /// Autopilot run directory containing plan.json.
    run_dir: Option<std::path::PathBuf>,

    /// Autopilot plan JSON. Defaults the run directory to the plan's parent folder.
    #[arg(long)]
    plan: Option<std::path::PathBuf>,

    /// JSON policy audit path. Defaults to <run-dir>/policy.json.
    #[arg(long)]
    out: Option<std::path::PathBuf>,

    /// Markdown policy audit path. Defaults to <run-dir>/policy.md.
    #[arg(long)]
    markdown: Option<std::path::PathBuf>,

    /// Skip writing the Markdown companion file.
    #[arg(long)]
    no_markdown: bool,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotAssetBriefArgs {
    /// Autopilot run directory containing plan.json.
    run_dir: Option<std::path::PathBuf>,

    /// Autopilot plan JSON. Defaults the run directory to the plan's parent folder.
    #[arg(long)]
    plan: Option<std::path::PathBuf>,

    /// JSON asset brief path. Defaults to <run-dir>/asset-brief.json.
    #[arg(long)]
    out: Option<std::path::PathBuf>,

    /// Markdown asset brief path. Defaults to <run-dir>/asset-brief.md.
    #[arg(long)]
    markdown: Option<std::path::PathBuf>,

    /// Skip writing the Markdown companion file.
    #[arg(long)]
    no_markdown: bool,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotStyleGuideArgs {
    /// Autopilot run directory containing plan.json.
    run_dir: Option<std::path::PathBuf>,

    /// Autopilot plan JSON. Defaults the run directory to the plan's parent folder.
    #[arg(long)]
    plan: Option<std::path::PathBuf>,

    /// JSON style guide path. Defaults to <run-dir>/style-guide.json.
    #[arg(long)]
    out: Option<std::path::PathBuf>,

    /// Markdown style guide path. Defaults to <run-dir>/style-guide.md.
    #[arg(long)]
    markdown: Option<std::path::PathBuf>,

    /// Skip writing the Markdown companion file.
    #[arg(long)]
    no_markdown: bool,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotWorldBlueprintArgs {
    /// Autopilot run directory containing plan.json.
    run_dir: Option<std::path::PathBuf>,

    /// Autopilot plan JSON. Defaults the run directory to the plan's parent folder.
    #[arg(long)]
    plan: Option<std::path::PathBuf>,

    /// JSON world blueprint path. Defaults to <run-dir>/world-blueprint.json.
    #[arg(long)]
    out: Option<std::path::PathBuf>,

    /// Markdown world blueprint path. Defaults to <run-dir>/world-blueprint.md.
    #[arg(long)]
    markdown: Option<std::path::PathBuf>,

    /// Skip writing the Markdown companion file.
    #[arg(long)]
    no_markdown: bool,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotOnboardingArgs {
    /// Autopilot run directory containing plan.json.
    run_dir: Option<std::path::PathBuf>,

    /// Autopilot plan JSON. Defaults the run directory to the plan's parent folder.
    #[arg(long)]
    plan: Option<std::path::PathBuf>,

    /// JSON onboarding path. Defaults to <run-dir>/onboarding.json.
    #[arg(long)]
    out: Option<std::path::PathBuf>,

    /// Markdown onboarding path. Defaults to <run-dir>/onboarding.md.
    #[arg(long)]
    markdown: Option<std::path::PathBuf>,

    /// Skip writing the Markdown companion file.
    #[arg(long)]
    no_markdown: bool,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotShowcaseArgs {
    /// Autopilot run directory containing plan.json.
    run_dir: Option<std::path::PathBuf>,

    /// Autopilot plan JSON. Defaults the run directory to the plan's parent folder.
    #[arg(long)]
    plan: Option<std::path::PathBuf>,

    /// JSON showcase path. Defaults to <run-dir>/showcase.json.
    #[arg(long)]
    out: Option<std::path::PathBuf>,

    /// Markdown showcase path. Defaults to <run-dir>/showcase.md.
    #[arg(long)]
    markdown: Option<std::path::PathBuf>,

    /// Skip writing the Markdown companion file.
    #[arg(long)]
    no_markdown: bool,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotTelemetryArgs {
    /// Autopilot run directory containing plan.json.
    run_dir: Option<std::path::PathBuf>,

    /// Autopilot plan JSON. Defaults the run directory to the plan's parent folder.
    #[arg(long)]
    plan: Option<std::path::PathBuf>,

    /// JSON telemetry path. Defaults to <run-dir>/telemetry.json.
    #[arg(long)]
    out: Option<std::path::PathBuf>,

    /// Markdown telemetry path. Defaults to <run-dir>/telemetry.md.
    #[arg(long)]
    markdown: Option<std::path::PathBuf>,

    /// Skip writing the Markdown companion file.
    #[arg(long)]
    no_markdown: bool,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotMonetizationArgs {
    /// Autopilot run directory containing plan.json.
    run_dir: Option<std::path::PathBuf>,

    /// Autopilot plan JSON. Defaults the run directory to the plan's parent folder.
    #[arg(long)]
    plan: Option<std::path::PathBuf>,

    /// JSON monetization path. Defaults to <run-dir>/monetization.json.
    #[arg(long)]
    out: Option<std::path::PathBuf>,

    /// Markdown monetization path. Defaults to <run-dir>/monetization.md.
    #[arg(long)]
    markdown: Option<std::path::PathBuf>,

    /// Skip writing the Markdown companion file.
    #[arg(long)]
    no_markdown: bool,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotSocialArgs {
    /// Autopilot run directory containing plan.json.
    run_dir: Option<std::path::PathBuf>,

    /// Autopilot plan JSON. Defaults the run directory to the plan's parent folder.
    #[arg(long)]
    plan: Option<std::path::PathBuf>,

    /// JSON social plan path. Defaults to <run-dir>/social.json.
    #[arg(long)]
    out: Option<std::path::PathBuf>,

    /// Markdown social plan path. Defaults to <run-dir>/social.md.
    #[arg(long)]
    markdown: Option<std::path::PathBuf>,

    /// Skip writing the Markdown companion file.
    #[arg(long)]
    no_markdown: bool,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotLiveopsArgs {
    /// Autopilot run directory containing plan.json.
    run_dir: Option<std::path::PathBuf>,

    /// Autopilot plan JSON. Defaults the run directory to the plan's parent folder.
    #[arg(long)]
    plan: Option<std::path::PathBuf>,

    /// JSON liveops plan path. Defaults to <run-dir>/liveops.json.
    #[arg(long)]
    out: Option<std::path::PathBuf>,

    /// Markdown liveops plan path. Defaults to <run-dir>/liveops.md.
    #[arg(long)]
    markdown: Option<std::path::PathBuf>,

    /// Skip writing the Markdown companion file.
    #[arg(long)]
    no_markdown: bool,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotPersistenceArgs {
    /// Autopilot run directory containing plan.json.
    run_dir: Option<std::path::PathBuf>,

    /// Autopilot plan JSON. Defaults the run directory to the plan's parent folder.
    #[arg(long)]
    plan: Option<std::path::PathBuf>,

    /// JSON persistence plan path. Defaults to <run-dir>/persistence.json.
    #[arg(long)]
    out: Option<std::path::PathBuf>,

    /// Markdown persistence plan path. Defaults to <run-dir>/persistence.md.
    #[arg(long)]
    markdown: Option<std::path::PathBuf>,

    /// Skip writing the Markdown companion file.
    #[arg(long)]
    no_markdown: bool,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotEvidenceArgs {
    /// Autopilot run directory containing plan.json.
    run_dir: std::path::PathBuf,

    /// Evidence folder root. Defaults to <run-dir>/evidence.
    #[arg(long)]
    evidence_dir: Option<std::path::PathBuf>,

    /// JSON evidence kit path. Defaults to <run-dir>/evidence-kit.json.
    #[arg(long)]
    out: Option<std::path::PathBuf>,

    /// Markdown evidence kit path. Defaults to <run-dir>/evidence-kit.md.
    #[arg(long)]
    markdown: Option<std::path::PathBuf>,

    /// Skip writing the Markdown companion file.
    #[arg(long)]
    no_markdown: bool,

    /// Do not create evidence subdirectories.
    #[arg(long)]
    no_create_dirs: bool,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotRecordPlaytestArgs {
    /// Autopilot run directory containing plan.json.
    run_dir: std::path::PathBuf,

    /// Live playtest result: passed, failed, blocked, or inconclusive.
    #[arg(long, default_value = "passed")]
    result: String,

    /// Evidence item or local path observed during live playtest. Repeat as needed.
    #[arg(long)]
    evidence: Vec<String>,

    /// Human or AI note to preserve with the result. Repeat as needed.
    #[arg(long)]
    note: Vec<String>,

    /// Scenario result in the form scenario-id=result. Repeat as needed.
    #[arg(long)]
    scenario: Vec<String>,

    /// JSON result path. Defaults to <run-dir>/playtest-result.json.
    #[arg(long)]
    out: Option<std::path::PathBuf>,

    /// Markdown result path. Defaults to <run-dir>/playtest-result.md.
    #[arg(long)]
    markdown: Option<std::path::PathBuf>,

    /// Skip writing the Markdown companion file.
    #[arg(long)]
    no_markdown: bool,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotEvidenceReviewArgs {
    /// Autopilot run directory containing playtest-result.json.
    run_dir: std::path::PathBuf,

    /// JSON evidence review path. Defaults to <run-dir>/evidence-review.json.
    #[arg(long)]
    out: Option<std::path::PathBuf>,

    /// Markdown evidence review path. Defaults to <run-dir>/evidence-review.md.
    #[arg(long)]
    markdown: Option<std::path::PathBuf>,

    /// Skip writing the Markdown companion file.
    #[arg(long)]
    no_markdown: bool,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotHealthArgs {
    /// Autopilot run directory to verify after live apply.
    run_dir: std::path::PathBuf,

    /// JSON health report path. Defaults to <run-dir>/health.json.
    #[arg(long)]
    out: Option<std::path::PathBuf>,

    /// Markdown health report path. Defaults to <run-dir>/health.md.
    #[arg(long)]
    markdown: Option<std::path::PathBuf>,

    /// Skip writing the Markdown companion file.
    #[arg(long)]
    no_markdown: bool,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotRepairPlanArgs {
    /// Autopilot run directory containing playtest-result.json.
    run_dir: std::path::PathBuf,

    /// JSON repair plan path. Defaults to <run-dir>/repair-plan.json.
    #[arg(long)]
    out: Option<std::path::PathBuf>,

    /// Markdown repair plan path. Defaults to <run-dir>/repair-plan.md.
    #[arg(long)]
    markdown: Option<std::path::PathBuf>,

    /// Skip writing the Markdown companion file.
    #[arg(long)]
    no_markdown: bool,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotImproveArgs {
    /// Autopilot run directory containing plan.json.
    #[arg(long)]
    run_dir: Option<std::path::PathBuf>,

    /// Autopilot plan JSON. Defaults the source run directory to the plan's parent folder.
    #[arg(long)]
    plan: Option<std::path::PathBuf>,

    /// Patch run output folder. Defaults beside the source run.
    #[arg(long)]
    out: Option<std::path::PathBuf>,

    /// Markdown improvement report path. Defaults to <patch-run>/improve.md.
    #[arg(long)]
    markdown: Option<std::path::PathBuf>,

    /// Skip writing the Markdown companion file.
    #[arg(long)]
    no_markdown: bool,

    /// Force specific patch recipes instead of selecting from critique gaps.
    #[arg(long, value_delimiter = ',')]
    recipe: Vec<String>,

    /// Maximum critique-suggested recipes to include.
    #[arg(long, default_value_t = 2)]
    max_recipes: usize,

    /// Optional smoke suite to append to the patch plan.
    #[arg(long)]
    smoke: Option<String>,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotCompareArgs {
    /// Baseline Autopilot run directory.
    #[arg(long)]
    base_run: std::path::PathBuf,

    /// Candidate Autopilot run directory to compare against the baseline.
    #[arg(long)]
    candidate_run: std::path::PathBuf,

    /// JSON comparison path. Defaults to <candidate-run>/comparison.json.
    #[arg(long)]
    out: Option<std::path::PathBuf>,

    /// Markdown comparison path. Defaults to <candidate-run>/comparison.md.
    #[arg(long)]
    markdown: Option<std::path::PathBuf>,

    /// Skip writing the Markdown companion file.
    #[arg(long)]
    no_markdown: bool,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotIterateArgs {
    /// Source Autopilot run directory to improve.
    #[arg(long)]
    run_dir: std::path::PathBuf,

    /// Iteration session folder. Defaults beside the source run.
    #[arg(long)]
    out: Option<std::path::PathBuf>,

    /// Markdown iteration report path. Defaults to <out>/iteration.md.
    #[arg(long)]
    markdown: Option<std::path::PathBuf>,

    /// Skip writing the Markdown companion file.
    #[arg(long)]
    no_markdown: bool,

    /// Maximum offline improvement steps.
    #[arg(long, default_value_t = 3)]
    max_steps: usize,

    /// Maximum critique-suggested recipes per step.
    #[arg(long, default_value_t = 2)]
    max_recipes: usize,

    /// Optional smoke suite to append to generated patch plans.
    #[arg(long)]
    smoke: Option<String>,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotSequenceArgs {
    /// Ordered Autopilot run directories. Repeat for baseline then patches.
    #[arg(long = "run-dir", required = true)]
    run_dirs: Vec<std::path::PathBuf>,

    /// JSON sequence path. Defaults to .rs/autopilot/sequence.json.
    #[arg(long)]
    out: Option<std::path::PathBuf>,

    /// Markdown sequence path. Defaults next to the JSON path.
    #[arg(long)]
    markdown: Option<std::path::PathBuf>,

    /// Skip writing the Markdown companion file.
    #[arg(long)]
    no_markdown: bool,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotArchitectArgs {
    /// Creator request to turn into a staged build blueprint.
    #[arg()]
    prompt: Vec<String>,

    /// Root folder containing Autopilot run directories.
    #[arg(long, default_value = ".rs\\autopilot\\runs")]
    root: std::path::PathBuf,

    /// Maximum number of existing runs to inspect for continuity warnings.
    #[arg(long, default_value_t = 5)]
    limit: usize,

    /// JSON architecture path. Defaults to .rs/autopilot/architect.json.
    #[arg(long)]
    out: Option<std::path::PathBuf>,

    /// Markdown architecture path. Defaults to .rs/autopilot/architect.md.
    #[arg(long)]
    markdown: Option<std::path::PathBuf>,

    /// Skip writing the Markdown companion file.
    #[arg(long)]
    no_markdown: bool,

    /// Smoke suite to include in generated phase commands.
    #[arg(long)]
    smoke: Option<String>,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotKickoffArgs {
    /// Creator request to turn into a ready-to-review offline run packet.
    #[arg()]
    prompt: Vec<String>,

    /// Studio name, substring, or UUID. Stored in the generated plan for later apply.
    #[arg(long)]
    studio: Option<String>,

    /// Studio path used as the operation root.
    #[arg(long, default_value = "game")]
    scope: String,

    /// Root folder containing prior Autopilot run directories.
    #[arg(long, default_value = ".rs\\autopilot\\runs")]
    root: std::path::PathBuf,

    /// Maximum number of existing runs to inspect for continuity warnings.
    #[arg(long, default_value_t = 5)]
    limit: usize,

    /// Run output folder. Defaults to a timestamped .rs/autopilot/runs folder.
    #[arg(long)]
    out: Option<std::path::PathBuf>,

    /// Smoke suite to append to the composed plan and apply command.
    #[arg(long)]
    smoke: Option<String>,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotAuditSourcesArgs {
    /// Autopilot run directory containing generated sources and plan.json.
    run_dir: std::path::PathBuf,

    /// Plan JSON. Defaults to <run-dir>/plan.json.
    #[arg(long)]
    plan: Option<std::path::PathBuf>,

    /// JSON source audit path. Defaults to <run-dir>/source-audit.json.
    #[arg(long)]
    out: Option<std::path::PathBuf>,

    /// Markdown source audit path. Defaults to <run-dir>/source-audit.md.
    #[arg(long)]
    markdown: Option<std::path::PathBuf>,

    /// Skip writing the Markdown companion file.
    #[arg(long)]
    no_markdown: bool,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotPlannerPackArgs {
    /// Creator request to package for an AI planner. Defaults to run metadata when --run-dir is provided.
    #[arg()]
    prompt: Vec<String>,

    /// Existing Autopilot run directory to include as redacted planning context.
    #[arg(long)]
    run_dir: Option<std::path::PathBuf>,

    /// Context JSON to include. Defaults to <run-dir>/context.json when --run-dir is provided.
    #[arg(long)]
    context: Option<std::path::PathBuf>,

    /// JSON planner-pack path. Defaults to <run-dir>/planner-pack.json or .rs/autopilot/planner-pack.json.
    #[arg(long)]
    out: Option<std::path::PathBuf>,

    /// Markdown planner-pack path. Defaults to <run-dir>/planner-pack.md or .rs/autopilot/planner-pack.md.
    #[arg(long)]
    markdown: Option<std::path::PathBuf>,

    /// Skip writing the Markdown companion file.
    #[arg(long)]
    no_markdown: bool,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotAdoptPlanArgs {
    /// Strict rs.autopilot.plan.v1 JSON returned by an AI planner.
    #[arg(long)]
    plan: std::path::PathBuf,

    /// Directory containing generated/ source files referenced by the plan.
    #[arg(long)]
    source_root: Option<std::path::PathBuf>,

    /// Optional redacted context JSON to copy into the adopted run.
    #[arg(long)]
    context: Option<std::path::PathBuf>,

    /// Run output folder. Defaults to a timestamped .rs/autopilot/runs folder.
    #[arg(long)]
    out: Option<std::path::PathBuf>,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotCertifyArgs {
    /// Autopilot run directory containing plan.json.
    run_dir: std::path::PathBuf,

    /// JSON certification path. Defaults to <run-dir>/certification.json.
    #[arg(long)]
    out: Option<std::path::PathBuf>,

    /// Markdown certification path. Defaults to <run-dir>/certification.md.
    #[arg(long)]
    markdown: Option<std::path::PathBuf>,

    /// Skip writing the Markdown companion file.
    #[arg(long)]
    no_markdown: bool,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotBundleArgs {
    /// Autopilot run directory containing plan.json.
    run_dir: std::path::PathBuf,

    /// Bundle manifest path. Defaults to <run-dir>/bundle.json.
    #[arg(long)]
    out: Option<std::path::PathBuf>,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotVerifyBundleArgs {
    /// Autopilot bundle manifest. Defaults to <run-dir>/bundle.json when --run-dir is used.
    #[arg(long)]
    bundle: Option<std::path::PathBuf>,

    /// Autopilot run directory containing bundle.json.
    #[arg(long)]
    run_dir: Option<std::path::PathBuf>,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotSetupArgs {
    /// Studio name, substring, or UUID. Optional if exactly one compatible Studio is connected.
    #[arg(long)]
    studio: Option<String>,

    /// Build/copy the current plugin bundle before checking readiness.
    #[arg(long)]
    fix: bool,

    /// Seconds to wait before returning a structured blocker report.
    #[arg(long, default_value_t = 10)]
    timeout: u64,

    /// Poll interval in milliseconds.
    #[arg(long, default_value_t = 1000)]
    poll_ms: u64,

    /// Required plugin capabilities. Defaults to the Autopilot live capability set.
    #[arg(long, value_delimiter = ',')]
    require_capability: Vec<String>,

    /// JSON setup path. Defaults to .rs/autopilot/setup.json.
    #[arg(long)]
    out: Option<std::path::PathBuf>,

    /// Markdown setup path. Defaults next to the JSON path.
    #[arg(long)]
    markdown: Option<std::path::PathBuf>,

    /// Skip writing the Markdown companion file.
    #[arg(long)]
    no_markdown: bool,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotReadyArgs {
    /// Studio name, substring, or UUID. Optional if exactly one compatible Studio is connected.
    #[arg(long)]
    studio: Option<String>,

    /// Seconds to wait before returning a structured blocker report.
    #[arg(long, default_value_t = 60)]
    timeout: u64,

    /// Poll interval in milliseconds.
    #[arg(long, default_value_t = 1000)]
    poll_ms: u64,

    /// Required plugin capabilities. Defaults to the Autopilot live capability set.
    #[arg(long, value_delimiter = ',')]
    require_capability: Vec<String>,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotLiveGateArgs {
    /// Autopilot run directory to gate before live apply.
    #[arg(long)]
    run_dir: Option<std::path::PathBuf>,

    /// Session JSON whose runDir should be gated.
    #[arg(long)]
    session: Option<std::path::PathBuf>,

    /// Studio name, substring, or UUID. Defaults to the plan's stored Studio when present.
    #[arg(long)]
    studio: Option<String>,

    /// Assert that the creator approved the exact apply command in approval.json.
    #[arg(long)]
    approved: bool,

    /// Do not contact the live bridge; report that live readiness is still required.
    #[arg(long)]
    skip_ready: bool,

    /// Seconds to wait for live readiness.
    #[arg(long, default_value_t = 60)]
    timeout: u64,

    /// Poll interval in milliseconds.
    #[arg(long, default_value_t = 1000)]
    poll_ms: u64,

    /// Required plugin capabilities. Defaults to the Autopilot live capability set.
    #[arg(long, value_delimiter = ',')]
    require_capability: Vec<String>,

    /// JSON live gate path. Defaults to <run-dir>/live-gate.json.
    #[arg(long)]
    out: Option<std::path::PathBuf>,

    /// Markdown live gate path. Defaults to the JSON path with .md extension.
    #[arg(long)]
    markdown: Option<std::path::PathBuf>,

    /// Skip writing the Markdown companion file.
    #[arg(long)]
    no_markdown: bool,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotRehearsalArgs {
    /// Autopilot run directory to rehearse for creator approval, live proof, and closeout.
    #[arg()]
    run_dir: std::path::PathBuf,

    /// JSON rehearsal runbook path. Defaults to <run-dir>/rehearsal.json.
    #[arg(long)]
    out: Option<std::path::PathBuf>,

    /// Markdown rehearsal runbook path. Defaults to the JSON path with .md extension.
    #[arg(long)]
    markdown: Option<std::path::PathBuf>,

    /// Skip writing the Markdown companion file.
    #[arg(long)]
    no_markdown: bool,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotCloseoutArgs {
    /// Autopilot run directory to evaluate for completion.
    #[arg()]
    run_dir: std::path::PathBuf,

    /// JSON closeout path. Defaults to <run-dir>/closeout.json.
    #[arg(long)]
    out: Option<std::path::PathBuf>,

    /// Markdown closeout path. Defaults to the JSON path with .md extension.
    #[arg(long)]
    markdown: Option<std::path::PathBuf>,

    /// Skip writing the Markdown companion file.
    #[arg(long)]
    no_markdown: bool,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotTimelineArgs {
    /// Autopilot run directory to summarize as a black-box timeline.
    #[arg()]
    run_dir: std::path::PathBuf,

    /// JSON timeline path. Defaults to <run-dir>/timeline.json.
    #[arg(long)]
    out: Option<std::path::PathBuf>,

    /// Markdown timeline path. Defaults to the JSON path with .md extension.
    #[arg(long)]
    markdown: Option<std::path::PathBuf>,

    /// Skip writing the Markdown companion file.
    #[arg(long)]
    no_markdown: bool,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotRunArgs {
    /// Natural-language request. Quote multi-word prompts.
    #[arg()]
    prompt: Vec<String>,

    /// Studio name, substring, or UUID.
    #[arg(long)]
    studio: Option<String>,

    /// Studio path used as the operation root.
    #[arg(long, default_value = "game")]
    scope: String,

    /// Artifact output folder.
    #[arg(long)]
    out: Option<std::path::PathBuf>,

    /// Built-in deterministic recipe, such as starterShop or inventorySystem.
    #[arg(long)]
    recipe: Option<String>,

    /// Structured Autopilot manifest JSON.
    #[arg(long)]
    from_manifest: Option<std::path::PathBuf>,

    /// Approve mutation after reviewing the generated plan artifacts.
    #[arg(long)]
    yes: bool,

    /// Run validation after applying.
    #[arg(long)]
    validate: bool,

    /// Capture rollback artifacts before mutation and attempt safe restore on failure.
    #[arg(long)]
    rollback_on_error: bool,

    /// Permit high-risk operations that target non-owned instances.
    #[arg(long)]
    force: bool,

    /// Apply only operation kinds or groups such as upsertScript,scripts.
    #[arg(long, value_delimiter = ',')]
    only: Vec<String>,

    /// Exclude operation kinds or groups such as scripts,assets,deletes.
    #[arg(long, value_delimiter = ',')]
    exclude: Vec<String>,

    /// Smoke suite to run after apply.
    #[arg(long)]
    smoke: Option<String>,
}

#[derive(Debug, ClapArgs)]
struct AutopilotExplainArgs {
    /// Autopilot plan JSON.
    #[arg(long)]
    plan: std::path::PathBuf,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotPlanArgs {
    /// Natural-language request. Quote multi-word prompts.
    #[arg()]
    prompt: Vec<String>,

    /// Studio name, substring, or UUID. Stored in the plan for later apply.
    #[arg(long)]
    studio: Option<String>,

    /// Studio path used as the operation root.
    #[arg(long, default_value = "game")]
    scope: String,

    /// Artifact output folder.
    #[arg(long)]
    out: Option<std::path::PathBuf>,

    /// Maximum read depth reserved for live context collection.
    #[arg(long, default_value_t = 3)]
    max_read_depth: u32,

    /// Allow script source in future live planner context.
    #[arg(long)]
    include_scripts: bool,

    /// Allow asset metadata in future live planner context.
    #[arg(long)]
    include_assets: bool,

    /// Built-in deterministic recipe, such as starterShop or collectibleCoin.
    #[arg(long)]
    recipe: Option<String>,

    /// Structured Autopilot manifest JSON.
    #[arg(long)]
    from_manifest: Option<std::path::PathBuf>,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotContextArgs {
    /// Studio name, substring, or UUID. Optional if exactly one Studio is connected.
    #[arg(long)]
    studio: Option<String>,

    /// Studio path to inspect.
    #[arg(long, default_value = "game")]
    path: String,

    /// Artifact output folder.
    #[arg(long)]
    out: std::path::PathBuf,

    /// Include every visited path in snapshot output.
    #[arg(long)]
    include_paths: bool,

    /// Include a bounded read tree in context.json.
    #[arg(long)]
    include_read: bool,

    /// Descendant depth for --include-read.
    #[arg(long, default_value_t = 3)]
    read_depth: u32,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotSurveyArgs {
    /// Studio name, substring, or UUID. Optional if exactly one Studio is connected.
    #[arg(long)]
    studio: Option<String>,

    /// Studio path to survey when --context is not provided.
    #[arg(long, default_value = "game")]
    path: String,

    /// Existing autopilot context.json to summarize without contacting Studio.
    #[arg(long)]
    context: Option<std::path::PathBuf>,

    /// JSON survey path. Defaults to .rs/autopilot/survey.json.
    #[arg(long)]
    out: Option<std::path::PathBuf>,

    /// Markdown survey path. Defaults to the JSON path with .md extension.
    #[arg(long)]
    markdown: Option<std::path::PathBuf>,

    /// Skip writing the Markdown companion file.
    #[arg(long)]
    no_markdown: bool,

    /// Include every visited path in the live snapshot.
    #[arg(long)]
    include_paths: bool,

    /// Include a bounded read tree in the live survey source.
    #[arg(long)]
    include_read: bool,

    /// Descendant depth for --include-read.
    #[arg(long, default_value_t = 3)]
    read_depth: u32,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotReconcileArgs {
    /// Autopilot run directory containing plan.json.
    run_dir: std::path::PathBuf,

    /// Existing survey.json to compare against.
    #[arg(long)]
    survey: Option<std::path::PathBuf>,

    /// Existing context.json to compare against.
    #[arg(long)]
    context: Option<std::path::PathBuf>,

    /// JSON reconcile path. Defaults to <run-dir>/reconcile.json.
    #[arg(long)]
    out: Option<std::path::PathBuf>,

    /// Markdown reconcile path. Defaults to <run-dir>/reconcile.md.
    #[arg(long)]
    markdown: Option<std::path::PathBuf>,

    /// Skip writing the Markdown companion file.
    #[arg(long)]
    no_markdown: bool,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotScoutArgs {
    /// Creator request to evaluate against the current place survey.
    #[arg()]
    prompt: Vec<String>,

    /// Existing survey.json to use as place evidence.
    #[arg(long)]
    survey: Option<std::path::PathBuf>,

    /// Existing context.json to summarize into an in-memory survey.
    #[arg(long)]
    context: Option<std::path::PathBuf>,

    /// Root folder for suggested Autopilot runs.
    #[arg(long, default_value = ".rs/autopilot/runs")]
    root: std::path::PathBuf,

    /// Override the selected planning scope. Defaults to the survey path or game.
    #[arg(long)]
    scope: Option<String>,

    /// Limit prior runs when intake checks project continuity.
    #[arg(long, default_value_t = 5)]
    limit: usize,

    /// JSON scout path. Defaults to .rs/autopilot/scout.json.
    #[arg(long)]
    out: Option<std::path::PathBuf>,

    /// Markdown scout path. Defaults to the JSON path with .md extension.
    #[arg(long)]
    markdown: Option<std::path::PathBuf>,

    /// Skip writing the Markdown companion file.
    #[arg(long)]
    no_markdown: bool,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotSessionArgs {
    /// Creator request to scout and bootstrap when --scout is not provided.
    #[arg()]
    prompt: Vec<String>,

    /// Existing scout.json to turn into a full offline session.
    #[arg(long)]
    scout: Option<std::path::PathBuf>,

    /// Existing survey.json to scout before bootstrapping.
    #[arg(long)]
    survey: Option<std::path::PathBuf>,

    /// Existing context.json to summarize and scout before bootstrapping.
    #[arg(long)]
    context: Option<std::path::PathBuf>,

    /// Root folder for generated Autopilot runs.
    #[arg(long, default_value = ".rs/autopilot/runs")]
    root: std::path::PathBuf,

    /// Explicit run output folder for the bootstrapped start packet.
    #[arg(long)]
    run_dir: Option<std::path::PathBuf>,

    /// Override the selected planning scope.
    #[arg(long)]
    scope: Option<String>,

    /// Limit prior runs when intake checks project continuity.
    #[arg(long, default_value_t = 5)]
    limit: usize,

    /// Proceed through intake clarification with explicit assumptions.
    #[arg(long)]
    assume: bool,

    /// Optional smoke suite to append to the bootstrapped plan.
    #[arg(long)]
    smoke: Option<String>,

    /// JSON session path. Defaults to .rs/autopilot/session.json.
    #[arg(long)]
    out: Option<std::path::PathBuf>,

    /// Markdown session path. Defaults to the JSON path with .md extension.
    #[arg(long)]
    markdown: Option<std::path::PathBuf>,

    /// Skip writing Markdown companions.
    #[arg(long)]
    no_markdown: bool,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotPreviewArgs {
    /// Studio name, substring, or UUID. Overrides the plan's stored Studio in preview output.
    #[arg(long)]
    studio: Option<String>,

    /// Autopilot plan JSON.
    #[arg(long)]
    plan: std::path::PathBuf,

    /// Artifact output folder. Defaults to the plan's folder.
    #[arg(long)]
    out: Option<std::path::PathBuf>,

    /// Ask Studio to check preconditions and dry-run supported operations.
    #[arg(long)]
    live: bool,

    /// Permit high-risk dry-run checks against non-owned instances.
    #[arg(long)]
    force: bool,

    /// Preview only operation kinds or groups such as upsertScript,scripts.
    #[arg(long, value_delimiter = ',')]
    only: Vec<String>,

    /// Exclude operation kinds or groups such as scripts,assets,deletes.
    #[arg(long, value_delimiter = ',')]
    exclude: Vec<String>,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotApplyArgs {
    /// Studio name, substring, or UUID. Overrides the plan's stored Studio.
    #[arg(long)]
    studio: Option<String>,

    /// Autopilot plan JSON.
    #[arg(long)]
    plan: std::path::PathBuf,

    /// Artifact output folder. Defaults to the plan's folder.
    #[arg(long)]
    out: Option<std::path::PathBuf>,

    /// Approve mutation after reviewing the plan.
    #[arg(long)]
    yes: bool,

    /// Run validation after applying.
    #[arg(long)]
    validate: bool,

    /// Capture rollback artifacts before mutation.
    #[arg(long)]
    rollback_on_error: bool,

    /// Permit high-risk operations that target non-owned instances.
    #[arg(long)]
    force: bool,

    /// Apply only operation kinds or groups such as upsertScript,scripts.
    #[arg(long, value_delimiter = ',')]
    only: Vec<String>,

    /// Exclude operation kinds or groups such as scripts,assets,deletes.
    #[arg(long, value_delimiter = ',')]
    exclude: Vec<String>,

    /// Smoke suite to record as required follow-up.
    #[arg(long)]
    smoke: Option<String>,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct AutopilotReportArgs {
    /// Autopilot run directory containing report.md.
    run_dir: std::path::PathBuf,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, Subcommand)]
enum SyncCommand {
    /// Pull Studio changes back to disk as scripts, metadata JSON, asset refs, and transfer blob.
    Pull {
        /// Studio name, substring, or UUID. Optional if exactly one Studio is connected.
        #[arg(long)]
        studio: Option<String>,

        /// Dot path such as Workspace.Tool.
        #[arg(long)]
        path: String,

        /// Output directory.
        #[arg(long)]
        out: std::path::PathBuf,

        /// Optional descendant depth for the file tree export.
        #[arg(long)]
        depth: Option<u32>,

        /// Overwrite existing files.
        #[arg(long)]
        overwrite: bool,

        /// Output format.
        #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
        format: OutputFormat,
    },
}

#[derive(Debug, Subcommand)]
enum TransactionCommand {
    /// Snapshot a Studio subtree into a transfer blob on disk.
    Snapshot {
        /// Studio name, substring, or UUID. Optional if exactly one Studio is connected.
        #[arg(long)]
        studio: Option<String>,

        /// Dot path to snapshot.
        #[arg(long)]
        path: String,

        /// Output JSON file.
        #[arg(long)]
        out: std::path::PathBuf,

        /// Output format.
        #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
        format: OutputFormat,
    },

    /// Restore a transaction snapshot into Studio.
    Restore {
        /// Studio name, substring, or UUID. Optional if exactly one Studio is connected.
        #[arg(long)]
        studio: Option<String>,

        /// Snapshot JSON file.
        #[arg(long)]
        file: std::path::PathBuf,

        /// Destination parent path.
        #[arg(long)]
        to: String,

        /// What to do when an instance with the snapshot root name already exists.
        #[arg(long, value_enum, default_value_t = IfExists::Replace)]
        if_exists: IfExists,

        /// Output format.
        #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
        format: OutputFormat,
    },
}

#[derive(Debug, Subcommand)]
enum HistoryCommand {
    /// Show one history record.
    Show {
        /// Command ID from rs history.
        id: String,
    },
}

#[derive(Debug, ClapArgs)]
struct RepairToolArgs {
    /// Studio name, substring, or UUID. Optional if exactly one Studio is connected.
    #[arg(long)]
    studio: Option<String>,

    /// Dot path of the Tool to repair.
    #[arg(long)]
    path: String,

    /// Handle child name. Defaults to Handle.
    #[arg(long)]
    handle: Option<String>,

    /// Report intended changes without mutating Studio.
    #[arg(long)]
    dry_run: bool,

    /// Delete broken joints instead of only reporting them.
    #[arg(long)]
    replace_broken: bool,

    /// Do not change Anchored, CanCollide, or Massless.
    #[arg(long)]
    no_physics_fix: bool,

    /// Collision setting for non-handle parts.
    #[arg(long, value_enum)]
    collision: Option<OnOff>,

    /// Massless setting for non-handle parts.
    #[arg(long, value_enum)]
    massless: Option<OnOff>,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, Subcommand)]
enum PackageCommand {
    /// Inspect a package manifest.
    Inspect {
        /// Package folder.
        file: std::path::PathBuf,

        /// Output format.
        #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
        format: OutputFormat,
    },

    /// Import a package into Studio.
    Import {
        /// Studio name, substring, or UUID. Optional if exactly one Studio is connected.
        #[arg(long)]
        studio: Option<String>,

        /// Package folder.
        #[arg(long)]
        file: std::path::PathBuf,

        /// Destination parent path.
        #[arg(long, default_value = "Workspace")]
        to: String,

        /// What to do when an instance with the package root name already exists.
        #[arg(long, value_enum, default_value_t = IfExists::Fail)]
        if_exists: IfExists,

        /// Report the import plan without mutating Studio.
        #[arg(long)]
        dry_run: bool,

        /// Restore replaced content when package deserialize fails.
        #[arg(long)]
        rollback_on_error: bool,

        #[command(flatten)]
        image_rehost: ImageRehostArgs,

        /// Output format.
        #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
        format: OutputFormat,
    },

    /// Reapply a package to an existing install using stable ownership IDs.
    Update {
        /// Studio name, substring, or UUID. Optional if exactly one Studio is connected.
        #[arg(long)]
        studio: Option<String>,

        /// Package folder.
        #[arg(long)]
        file: std::path::PathBuf,

        /// Destination parent path.
        #[arg(long, default_value = "Workspace")]
        to: String,

        /// Update only instances owned by this package/rs.
        #[arg(long)]
        owned_only: bool,

        /// Preserve local/manual instances when conflicts are found.
        #[arg(long)]
        preserve_local: bool,

        /// Replace matching owned instances with package content.
        #[arg(long)]
        replace_owned: bool,

        /// Report conflicts without mutating Studio.
        #[arg(long)]
        conflict_report: bool,

        /// Report the update plan without mutating Studio.
        #[arg(long)]
        dry_run: bool,

        /// Permit overwriting user-owned/manual instances.
        #[arg(long)]
        force: bool,

        /// Output format.
        #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
        format: OutputFormat,
    },

    /// Verify manifest, checksums, transfer blob, asset references, and optional conflicts.
    Verify {
        /// Package folder.
        #[arg(long)]
        file: std::path::PathBuf,

        /// Studio name, substring, or UUID for optional dry-run conflict report.
        #[arg(long)]
        studio: Option<String>,

        /// Destination parent path for optional dry-run conflict report.
        #[arg(long)]
        to: Option<String>,

        /// Conflict mode to plan when --studio is supplied.
        #[arg(long, value_enum, default_value_t = IfExists::Fail)]
        if_exists: IfExists,

        /// Output format.
        #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
        format: OutputFormat,
    },

    /// Pack a package folder into a portable zip archive.
    Pack {
        /// Package folder.
        file: std::path::PathBuf,

        /// Output .zip path.
        #[arg(long)]
        out: std::path::PathBuf,
    },

    /// Unpack a package zip archive into a folder.
    Unpack {
        /// Package archive.
        file: std::path::PathBuf,

        /// Output folder.
        #[arg(long)]
        out: std::path::PathBuf,

        /// Overwrite existing output folder contents.
        #[arg(long)]
        overwrite: bool,
    },
}

#[derive(Debug, Subcommand)]
enum SmokeCommand {
    /// Create a broken Tool and verify validate reports it.
    Validate {
        #[arg(long)]
        studio: Option<String>,
    },

    /// Create a loose Tool, repair it, and verify validation passes.
    RepairTool {
        #[arg(long)]
        studio: Option<String>,
    },

    /// Import a one-pixel UI pack and verify it exists.
    ImportUiPack {
        #[arg(long)]
        studio: Option<String>,
    },

    /// Run all smoke checks.
    All {
        #[arg(long)]
        studio: Option<String>,
    },

    /// Run the broader regression suite and save a JSON report.
    Regression {
        #[arg(long)]
        studio: Option<String>,

        /// Report path to write.
        #[arg(long)]
        out: Option<std::path::PathBuf>,

        /// Use mocked upload parsing instead of real Open Cloud network calls.
        #[arg(long)]
        upload_mock: bool,
    },
}

#[derive(Debug, Subcommand)]
enum UploadCommand {
    /// Upload an image asset.
    Image(UploadAssetArgs),

    /// Upload an audio asset.
    Audio(UploadAssetArgs),

    /// Upload a Roblox-supported model container.
    Model(UploadAssetArgs),

    /// Alias for model uploads. Raw OBJ/STL should use import-asset instead.
    Mesh(UploadAssetArgs),
}

#[derive(Debug, ClapArgs)]
struct UploadAssetArgs {
    /// Local file to upload.
    file: std::path::PathBuf,

    /// Open Cloud creator ID.
    #[arg(long)]
    creator_id: Option<u64>,

    /// Whether --creator-id is a group or user ID.
    #[arg(long, value_enum)]
    creator_type: Option<CreatorType>,

    /// Open Cloud profile to use.
    #[arg(long)]
    profile: Option<String>,

    /// Display name for the asset. Defaults to the file stem.
    #[arg(long)]
    name: Option<String>,

    /// Asset description.
    #[arg(long)]
    description: Option<String>,

    /// Open Cloud API key. Defaults to ROBLOX_API_KEY.
    #[arg(long, env = "ROBLOX_API_KEY")]
    api_key: Option<String>,

    /// Poll the Open Cloud operation until a final asset ID is available.
    #[arg(long)]
    wait: bool,

    /// Maximum seconds to wait for Open Cloud processing.
    #[arg(long, default_value_t = 300)]
    wait_timeout: u64,

    /// After upload completion, import the resulting image/audio asset into this Studio path.
    #[arg(long)]
    import_to: Option<String>,

    /// Studio name, substring, or UUID used with --import-to.
    #[arg(long)]
    studio: Option<String>,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, Clone, ClapArgs)]
struct ImageRehostArgs {
    /// Download referenced image assets and upload target-owned copies before import/transfer.
    #[arg(long)]
    rehost_images: bool,

    /// Target Open Cloud creator ID for rehosted image uploads.
    #[arg(long)]
    creator_id: Option<u64>,

    /// Whether --creator-id is a group or user ID.
    #[arg(long, value_enum)]
    creator_type: Option<CreatorType>,

    /// Target Open Cloud profile to use.
    #[arg(long)]
    profile: Option<String>,

    /// Target Open Cloud API key. Defaults to ROBLOX_API_KEY or the selected profile.
    #[arg(long, env = "ROBLOX_API_KEY")]
    api_key: Option<String>,

    /// Source Open Cloud API key for reading image content. Defaults to --api-key/profile key.
    #[arg(long)]
    source_api_key: Option<String>,

    /// Maximum seconds to wait for each rehosted image upload.
    #[arg(long, default_value_t = 300)]
    rehost_timeout: u64,
}

#[derive(Debug, Subcommand)]
enum ImportUploadedCommand {
    /// Import an uploaded image asset as Studio UI.
    Image(ImportUploadedImageArgs),

    /// Import an uploaded audio asset as a Sound.
    Audio(ImportUploadedAudioArgs),
}

#[derive(Debug, ClapArgs)]
struct ImportUploadedImageArgs {
    /// Roblox asset ID or rbxassetid:// URI.
    asset_id: String,

    #[arg(long)]
    studio: Option<String>,

    #[arg(long, default_value = "StarterGui")]
    to: String,

    #[arg(long)]
    name: Option<String>,

    #[arg(long, value_enum, default_value_t = ImageKind::Image)]
    kind: ImageKind,

    #[arg(long)]
    size: Option<String>,

    #[arg(long, default_value = "0,0")]
    position: String,

    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, ClapArgs)]
struct ImportUploadedAudioArgs {
    /// Roblox asset ID or rbxassetid:// URI.
    asset_id: String,

    #[arg(long)]
    studio: Option<String>,

    #[arg(long, default_value = "SoundService")]
    to: String,

    #[arg(long)]
    name: Option<String>,

    #[arg(long)]
    volume: Option<f32>,

    #[arg(long)]
    playback_speed: Option<f32>,

    #[arg(long)]
    looped: bool,

    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, Subcommand)]
enum AuthCommand {
    /// Manage Open Cloud profiles.
    Profile {
        #[command(subcommand)]
        command: AuthProfileCommand,
    },
}

#[derive(Debug, Subcommand)]
enum AuthProfileCommand {
    /// Add or update an Open Cloud profile.
    Add {
        name: String,

        #[arg(long)]
        creator_id: u64,

        #[arg(long, value_enum, default_value_t = CreatorType::Group)]
        creator_type: CreatorType,

        #[arg(long, env = "ROBLOX_API_KEY")]
        api_key: Option<String>,

        #[arg(long)]
        set_default: bool,
    },

    /// List configured profiles without printing API keys.
    List {
        #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
        format: OutputFormat,
    },

    /// Diagnose local profile credential storage.
    Doctor {
        #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
        format: OutputFormat,
    },

    /// Remove a profile.
    Remove { name: String },

    /// Set the default profile.
    Default { name: String },
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

#[derive(Debug, Clone, ValueEnum)]
enum OutputFormat {
    Text,
    Json,
}

#[derive(Debug, Clone, ValueEnum)]
enum OnOff {
    On,
    Off,
}

#[derive(Debug, Clone, ValueEnum)]
enum IfExists {
    Fail,
    Replace,
    Merge,
    Rename,
}

#[derive(Debug, Clone, ValueEnum)]
enum CreatorType {
    Group,
    User,
}

#[derive(Debug, Clone, ValueEnum)]
enum PlanChangeKind {
    Added,
    Modified,
    Deleted,
    Reference,
}

fn main() {
    if let Err(err) = run() {
        if !err.is_silent() {
            eprintln!("{err}");
        }
        let _ = std::io::stdout().flush();
        let _ = std::io::stderr().flush();
        std::process::exit(err.exit_code());
    }
}

fn run() -> AppResult<()> {
    let args = Args::parse();

    match args.command {
        Command::List { json } => cli::list::run(args.port, json),
        Command::Doctor { fix, json, format } => cli::doctor::run(
            args.port,
            fix,
            json || format.is_some_and(|value| value.is_json()),
        ),
        Command::InstallPlugin { watch, json } => cli::install_plugin::run(args.port, watch, json),
        Command::Exec {
            studio,
            lua,
            allow_dangerous_exec,
        } => cli::exec::run(args.port, studio, lua, allow_dangerous_exec),
        Command::Read {
            studio,
            path,
            depth,
        } => cli::read::run(args.port, studio, path, depth),
        Command::Transfer {
            from,
            to,
            dry_run,
            replace,
            rollback_on_error,
            allow_external_refs,
            image_rehost,
        } => cli::transfer::run(
            args.port,
            from,
            to,
            dry_run,
            replace,
            rollback_on_error,
            allow_external_refs,
            image_rehost_options(image_rehost, dry_run, false),
        ),
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
            texture_root,
        } => cli::import_asset::run(
            args.port,
            studio,
            file,
            to,
            name,
            scale,
            anchored,
            !no_weld,
            texture_root,
        ),
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
        Command::ImportUiPack {
            studio,
            folder,
            manifest,
            to,
            name,
            kind,
            format,
        } => cli::import_ui_pack::run(
            args.port,
            studio,
            folder,
            manifest,
            to,
            name,
            kind.as_str().to_string(),
            format.is_json(),
        ),
        Command::ImportAudio {
            studio,
            file,
            manifest,
            to,
            name,
            asset_id,
            volume,
            playback_speed,
            looped,
            format,
        } => cli::import_audio::run(
            args.port,
            studio,
            file,
            manifest,
            to,
            name,
            asset_id,
            volume,
            playback_speed,
            looped,
            format.is_json(),
        ),
        Command::Upload { command } => match command {
            UploadCommand::Image(upload_args) => {
                run_upload(args.port, cli::upload::UploadKind::Image, upload_args)
            }
            UploadCommand::Audio(upload_args) => {
                run_upload(args.port, cli::upload::UploadKind::Audio, upload_args)
            }
            UploadCommand::Model(upload_args) => {
                run_upload(args.port, cli::upload::UploadKind::Model, upload_args)
            }
            UploadCommand::Mesh(upload_args) => {
                run_upload(args.port, cli::upload::UploadKind::Mesh, upload_args)
            }
        },
        Command::ImportUploaded { command } => match command {
            ImportUploadedCommand::Image(image_args) => cli::import_uploaded::run(
                args.port,
                image_args.studio,
                "image".into(),
                image_args.asset_id,
                Some(image_args.to),
                image_args.name,
                Some(image_args.kind.as_str().to_string()),
                image_args.size,
                Some(image_args.position),
                None,
                None,
                false,
                image_args.format.is_json(),
            ),
            ImportUploadedCommand::Audio(audio_args) => cli::import_uploaded::run(
                args.port,
                audio_args.studio,
                "audio".into(),
                audio_args.asset_id,
                Some(audio_args.to),
                audio_args.name,
                None,
                None,
                None,
                audio_args.volume,
                audio_args.playback_speed,
                audio_args.looped,
                audio_args.format.is_json(),
            ),
        },
        Command::Auth { command } => match command {
            AuthCommand::Profile { command } => match command {
                AuthProfileCommand::Add {
                    name,
                    creator_id,
                    creator_type,
                    api_key,
                    set_default,
                } => cli::auth::profile_add(
                    name,
                    creator_id,
                    creator_type.as_str().to_string(),
                    api_key,
                    set_default,
                ),
                AuthProfileCommand::List { format } => cli::auth::profile_list(format.is_json()),
                AuthProfileCommand::Doctor { format } => {
                    cli::auth::profile_doctor(format.is_json())
                }
                AuthProfileCommand::Remove { name } => cli::auth::profile_remove(name),
                AuthProfileCommand::Default { name } => cli::auth::profile_default(name),
            },
        },
        Command::Validate {
            studio,
            path,
            rules,
            format,
            fix,
        } => cli::validate::run(args.port, studio, path, rules, format.is_json(), fix),
        Command::RepairTool(repair_args) | Command::WireTool(repair_args) => cli::repair_tool::run(
            args.port,
            repair_args.studio,
            repair_args.path,
            repair_args.handle,
            repair_args.dry_run,
            repair_args.replace_broken,
            repair_args.no_physics_fix,
            repair_args.collision.map(|value| value.as_bool()),
            repair_args.massless.map(|value| value.as_bool()),
            repair_args.format.is_json(),
        ),
        Command::Snapshot {
            studio,
            path,
            include_paths,
            out,
            format,
        } => cli::snapshot::run(
            args.port,
            studio,
            path,
            include_paths,
            format.is_json(),
            out,
        ),
        Command::Smoke { command } => match command {
            SmokeCommand::Validate { studio } => cli::smoke::run(args.port, studio, "validate"),
            SmokeCommand::RepairTool { studio } => {
                cli::smoke::run(args.port, studio, "repair-tool")
            }
            SmokeCommand::ImportUiPack { studio } => {
                cli::smoke::run(args.port, studio, "import-ui-pack")
            }
            SmokeCommand::All { studio } => cli::smoke::run(args.port, studio, "all"),
            SmokeCommand::Regression {
                studio,
                out,
                upload_mock,
            } => cli::smoke::regression(args.port, studio, out, upload_mock),
        },
        Command::Create {
            studio,
            class_name,
            to,
            name,
            properties,
            json,
            format,
        } => cli::create::run(
            args.port,
            studio,
            class_name,
            to,
            name,
            properties,
            json,
            format.is_json(),
        ),
        Command::Diff {
            studio,
            path,
            export_path,
            against_studio,
            against_path,
            against_export,
            depth,
            ignore_scripts,
            ignore_assets,
            fix_plan,
            format,
        } => cli::diff::run(
            args.port,
            studio,
            path,
            export_path,
            against_studio,
            against_path,
            against_export,
            depth,
            format.is_json(),
            ignore_scripts,
            ignore_assets,
            fix_plan,
        ),
        Command::ApplyPlan {
            studio,
            root,
            file,
            dry_run,
            yes,
            only,
            exclude,
            force,
            format,
        } => cli::apply_plan::run(
            args.port,
            studio,
            root,
            file,
            dry_run,
            yes,
            only.into_iter()
                .map(|value| value.as_str().to_string())
                .collect(),
            exclude,
            force,
            format.is_json(),
        ),
        Command::Autopilot { command } => match command {
            AutopilotCommand::Recipes(recipes_args) => {
                cli::autopilot::recipes(cli::autopilot::RecipesOptions {
                    json: recipes_args.format.is_json(),
                })
            }
            AutopilotCommand::Capabilities(capabilities_args) => {
                cli::autopilot::capabilities(cli::autopilot::CapabilitiesOptions {
                    root: capabilities_args.root,
                    limit: capabilities_args.limit,
                    out: capabilities_args.out,
                    markdown: capabilities_args.markdown,
                    no_markdown: capabilities_args.no_markdown,
                    json: capabilities_args.format.is_json(),
                })
            }
            AutopilotCommand::Compose(compose_args) => {
                cli::autopilot::compose(cli::autopilot::ComposeOptions {
                    port: args.port,
                    prompt: if compose_args.prompt.is_empty() {
                        None
                    } else {
                        Some(compose_args.prompt.join(" "))
                    },
                    studio: compose_args.studio,
                    scope: compose_args.scope,
                    out: compose_args.out,
                    preset: compose_args.preset,
                    recipes: compose_args.recipe,
                    from_manifest: compose_args.from_manifest,
                    smoke: compose_args.smoke,
                    json: compose_args.format.is_json(),
                })
            }
            AutopilotCommand::Tune(tune_args) => {
                cli::autopilot::tune(cli::autopilot::TuneOptions {
                    prompt: if tune_args.prompt.is_empty() {
                        None
                    } else {
                        Some(tune_args.prompt.join(" "))
                    },
                    preset: tune_args.preset,
                    recipes: tune_args.recipe,
                    smoke: tune_args.smoke,
                    out: tune_args.out,
                    markdown: tune_args.markdown,
                    no_markdown: tune_args.no_markdown,
                    json: tune_args.format.is_json(),
                })
            }
            AutopilotCommand::Coach(coach_args) => {
                cli::autopilot::coach(cli::autopilot::CoachOptions {
                    run_dir: coach_args.run_dir,
                    plan: coach_args.plan,
                    json: coach_args.format.is_json(),
                })
            }
            AutopilotCommand::Handoff(handoff_args) => {
                cli::autopilot::handoff(cli::autopilot::HandoffOptions {
                    run_dir: handoff_args.run_dir,
                    out: handoff_args.out,
                    markdown: handoff_args.markdown,
                    no_markdown: handoff_args.no_markdown,
                    json: handoff_args.format.is_json(),
                })
            }
            AutopilotCommand::Runs(runs_args) => {
                cli::autopilot::runs(cli::autopilot::RunsOptions {
                    root: runs_args.root,
                    limit: runs_args.limit,
                    json: runs_args.format.is_json(),
                })
            }
            AutopilotCommand::Mission(mission_args) => {
                cli::autopilot::mission(cli::autopilot::MissionOptions {
                    prompt: if mission_args.prompt.is_empty() {
                        None
                    } else {
                        Some(mission_args.prompt.join(" "))
                    },
                    root: mission_args.root,
                    limit: mission_args.limit,
                    out: mission_args.out,
                    markdown: mission_args.markdown,
                    no_markdown: mission_args.no_markdown,
                    json: mission_args.format.is_json(),
                })
            }
            AutopilotCommand::Memory(memory_args) => {
                cli::autopilot::memory(cli::autopilot::MemoryOptions {
                    root: memory_args.root,
                    limit: memory_args.limit,
                    out: memory_args.out,
                    markdown: memory_args.markdown,
                    no_markdown: memory_args.no_markdown,
                    json: memory_args.format.is_json(),
                })
            }
            AutopilotCommand::Preferences(preferences_args) => {
                cli::autopilot::preferences(cli::autopilot::PreferencesOptions {
                    root: preferences_args.root,
                    limit: preferences_args.limit,
                    out: preferences_args.out,
                    markdown: preferences_args.markdown,
                    no_markdown: preferences_args.no_markdown,
                    json: preferences_args.format.is_json(),
                })
            }
            AutopilotCommand::GameBible(game_bible_args) => {
                cli::autopilot::game_bible(cli::autopilot::GameBibleOptions {
                    root: game_bible_args.root,
                    limit: game_bible_args.limit,
                    out: game_bible_args.out,
                    markdown: game_bible_args.markdown,
                    no_markdown: game_bible_args.no_markdown,
                    json: game_bible_args.format.is_json(),
                })
            }
            AutopilotCommand::Playbook(playbook_args) => {
                cli::autopilot::playbook(cli::autopilot::PlaybookOptions {
                    root: playbook_args.root,
                    limit: playbook_args.limit,
                    out: playbook_args.out,
                    markdown: playbook_args.markdown,
                    no_markdown: playbook_args.no_markdown,
                    json: playbook_args.format.is_json(),
                })
            }
            AutopilotCommand::Director(director_args) => {
                cli::autopilot::director(cli::autopilot::DirectorOptions {
                    prompt: if director_args.prompt.is_empty() {
                        None
                    } else {
                        Some(director_args.prompt.join(" "))
                    },
                    root: director_args.root,
                    limit: director_args.limit,
                    out: director_args.out,
                    markdown: director_args.markdown,
                    no_markdown: director_args.no_markdown,
                    json: director_args.format.is_json(),
                })
            }
            AutopilotCommand::Pursue(pursue_args) => {
                cli::autopilot::pursue(cli::autopilot::PursueOptions {
                    prompt: if pursue_args.prompt.is_empty() {
                        None
                    } else {
                        Some(pursue_args.prompt.join(" "))
                    },
                    bet: pursue_args.bet,
                    root: pursue_args.root,
                    limit: pursue_args.limit,
                    out: pursue_args.out,
                    markdown: pursue_args.markdown,
                    no_markdown: pursue_args.no_markdown,
                    dry_run: pursue_args.dry_run,
                    json: pursue_args.format.is_json(),
                })
            }
            AutopilotCommand::Agenda(agenda_args) => {
                cli::autopilot::agenda(cli::autopilot::AgendaOptions {
                    prompt: if agenda_args.prompt.is_empty() {
                        None
                    } else {
                        Some(agenda_args.prompt.join(" "))
                    },
                    run_dir: agenda_args.run_dir,
                    root: agenda_args.root,
                    limit: agenda_args.limit,
                    out: agenda_args.out,
                    markdown: agenda_args.markdown,
                    no_markdown: agenda_args.no_markdown,
                    json: agenda_args.format.is_json(),
                })
            }
            AutopilotCommand::Sprint(sprint_args) => {
                cli::autopilot::sprint(cli::autopilot::SprintOptions {
                    prompt: if sprint_args.prompt.is_empty() {
                        None
                    } else {
                        Some(sprint_args.prompt.join(" "))
                    },
                    run_dir: sprint_args.run_dir,
                    root: sprint_args.root,
                    limit: sprint_args.limit,
                    max_steps: sprint_args.max_steps,
                    dry_run: sprint_args.dry_run,
                    out: sprint_args.out,
                    markdown: sprint_args.markdown,
                    no_markdown: sprint_args.no_markdown,
                    json: sprint_args.format.is_json(),
                })
            }
            AutopilotCommand::Retrospect(retrospect_args) => {
                cli::autopilot::retrospect(cli::autopilot::RetrospectOptions {
                    run_dir: retrospect_args.run_dir,
                    root: retrospect_args.root,
                    limit: retrospect_args.limit,
                    out: retrospect_args.out,
                    markdown: retrospect_args.markdown,
                    no_markdown: retrospect_args.no_markdown,
                    json: retrospect_args.format.is_json(),
                })
            }
            AutopilotCommand::Control(control_args) => {
                cli::autopilot::control(cli::autopilot::ControlOptions {
                    prompt: if control_args.prompt.is_empty() {
                        None
                    } else {
                        Some(control_args.prompt.join(" "))
                    },
                    run_dir: control_args.run_dir,
                    root: control_args.root,
                    limit: control_args.limit,
                    out: control_args.out,
                    markdown: control_args.markdown,
                    no_markdown: control_args.no_markdown,
                    json: control_args.format.is_json(),
                })
            }
            AutopilotCommand::Brief(brief_args) => {
                cli::autopilot::brief(cli::autopilot::BriefOptions {
                    prompt: if brief_args.prompt.is_empty() {
                        None
                    } else {
                        Some(brief_args.prompt.join(" "))
                    },
                    run_dir: brief_args.run_dir,
                    root: brief_args.root,
                    limit: brief_args.limit,
                    out: brief_args.out,
                    markdown: brief_args.markdown,
                    no_markdown: brief_args.no_markdown,
                    json: brief_args.format.is_json(),
                })
            }
            AutopilotCommand::Inbox(inbox_args) => {
                cli::autopilot::inbox(cli::autopilot::InboxOptions {
                    message: if inbox_args.message.is_empty() {
                        None
                    } else {
                        Some(inbox_args.message.join(" "))
                    },
                    run_dir: inbox_args.run_dir,
                    root: inbox_args.root,
                    limit: inbox_args.limit,
                    source: inbox_args.source,
                    out: inbox_args.out,
                    markdown: inbox_args.markdown,
                    no_markdown: inbox_args.no_markdown,
                    json: inbox_args.format.is_json(),
                })
            }
            AutopilotCommand::Handle(handle_args) => {
                cli::autopilot::handle(cli::autopilot::HandleOptions {
                    port: args.port,
                    message: if handle_args.message.is_empty() {
                        None
                    } else {
                        Some(handle_args.message.join(" "))
                    },
                    run_dir: handle_args.run_dir,
                    studio: handle_args.studio,
                    scope: handle_args.scope,
                    root: handle_args.root,
                    limit: handle_args.limit,
                    source: handle_args.source,
                    assume: handle_args.assume,
                    smoke: handle_args.smoke,
                    dry_run: handle_args.dry_run,
                    out: handle_args.out,
                    markdown: handle_args.markdown,
                    no_markdown: handle_args.no_markdown,
                    json: handle_args.format.is_json(),
                })
            }
            AutopilotCommand::Conversation(conversation_args) => {
                cli::autopilot::conversation(cli::autopilot::ConversationOptions {
                    message: if conversation_args.message.is_empty() {
                        None
                    } else {
                        Some(conversation_args.message.join(" "))
                    },
                    run_dir: conversation_args.run_dir,
                    root: conversation_args.root,
                    limit: conversation_args.limit,
                    source: conversation_args.source,
                    out: conversation_args.out,
                    markdown: conversation_args.markdown,
                    no_markdown: conversation_args.no_markdown,
                    json: conversation_args.format.is_json(),
                })
            }
            AutopilotCommand::Chat(chat_args) => {
                cli::autopilot::chat(cli::autopilot::ChatOptions {
                    port: args.port,
                    message: if chat_args.message.is_empty() {
                        None
                    } else {
                        Some(chat_args.message.join(" "))
                    },
                    run_dir: chat_args.run_dir,
                    studio: chat_args.studio,
                    scope: chat_args.scope,
                    root: chat_args.root,
                    limit: chat_args.limit,
                    source: chat_args.source,
                    assume: chat_args.assume,
                    smoke: chat_args.smoke,
                    max_steps: chat_args.max_steps,
                    dry_run: chat_args.dry_run,
                    out: chat_args.out,
                    markdown: chat_args.markdown,
                    no_markdown: chat_args.no_markdown,
                    json: chat_args.format.is_json(),
                })
            }
            AutopilotCommand::Intake(intake_args) => {
                cli::autopilot::intake(cli::autopilot::IntakeOptions {
                    prompt: if intake_args.prompt.is_empty() {
                        None
                    } else {
                        Some(intake_args.prompt.join(" "))
                    },
                    root: intake_args.root,
                    limit: intake_args.limit,
                    out: intake_args.out,
                    markdown: intake_args.markdown,
                    no_markdown: intake_args.no_markdown,
                    json: intake_args.format.is_json(),
                })
            }
            AutopilotCommand::Start(start_args) => {
                cli::autopilot::start(cli::autopilot::StartOptions {
                    port: args.port,
                    prompt: if start_args.prompt.is_empty() {
                        None
                    } else {
                        Some(start_args.prompt.join(" "))
                    },
                    studio: start_args.studio,
                    scope: start_args.scope,
                    root: start_args.root,
                    run_dir: start_args.run_dir,
                    limit: start_args.limit,
                    assume: start_args.assume,
                    smoke: start_args.smoke,
                    out: start_args.out,
                    markdown: start_args.markdown,
                    no_markdown: start_args.no_markdown,
                    json: start_args.format.is_json(),
                })
            }
            AutopilotCommand::Pitch(pitch_args) => {
                cli::autopilot::pitch(cli::autopilot::PitchOptions {
                    prompt: if pitch_args.prompt.is_empty() {
                        None
                    } else {
                        Some(pitch_args.prompt.join(" "))
                    },
                    studio: pitch_args.studio,
                    scope: pitch_args.scope,
                    root: pitch_args.root,
                    max_candidates: pitch_args.max_candidates,
                    out: pitch_args.out,
                    markdown: pitch_args.markdown,
                    no_markdown: pitch_args.no_markdown,
                    json: pitch_args.format.is_json(),
                })
            }
            AutopilotCommand::Storyboard(storyboard_args) => {
                cli::autopilot::storyboard(cli::autopilot::StoryboardOptions {
                    prompt: if storyboard_args.prompt.is_empty() {
                        None
                    } else {
                        Some(storyboard_args.prompt.join(" "))
                    },
                    run_dir: storyboard_args.run_dir,
                    root: storyboard_args.root,
                    out: storyboard_args.out,
                    markdown: storyboard_args.markdown,
                    no_markdown: storyboard_args.no_markdown,
                    json: storyboard_args.format.is_json(),
                })
            }
            AutopilotCommand::Proposal(proposal_args) => {
                cli::autopilot::proposal(cli::autopilot::ProposalOptions {
                    prompt: if proposal_args.prompt.is_empty() {
                        None
                    } else {
                        Some(proposal_args.prompt.join(" "))
                    },
                    studio: proposal_args.studio,
                    scope: proposal_args.scope,
                    root: proposal_args.root,
                    max_candidates: proposal_args.max_candidates,
                    out: proposal_args.out,
                    markdown: proposal_args.markdown,
                    no_markdown: proposal_args.no_markdown,
                    json: proposal_args.format.is_json(),
                })
            }
            AutopilotCommand::Companion(companion_args) => {
                cli::autopilot::companion(cli::autopilot::CompanionOptions {
                    port: args.port,
                    prompt: if companion_args.prompt.is_empty() {
                        None
                    } else {
                        Some(companion_args.prompt.join(" "))
                    },
                    studio: companion_args.studio,
                    scope: companion_args.scope,
                    root: companion_args.root,
                    max_candidates: companion_args.max_candidates,
                    fix: companion_args.fix,
                    timeout_secs: companion_args.timeout,
                    poll_ms: companion_args.poll_ms,
                    out: companion_args.out,
                    markdown: companion_args.markdown,
                    no_markdown: companion_args.no_markdown,
                    json: companion_args.format.is_json(),
                })
            }
            AutopilotCommand::Select(select_args) => {
                cli::autopilot::select(cli::autopilot::SelectOptions {
                    proposal: select_args.proposal,
                    candidate: select_args.candidate,
                    out: select_args.out,
                    markdown: select_args.markdown,
                    no_markdown: select_args.no_markdown,
                    json: select_args.format.is_json(),
                })
            }
            AutopilotCommand::Launch(launch_args) => {
                cli::autopilot::launch(cli::autopilot::LaunchOptions {
                    port: args.port,
                    selection: launch_args.selection,
                    limit: launch_args.limit,
                    assume: launch_args.assume,
                    smoke: launch_args.smoke,
                    out: launch_args.out,
                    markdown: launch_args.markdown,
                    no_markdown: launch_args.no_markdown,
                    json: launch_args.format.is_json(),
                })
            }
            AutopilotCommand::Drive(drive_args) => {
                cli::autopilot::drive(cli::autopilot::DriveOptions {
                    port: args.port,
                    prompt: if drive_args.prompt.is_empty() {
                        None
                    } else {
                        Some(drive_args.prompt.join(" "))
                    },
                    studio: drive_args.studio,
                    scope: drive_args.scope,
                    root: drive_args.root,
                    run_dir: drive_args.run_dir,
                    limit: drive_args.limit,
                    assume: drive_args.assume,
                    smoke: drive_args.smoke,
                    out: drive_args.out,
                    markdown: drive_args.markdown,
                    no_markdown: drive_args.no_markdown,
                    json: drive_args.format.is_json(),
                })
            }
            AutopilotCommand::Cockpit(cockpit_args) => {
                cli::autopilot::cockpit(cli::autopilot::CockpitOptions {
                    prompt: if cockpit_args.prompt.is_empty() {
                        None
                    } else {
                        Some(cockpit_args.prompt.join(" "))
                    },
                    run_dir: cockpit_args.run_dir,
                    root: cockpit_args.root,
                    limit: cockpit_args.limit,
                    out: cockpit_args.out,
                    markdown: cockpit_args.markdown,
                    no_markdown: cockpit_args.no_markdown,
                    json: cockpit_args.format.is_json(),
                })
            }
            AutopilotCommand::Capsule(capsule_args) => {
                cli::autopilot::capsule(cli::autopilot::CapsuleOptions {
                    prompt: if capsule_args.prompt.is_empty() {
                        None
                    } else {
                        Some(capsule_args.prompt.join(" "))
                    },
                    run_dir: capsule_args.run_dir,
                    root: capsule_args.root,
                    limit: capsule_args.limit,
                    out: capsule_args.out,
                    markdown: capsule_args.markdown,
                    no_markdown: capsule_args.no_markdown,
                    json: capsule_args.format.is_json(),
                })
            }
            AutopilotCommand::Orient(orient_args) => {
                cli::autopilot::orient(cli::autopilot::OrientOptions {
                    prompt: if orient_args.prompt.is_empty() {
                        None
                    } else {
                        Some(orient_args.prompt.join(" "))
                    },
                    run_dir: orient_args.run_dir,
                    root: orient_args.root,
                    limit: orient_args.limit,
                    out: orient_args.out,
                    markdown: orient_args.markdown,
                    no_markdown: orient_args.no_markdown,
                    json: orient_args.format.is_json(),
                })
            }
            AutopilotCommand::ModelPack(pack_args) => {
                cli::autopilot::model_pack(cli::autopilot::ModelPackOptions {
                    prompt: if pack_args.prompt.is_empty() {
                        None
                    } else {
                        Some(pack_args.prompt.join(" "))
                    },
                    run_dir: pack_args.run_dir,
                    root: pack_args.root,
                    limit: pack_args.limit,
                    max_chars: pack_args.max_chars,
                    out: pack_args.out,
                    markdown: pack_args.markdown,
                    no_markdown: pack_args.no_markdown,
                    json: pack_args.format.is_json(),
                })
            }
            AutopilotCommand::TaskPack(pack_args) => {
                cli::autopilot::task_pack(cli::autopilot::TaskPackOptions {
                    prompt: if pack_args.prompt.is_empty() {
                        None
                    } else {
                        Some(pack_args.prompt.join(" "))
                    },
                    run_dir: pack_args.run_dir,
                    root: pack_args.root,
                    limit: pack_args.limit,
                    opportunity: pack_args.opportunity,
                    max_chars: pack_args.max_chars,
                    out: pack_args.out,
                    markdown: pack_args.markdown,
                    no_markdown: pack_args.no_markdown,
                    json: pack_args.format.is_json(),
                })
            }
            AutopilotCommand::BestFriend(friend_args) => {
                cli::autopilot::best_friend(cli::autopilot::BestFriendOptions {
                    prompt: if friend_args.prompt.is_empty() {
                        None
                    } else {
                        Some(friend_args.prompt.join(" "))
                    },
                    run_dir: friend_args.run_dir,
                    root: friend_args.root,
                    limit: friend_args.limit,
                    opportunity: friend_args.opportunity,
                    max_chars: friend_args.max_chars,
                    out: friend_args.out,
                    markdown: friend_args.markdown,
                    no_markdown: friend_args.no_markdown,
                    json: friend_args.format.is_json(),
                })
            }
            AutopilotCommand::BestFriendCheck(check_args) => {
                cli::autopilot::best_friend_check(cli::autopilot::BestFriendCheckOptions {
                    prompt: if check_args.prompt.is_empty() {
                        None
                    } else {
                        Some(check_args.prompt.join(" "))
                    },
                    run_dir: check_args.run_dir,
                    root: check_args.root,
                    limit: check_args.limit,
                    opportunity: check_args.opportunity,
                    max_chars: check_args.max_chars,
                    out: check_args.out,
                    markdown: check_args.markdown,
                    no_markdown: check_args.no_markdown,
                    json: check_args.format.is_json(),
                })
            }
            AutopilotCommand::BestFriendRescue(rescue_args) => {
                cli::autopilot::best_friend_rescue(cli::autopilot::BestFriendRescueOptions {
                    prompt: if rescue_args.prompt.is_empty() {
                        None
                    } else {
                        Some(rescue_args.prompt.join(" "))
                    },
                    run_dir: rescue_args.run_dir,
                    root: rescue_args.root,
                    limit: rescue_args.limit,
                    opportunity: rescue_args.opportunity,
                    max_chars: rescue_args.max_chars,
                    command: rescue_args.command,
                    result: rescue_args.result,
                    errors: rescue_args.error,
                    evidence: rescue_args.evidence,
                    out: rescue_args.out,
                    markdown: rescue_args.markdown,
                    no_markdown: rescue_args.no_markdown,
                    json: rescue_args.format.is_json(),
                })
            }
            AutopilotCommand::BestFriendMentor(mentor_args) => {
                cli::autopilot::best_friend_mentor(cli::autopilot::BestFriendMentorOptions {
                    prompt: if mentor_args.prompt.is_empty() {
                        None
                    } else {
                        Some(mentor_args.prompt.join(" "))
                    },
                    run_dir: mentor_args.run_dir,
                    root: mentor_args.root,
                    limit: mentor_args.limit,
                    opportunity: mentor_args.opportunity,
                    max_chars: mentor_args.max_chars,
                    out: mentor_args.out,
                    markdown: mentor_args.markdown,
                    no_markdown: mentor_args.no_markdown,
                    json: mentor_args.format.is_json(),
                })
            }
            AutopilotCommand::BestFriendPilot(pilot_args) => {
                cli::autopilot::best_friend_pilot(cli::autopilot::BestFriendPilotOptions {
                    prompt: if pilot_args.prompt.is_empty() {
                        None
                    } else {
                        Some(pilot_args.prompt.join(" "))
                    },
                    run_dir: pilot_args.run_dir,
                    root: pilot_args.root,
                    limit: pilot_args.limit,
                    opportunity: pilot_args.opportunity,
                    max_chars: pilot_args.max_chars,
                    dry_run: pilot_args.dry_run,
                    out: pilot_args.out,
                    markdown: pilot_args.markdown,
                    no_markdown: pilot_args.no_markdown,
                    json: pilot_args.format.is_json(),
                })
            }
            AutopilotCommand::BestFriendControl(control_args) => {
                cli::autopilot::best_friend_control(cli::autopilot::BestFriendControlOptions {
                    prompt: if control_args.prompt.is_empty() {
                        None
                    } else {
                        Some(control_args.prompt.join(" "))
                    },
                    run_dir: control_args.run_dir,
                    root: control_args.root,
                    limit: control_args.limit,
                    opportunity: control_args.opportunity,
                    max_chars: control_args.max_chars,
                    out: control_args.out,
                    markdown: control_args.markdown,
                    no_markdown: control_args.no_markdown,
                    json: control_args.format.is_json(),
                })
            }
            AutopilotCommand::BestFriendOperate(operate_args) => {
                cli::autopilot::best_friend_operate(cli::autopilot::BestFriendOperateOptions {
                    prompt: if operate_args.prompt.is_empty() {
                        None
                    } else {
                        Some(operate_args.prompt.join(" "))
                    },
                    run_dir: operate_args.run_dir,
                    root: operate_args.root,
                    limit: operate_args.limit,
                    opportunity: operate_args.opportunity,
                    max_chars: operate_args.max_chars,
                    dry_run: operate_args.dry_run,
                    artifact_prefix: None,
                    out: operate_args.out,
                    markdown: operate_args.markdown,
                    no_markdown: operate_args.no_markdown,
                    json: operate_args.format.is_json(),
                })
            }
            AutopilotCommand::BestFriendRunner(runner_args) => {
                cli::autopilot::best_friend_runner(cli::autopilot::BestFriendRunnerOptions {
                    prompt: if runner_args.prompt.is_empty() {
                        None
                    } else {
                        Some(runner_args.prompt.join(" "))
                    },
                    run_dir: runner_args.run_dir,
                    root: runner_args.root,
                    limit: runner_args.limit,
                    opportunity: runner_args.opportunity,
                    max_chars: runner_args.max_chars,
                    max_steps: runner_args.max_steps,
                    dry_run: runner_args.dry_run,
                    out: runner_args.out,
                    markdown: runner_args.markdown,
                    no_markdown: runner_args.no_markdown,
                    json: runner_args.format.is_json(),
                })
            }
            AutopilotCommand::FirstTurn(turn_args) => {
                cli::autopilot::first_turn(cli::autopilot::FirstTurnOptions {
                    prompt: if turn_args.prompt.is_empty() {
                        None
                    } else {
                        Some(turn_args.prompt.join(" "))
                    },
                    run_dir: turn_args.run_dir,
                    root: turn_args.root,
                    limit: turn_args.limit,
                    opportunity: turn_args.opportunity,
                    max_chars: turn_args.max_chars,
                    dry_run: turn_args.dry_run,
                    artifact_prefix: None,
                    out: turn_args.out,
                    markdown: turn_args.markdown,
                    no_markdown: turn_args.no_markdown,
                    json: turn_args.format.is_json(),
                })
            }
            AutopilotCommand::BestFriendLoop(loop_args) => {
                cli::autopilot::best_friend_loop(cli::autopilot::BestFriendLoopOptions {
                    prompt: if loop_args.prompt.is_empty() {
                        None
                    } else {
                        Some(loop_args.prompt.join(" "))
                    },
                    run_dir: loop_args.run_dir,
                    root: loop_args.root,
                    limit: loop_args.limit,
                    opportunity: loop_args.opportunity,
                    max_chars: loop_args.max_chars,
                    max_steps: loop_args.max_steps,
                    dry_run: loop_args.dry_run,
                    out: loop_args.out,
                    markdown: loop_args.markdown,
                    no_markdown: loop_args.no_markdown,
                    json: loop_args.format.is_json(),
                })
            }
            AutopilotCommand::BestFriendReply(reply_args) => {
                cli::autopilot::best_friend_reply(cli::autopilot::BestFriendReplyOptions {
                    run_dir: reply_args.run_dir,
                    root: reply_args.root,
                    limit: reply_args.limit,
                    out: reply_args.out,
                    markdown: reply_args.markdown,
                    no_markdown: reply_args.no_markdown,
                    json: reply_args.format.is_json(),
                })
            }
            AutopilotCommand::BestFriendTurn(turn_args) => {
                cli::autopilot::best_friend_turn(cli::autopilot::BestFriendTurnOptions {
                    port: args.port,
                    prompt: if turn_args.prompt.is_empty() {
                        None
                    } else {
                        Some(turn_args.prompt.join(" "))
                    },
                    message: turn_args.message,
                    source: turn_args.source,
                    run_dir: turn_args.run_dir,
                    root: turn_args.root,
                    limit: turn_args.limit,
                    opportunity: turn_args.opportunity,
                    max_chars: turn_args.max_chars,
                    max_steps: turn_args.max_steps,
                    dry_run: turn_args.dry_run,
                    out: turn_args.out,
                    markdown: turn_args.markdown,
                    no_markdown: turn_args.no_markdown,
                    json: turn_args.format.is_json(),
                })
            }
            AutopilotCommand::BestFriendSession(session_args) => {
                cli::autopilot::best_friend_session(cli::autopilot::BestFriendSessionOptions {
                    port: args.port,
                    prompt: if session_args.prompt.is_empty() {
                        None
                    } else {
                        Some(session_args.prompt.join(" "))
                    },
                    message: session_args.message,
                    source: session_args.source,
                    scout: session_args.scout,
                    survey: session_args.survey,
                    context: session_args.context,
                    run_dir: session_args.run_dir,
                    root: session_args.root,
                    scope: session_args.scope,
                    limit: session_args.limit,
                    assume: session_args.assume,
                    smoke: session_args.smoke,
                    opportunity: session_args.opportunity,
                    max_chars: session_args.max_chars,
                    max_steps: session_args.max_steps,
                    dry_run: session_args.dry_run,
                    out: session_args.out,
                    markdown: session_args.markdown,
                    no_markdown: session_args.no_markdown,
                    json: session_args.format.is_json(),
                })
            }
            AutopilotCommand::WowSession(session_args) => {
                cli::autopilot::wow_session(cli::autopilot::WowSessionOptions {
                    port: args.port,
                    prompt: if session_args.prompt.is_empty() {
                        None
                    } else {
                        Some(session_args.prompt.join(" "))
                    },
                    message: session_args.message,
                    source: session_args.source,
                    scout: session_args.scout,
                    survey: session_args.survey,
                    context: session_args.context,
                    run_dir: session_args.run_dir,
                    root: session_args.root,
                    scope: session_args.scope,
                    limit: session_args.limit,
                    assume: session_args.assume,
                    smoke: session_args.smoke,
                    idea: session_args.idea,
                    opportunity: session_args.opportunity,
                    max_ideas: session_args.max_ideas,
                    max_chars: session_args.max_chars,
                    max_steps: session_args.max_steps,
                    dry_run: session_args.dry_run,
                    out: session_args.out,
                    markdown: session_args.markdown,
                    no_markdown: session_args.no_markdown,
                    json: session_args.format.is_json(),
                })
            }
            AutopilotCommand::BestFriendArc(arc_args) => {
                cli::autopilot::best_friend_arc(cli::autopilot::BestFriendArcOptions {
                    port: args.port,
                    prompt: if arc_args.prompt.is_empty() {
                        None
                    } else {
                        Some(arc_args.prompt.join(" "))
                    },
                    message: arc_args.message,
                    source: arc_args.source,
                    scout: arc_args.scout,
                    survey: arc_args.survey,
                    context: arc_args.context,
                    run_dir: arc_args.run_dir,
                    root: arc_args.root,
                    scope: arc_args.scope,
                    limit: arc_args.limit,
                    assume: arc_args.assume,
                    smoke: arc_args.smoke,
                    idea: arc_args.idea,
                    opportunity: arc_args.opportunity,
                    max_ideas: arc_args.max_ideas,
                    max_chars: arc_args.max_chars,
                    max_steps: arc_args.max_steps,
                    dry_run: arc_args.dry_run,
                    out: arc_args.out,
                    markdown: arc_args.markdown,
                    no_markdown: arc_args.no_markdown,
                    json: arc_args.format.is_json(),
                })
            }
            AutopilotCommand::SquadPack(pack_args) => {
                cli::autopilot::squad_pack(cli::autopilot::SquadPackOptions {
                    prompt: if pack_args.prompt.is_empty() {
                        None
                    } else {
                        Some(pack_args.prompt.join(" "))
                    },
                    run_dir: pack_args.run_dir,
                    root: pack_args.root,
                    limit: pack_args.limit,
                    max_tasks: pack_args.max_tasks,
                    max_chars: pack_args.max_chars,
                    out: pack_args.out,
                    markdown: pack_args.markdown,
                    no_markdown: pack_args.no_markdown,
                    json: pack_args.format.is_json(),
                })
            }
            AutopilotCommand::SquadReview(review_args) => {
                cli::autopilot::squad_review(cli::autopilot::SquadReviewOptions {
                    run_dir: review_args.run_dir,
                    root: review_args.root,
                    limit: review_args.limit,
                    max_tasks: review_args.max_tasks,
                    max_chars: review_args.max_chars,
                    out: review_args.out,
                    markdown: review_args.markdown,
                    no_markdown: review_args.no_markdown,
                    json: review_args.format.is_json(),
                })
            }
            AutopilotCommand::WowPlan(plan_args) => {
                cli::autopilot::wow_plan(cli::autopilot::WowPlanOptions {
                    prompt: if plan_args.prompt.is_empty() {
                        None
                    } else {
                        Some(plan_args.prompt.join(" "))
                    },
                    run_dir: plan_args.run_dir,
                    root: plan_args.root,
                    limit: plan_args.limit,
                    max_ideas: plan_args.max_ideas,
                    out: plan_args.out,
                    markdown: plan_args.markdown,
                    no_markdown: plan_args.no_markdown,
                    json: plan_args.format.is_json(),
                })
            }
            AutopilotCommand::MomentPack(pack_args) => {
                cli::autopilot::moment_pack(cli::autopilot::MomentPackOptions {
                    prompt: if pack_args.prompt.is_empty() {
                        None
                    } else {
                        Some(pack_args.prompt.join(" "))
                    },
                    idea: pack_args.idea,
                    run_dir: pack_args.run_dir,
                    root: pack_args.root,
                    limit: pack_args.limit,
                    max_ideas: pack_args.max_ideas,
                    max_chars: pack_args.max_chars,
                    out: pack_args.out,
                    markdown: pack_args.markdown,
                    no_markdown: pack_args.no_markdown,
                    json: pack_args.format.is_json(),
                })
            }
            AutopilotCommand::MomentSprint(sprint_args) => {
                cli::autopilot::moment_sprint(cli::autopilot::MomentSprintOptions {
                    prompt: if sprint_args.prompt.is_empty() {
                        None
                    } else {
                        Some(sprint_args.prompt.join(" "))
                    },
                    idea: sprint_args.idea,
                    run_dir: sprint_args.run_dir,
                    root: sprint_args.root,
                    limit: sprint_args.limit,
                    max_ideas: sprint_args.max_ideas,
                    max_chars: sprint_args.max_chars,
                    dry_run: sprint_args.dry_run,
                    out: sprint_args.out,
                    markdown: sprint_args.markdown,
                    no_markdown: sprint_args.no_markdown,
                    json: sprint_args.format.is_json(),
                })
            }
            AutopilotCommand::MomentDecision(decision_args) => {
                cli::autopilot::moment_decision(cli::autopilot::MomentDecisionOptions {
                    prompt: if decision_args.prompt.is_empty() {
                        None
                    } else {
                        Some(decision_args.prompt.join(" "))
                    },
                    idea: decision_args.idea,
                    run_dir: decision_args.run_dir,
                    root: decision_args.root,
                    limit: decision_args.limit,
                    max_ideas: decision_args.max_ideas,
                    max_chars: decision_args.max_chars,
                    dry_run: decision_args.dry_run,
                    out: decision_args.out,
                    markdown: decision_args.markdown,
                    no_markdown: decision_args.no_markdown,
                    json: decision_args.format.is_json(),
                })
            }
            AutopilotCommand::CreatorDemo(demo_args) => {
                cli::autopilot::creator_demo(cli::autopilot::CreatorDemoOptions {
                    prompt: if demo_args.prompt.is_empty() {
                        None
                    } else {
                        Some(demo_args.prompt.join(" "))
                    },
                    idea: demo_args.idea,
                    run_dir: demo_args.run_dir,
                    root: demo_args.root,
                    limit: demo_args.limit,
                    max_ideas: demo_args.max_ideas,
                    max_chars: demo_args.max_chars,
                    dry_run: demo_args.dry_run,
                    out: demo_args.out,
                    markdown: demo_args.markdown,
                    no_markdown: demo_args.no_markdown,
                    json: demo_args.format.is_json(),
                })
            }
            AutopilotCommand::DemoResponse(response_args) => {
                let mut messages = response_args.message;
                if !response_args.response.is_empty() {
                    messages.push(response_args.response.join(" "));
                }
                cli::autopilot::demo_response(cli::autopilot::DemoResponseOptions {
                    messages,
                    idea: response_args.idea,
                    run_dir: response_args.run_dir,
                    root: response_args.root,
                    limit: response_args.limit,
                    max_ideas: response_args.max_ideas,
                    max_chars: response_args.max_chars,
                    dry_run: response_args.dry_run,
                    out: response_args.out,
                    markdown: response_args.markdown,
                    no_markdown: response_args.no_markdown,
                    json: response_args.format.is_json(),
                })
            }
            AutopilotCommand::DemoLoop(loop_args) => {
                let mut messages = loop_args.message;
                if !loop_args.response.is_empty() {
                    messages.push(loop_args.response.join(" "));
                }
                cli::autopilot::demo_loop(cli::autopilot::DemoLoopOptions {
                    messages,
                    idea: loop_args.idea,
                    run_dir: loop_args.run_dir,
                    root: loop_args.root,
                    limit: loop_args.limit,
                    max_ideas: loop_args.max_ideas,
                    max_chars: loop_args.max_chars,
                    dry_run: loop_args.dry_run,
                    out: loop_args.out,
                    markdown: loop_args.markdown,
                    no_markdown: loop_args.no_markdown,
                    json: loop_args.format.is_json(),
                })
            }
            AutopilotCommand::DemoSession(session_args) => {
                let mut messages = session_args.message;
                if !session_args.response.is_empty() {
                    messages.push(session_args.response.join(" "));
                }
                cli::autopilot::demo_session(cli::autopilot::DemoSessionOptions {
                    messages,
                    idea: session_args.idea,
                    run_dir: session_args.run_dir,
                    root: session_args.root,
                    limit: session_args.limit,
                    max_ideas: session_args.max_ideas,
                    max_chars: session_args.max_chars,
                    dry_run: session_args.dry_run,
                    out: session_args.out,
                    markdown: session_args.markdown,
                    no_markdown: session_args.no_markdown,
                    json: session_args.format.is_json(),
                })
            }
            AutopilotCommand::DemoCheck(check_args) => {
                cli::autopilot::demo_check(cli::autopilot::DemoCheckOptions {
                    run_dir: check_args.run_dir,
                    out: check_args.out,
                    markdown: check_args.markdown,
                    no_markdown: check_args.no_markdown,
                    json: check_args.format.is_json(),
                })
            }
            AutopilotCommand::DemoReply(reply_args) => {
                cli::autopilot::demo_reply(cli::autopilot::DemoReplyOptions {
                    run_dir: reply_args.run_dir,
                    out: reply_args.out,
                    markdown: reply_args.markdown,
                    no_markdown: reply_args.no_markdown,
                    json: reply_args.format.is_json(),
                })
            }
            AutopilotCommand::DemoLearn(learn_args) => {
                cli::autopilot::demo_learn(cli::autopilot::DemoLearnOptions {
                    run_dir: learn_args.run_dir,
                    out: learn_args.out,
                    markdown: learn_args.markdown,
                    no_markdown: learn_args.no_markdown,
                    json: learn_args.format.is_json(),
                })
            }
            AutopilotCommand::Remember(remember_args) => {
                cli::autopilot::remember(cli::autopilot::RememberOptions {
                    run_dir: remember_args.run_dir,
                    root: remember_args.root,
                    limit: remember_args.limit,
                    best_friend: remember_args.best_friend,
                    opportunity: remember_args.opportunity,
                    max_chars: remember_args.max_chars,
                    out: remember_args.out,
                    markdown: remember_args.markdown,
                    no_markdown: remember_args.no_markdown,
                    json: remember_args.format.is_json(),
                })
            }
            AutopilotCommand::ReviewPack(review_args) => {
                cli::autopilot::review_pack(cli::autopilot::ReviewPackOptions {
                    run_dir: review_args.run_dir,
                    out: review_args.out,
                    markdown: review_args.markdown,
                    no_markdown: review_args.no_markdown,
                    create_evidence_dirs: !review_args.no_create_evidence_dirs,
                    json: review_args.format.is_json(),
                })
            }
            AutopilotCommand::PublishReview(publish_args) => {
                cli::autopilot::publish_review(cli::autopilot::PublishReviewOptions {
                    port: args.port,
                    studio: publish_args.studio,
                    run_dir: publish_args.run_dir,
                    companion: publish_args.companion,
                    arc: publish_args.arc,
                    best_friend: publish_args.best_friend,
                    best_friend_pilot: publish_args.best_friend_pilot,
                    best_friend_runner: publish_args.best_friend_runner,
                    out: publish_args.out,
                    markdown: publish_args.markdown,
                    no_markdown: publish_args.no_markdown,
                    json: publish_args.format.is_json(),
                })
            }
            AutopilotCommand::PublishPrep(prep_args) => {
                cli::autopilot::publish_prep(cli::autopilot::PublishPrepOptions {
                    run_dir: prep_args.run_dir,
                    out: prep_args.out,
                    markdown: prep_args.markdown,
                    no_markdown: prep_args.no_markdown,
                    json: prep_args.format.is_json(),
                })
            }
            AutopilotCommand::Feedback(feedback_args) => {
                cli::autopilot::feedback(cli::autopilot::FeedbackOptions {
                    run_dir: feedback_args.run_dir,
                    notes: feedback_args.note,
                    source: feedback_args.source,
                    out: feedback_args.out,
                    markdown: feedback_args.markdown,
                    no_markdown: feedback_args.no_markdown,
                    json: feedback_args.format.is_json(),
                })
            }
            AutopilotCommand::FeedbackPatch(patch_args) => {
                cli::autopilot::feedback_patch(cli::autopilot::FeedbackPatchOptions {
                    run_dir: patch_args.run_dir,
                    feedback: patch_args.feedback,
                    out: patch_args.out,
                    markdown: patch_args.markdown,
                    planner_pack: patch_args.planner_pack,
                    no_planner_pack: patch_args.no_planner_pack,
                    no_markdown: patch_args.no_markdown,
                    json: patch_args.format.is_json(),
                })
            }
            AutopilotCommand::ClaimCheck(claim_args) => {
                cli::autopilot::claim_check(cli::autopilot::ClaimCheckOptions {
                    run_dir: claim_args.run_dir,
                    claims: claim_args.claim,
                    out: claim_args.out,
                    markdown: claim_args.markdown,
                    no_markdown: claim_args.no_markdown,
                    json: claim_args.format.is_json(),
                })
            }
            AutopilotCommand::Respond(respond_args) => {
                cli::autopilot::respond(cli::autopilot::RespondOptions {
                    run_dir: respond_args.run_dir,
                    claims: respond_args.claim,
                    out: respond_args.out,
                    markdown: respond_args.markdown,
                    no_markdown: respond_args.no_markdown,
                    json: respond_args.format.is_json(),
                })
            }
            AutopilotCommand::Decision(decision_args) => {
                cli::autopilot::decision(cli::autopilot::DecisionOptions {
                    run_dir: decision_args.run_dir,
                    source: decision_args.source,
                    decisions: decision_args.decision,
                    constraints: decision_args.constraint,
                    rejections: decision_args.rejection,
                    notes: decision_args.note,
                    out: decision_args.out,
                    markdown: decision_args.markdown,
                    no_markdown: decision_args.no_markdown,
                    json: decision_args.format.is_json(),
                })
            }
            AutopilotCommand::Align(align_args) => {
                cli::autopilot::align(cli::autopilot::AlignOptions {
                    run_dir: align_args.run_dir,
                    decisions: align_args.decisions,
                    out: align_args.out,
                    markdown: align_args.markdown,
                    no_markdown: align_args.no_markdown,
                    json: align_args.format.is_json(),
                })
            }
            AutopilotCommand::Journal(journal_args) => {
                cli::autopilot::journal(cli::autopilot::JournalOptions {
                    run_dir: journal_args.run_dir,
                    source: journal_args.source,
                    entries: journal_args.entry,
                    commands: journal_args.command,
                    results: journal_args.result,
                    evidence: journal_args.evidence,
                    out: journal_args.out,
                    markdown: journal_args.markdown,
                    no_markdown: journal_args.no_markdown,
                    json: journal_args.format.is_json(),
                })
            }
            AutopilotCommand::Proof(proof_args) => {
                cli::autopilot::proof(cli::autopilot::ProofOptions {
                    run_dir: proof_args.run_dir,
                    out: proof_args.out,
                    markdown: proof_args.markdown,
                    no_markdown: proof_args.no_markdown,
                    json: proof_args.format.is_json(),
                })
            }
            AutopilotCommand::Acceptance(acceptance_args) => {
                cli::autopilot::acceptance(cli::autopilot::AcceptanceOptions {
                    run_dir: acceptance_args.run_dir,
                    prompt: if acceptance_args.prompt.is_empty() {
                        None
                    } else {
                        Some(acceptance_args.prompt.join(" "))
                    },
                    out: acceptance_args.out,
                    markdown: acceptance_args.markdown,
                    no_markdown: acceptance_args.no_markdown,
                    json: acceptance_args.format.is_json(),
                })
            }
            AutopilotCommand::Fulfillment(fulfillment_args) => {
                cli::autopilot::fulfillment(cli::autopilot::FulfillmentOptions {
                    run_dir: fulfillment_args.run_dir,
                    prompt: if fulfillment_args.prompt.is_empty() {
                        None
                    } else {
                        Some(fulfillment_args.prompt.join(" "))
                    },
                    out: fulfillment_args.out,
                    markdown: fulfillment_args.markdown,
                    no_markdown: fulfillment_args.no_markdown,
                    json: fulfillment_args.format.is_json(),
                })
            }
            AutopilotCommand::CompletionAudit(audit_args) => {
                cli::autopilot::completion_audit(cli::autopilot::CompletionAuditOptions {
                    run_dir: audit_args.run_dir,
                    prompt: if audit_args.prompt.is_empty() {
                        None
                    } else {
                        Some(audit_args.prompt.join(" "))
                    },
                    out: audit_args.out,
                    markdown: audit_args.markdown,
                    no_markdown: audit_args.no_markdown,
                    json: audit_args.format.is_json(),
                })
            }
            AutopilotCommand::Deliver(deliver_args) => {
                cli::autopilot::deliver(cli::autopilot::DeliverOptions {
                    run_dir: deliver_args.run_dir,
                    prompt: if deliver_args.prompt.is_empty() {
                        None
                    } else {
                        Some(deliver_args.prompt.join(" "))
                    },
                    out: deliver_args.out,
                    markdown: deliver_args.markdown,
                    no_markdown: deliver_args.no_markdown,
                    json: deliver_args.format.is_json(),
                })
            }
            AutopilotCommand::Satisfy(satisfy_args) => {
                cli::autopilot::satisfy(cli::autopilot::SatisfyOptions {
                    port: args.port,
                    run_dir: satisfy_args.run_dir,
                    prompt: if satisfy_args.prompt.is_empty() {
                        None
                    } else {
                        Some(satisfy_args.prompt.join(" "))
                    },
                    patch_run: satisfy_args.patch_run,
                    out: satisfy_args.out,
                    markdown: satisfy_args.markdown,
                    no_markdown: satisfy_args.no_markdown,
                    max_recipes: satisfy_args.max_recipes,
                    smoke: satisfy_args.smoke,
                    dry_run: satisfy_args.dry_run,
                    json: satisfy_args.format.is_json(),
                })
            }
            AutopilotCommand::PromiseLoop(loop_args) => {
                cli::autopilot::promise_loop(cli::autopilot::PromiseLoopOptions {
                    port: args.port,
                    run_dir: loop_args.run_dir,
                    prompt: if loop_args.prompt.is_empty() {
                        None
                    } else {
                        Some(loop_args.prompt.join(" "))
                    },
                    out: loop_args.out,
                    markdown: loop_args.markdown,
                    no_markdown: loop_args.no_markdown,
                    max_steps: loop_args.max_steps,
                    max_recipes: loop_args.max_recipes,
                    smoke: loop_args.smoke,
                    dry_run: loop_args.dry_run,
                    json: loop_args.format.is_json(),
                })
            }
            AutopilotCommand::Trace(trace_args) => {
                cli::autopilot::trace(cli::autopilot::TraceOptions {
                    run_dir: trace_args.run_dir,
                    prompt: if trace_args.prompt.is_empty() {
                        None
                    } else {
                        Some(trace_args.prompt.join(" "))
                    },
                    out: trace_args.out,
                    markdown: trace_args.markdown,
                    no_markdown: trace_args.no_markdown,
                    json: trace_args.format.is_json(),
                })
            }
            AutopilotCommand::Refresh(refresh_args) => {
                cli::autopilot::refresh(cli::autopilot::RefreshOptions {
                    run_dir: refresh_args.run_dir,
                    out: refresh_args.out,
                    markdown: refresh_args.markdown,
                    no_markdown: refresh_args.no_markdown,
                    json: refresh_args.format.is_json(),
                })
            }
            AutopilotCommand::Rollback(rollback_args) => {
                cli::autopilot::rollback(cli::autopilot::RollbackOptions {
                    run_dir: rollback_args.run_dir,
                    out: rollback_args.out,
                    markdown: rollback_args.markdown,
                    no_markdown: rollback_args.no_markdown,
                    json: rollback_args.format.is_json(),
                })
            }
            AutopilotCommand::Approval(approval_args) => {
                cli::autopilot::approval(cli::autopilot::ApprovalOptions {
                    run_dir: approval_args.run_dir,
                    out: approval_args.out,
                    markdown: approval_args.markdown,
                    no_markdown: approval_args.no_markdown,
                    json: approval_args.format.is_json(),
                })
            }
            AutopilotCommand::Privacy(privacy_args) => {
                cli::autopilot::privacy(cli::autopilot::PrivacyOptions {
                    run_dir: privacy_args.run_dir,
                    out: privacy_args.out,
                    markdown: privacy_args.markdown,
                    no_markdown: privacy_args.no_markdown,
                    json: privacy_args.format.is_json(),
                })
            }
            AutopilotCommand::Next(next_args) => {
                cli::autopilot::next(cli::autopilot::NextOptions {
                    prompt: if next_args.prompt.is_empty() {
                        None
                    } else {
                        Some(next_args.prompt.join(" "))
                    },
                    run_dir: next_args.run_dir,
                    root: next_args.root,
                    limit: next_args.limit,
                    out: next_args.out,
                    markdown: next_args.markdown,
                    no_markdown: next_args.no_markdown,
                    json: next_args.format.is_json(),
                })
            }
            AutopilotCommand::Opportunities(opportunities_args) => {
                cli::autopilot::opportunities(cli::autopilot::OpportunitiesOptions {
                    prompt: if opportunities_args.prompt.is_empty() {
                        None
                    } else {
                        Some(opportunities_args.prompt.join(" "))
                    },
                    run_dir: opportunities_args.run_dir,
                    root: opportunities_args.root,
                    limit: opportunities_args.limit,
                    out: opportunities_args.out,
                    markdown: opportunities_args.markdown,
                    no_markdown: opportunities_args.no_markdown,
                    json: opportunities_args.format.is_json(),
                })
            }
            AutopilotCommand::WorkOrder(work_order_args) => {
                cli::autopilot::work_order(cli::autopilot::WorkOrderOptions {
                    prompt: if work_order_args.prompt.is_empty() {
                        None
                    } else {
                        Some(work_order_args.prompt.join(" "))
                    },
                    run_dir: work_order_args.run_dir,
                    opportunity: work_order_args.opportunity,
                    root: work_order_args.root,
                    limit: work_order_args.limit,
                    out: work_order_args.out,
                    markdown: work_order_args.markdown,
                    no_markdown: work_order_args.no_markdown,
                    json: work_order_args.format.is_json(),
                })
            }
            AutopilotCommand::WorkCheck(work_check_args) => {
                cli::autopilot::work_check(cli::autopilot::WorkCheckOptions {
                    run_dir: work_check_args.run_dir,
                    work_order: work_check_args.work_order,
                    out: work_check_args.out,
                    markdown: work_check_args.markdown,
                    no_markdown: work_check_args.no_markdown,
                    json: work_check_args.format.is_json(),
                })
            }
            AutopilotCommand::Cycle(cycle_args) => {
                cli::autopilot::cycle(cli::autopilot::CycleOptions {
                    run_dir: cycle_args.run_dir,
                    prompt: cycle_args.prompt,
                    root: cycle_args.root,
                    limit: cycle_args.limit,
                    out: cycle_args.out,
                    markdown: cycle_args.markdown,
                    no_markdown: cycle_args.no_markdown,
                    json: cycle_args.format.is_json(),
                })
            }
            AutopilotCommand::Diagnose(diagnose_args) => {
                cli::autopilot::diagnose(cli::autopilot::DiagnoseOptions {
                    run_dir: diagnose_args.run_dir,
                    command: diagnose_args.command,
                    result: diagnose_args.result,
                    errors: diagnose_args.error,
                    evidence: diagnose_args.evidence,
                    out: diagnose_args.out,
                    markdown: diagnose_args.markdown,
                    no_markdown: diagnose_args.no_markdown,
                    json: diagnose_args.format.is_json(),
                })
            }
            AutopilotCommand::CommandGuard(command_guard_args) => {
                cli::autopilot::command_guard(cli::autopilot::CommandGuardOptions {
                    run_dir: command_guard_args.run_dir,
                    root: command_guard_args.root,
                    limit: command_guard_args.limit,
                    commands: command_guard_args.command,
                    from_file: command_guard_args.from_file,
                    out: command_guard_args.out,
                    markdown: command_guard_args.markdown,
                    no_markdown: command_guard_args.no_markdown,
                    json: command_guard_args.format.is_json(),
                })
            }
            AutopilotCommand::SelfCheck(self_check_args) => {
                cli::autopilot::self_check(cli::autopilot::SelfCheckOptions {
                    run_dir: self_check_args.run_dir,
                    root: self_check_args.root,
                    limit: self_check_args.limit,
                    claims: self_check_args.claim,
                    messages: self_check_args.message,
                    commands: self_check_args.command,
                    from_file: self_check_args.from_file,
                    out: self_check_args.out,
                    markdown: self_check_args.markdown,
                    no_markdown: self_check_args.no_markdown,
                    json: self_check_args.format.is_json(),
                })
            }
            AutopilotCommand::Runbook(runbook_args) => {
                cli::autopilot::runbook(cli::autopilot::RunbookOptions {
                    prompt: if runbook_args.prompt.is_empty() {
                        None
                    } else {
                        Some(runbook_args.prompt.join(" "))
                    },
                    run_dir: runbook_args.run_dir,
                    root: runbook_args.root,
                    limit: runbook_args.limit,
                    max_steps: runbook_args.max_steps,
                    commands: runbook_args.command,
                    from_file: runbook_args.from_file,
                    out: runbook_args.out,
                    markdown: runbook_args.markdown,
                    no_markdown: runbook_args.no_markdown,
                    json: runbook_args.format.is_json(),
                })
            }
            AutopilotCommand::FlightRecorder(flight_recorder_args) => {
                cli::autopilot::flight_recorder(cli::autopilot::FlightRecorderOptions {
                    run_dir: flight_recorder_args.run_dir,
                    out: flight_recorder_args.out,
                    markdown: flight_recorder_args.markdown,
                    no_markdown: flight_recorder_args.no_markdown,
                    json: flight_recorder_args.format.is_json(),
                })
            }
            AutopilotCommand::Navigator(navigator_args) => {
                cli::autopilot::navigator(cli::autopilot::NavigatorOptions {
                    prompt: if navigator_args.prompt.is_empty() {
                        None
                    } else {
                        Some(navigator_args.prompt.join(" "))
                    },
                    run_dir: navigator_args.run_dir,
                    root: navigator_args.root,
                    limit: navigator_args.limit,
                    out: navigator_args.out,
                    markdown: navigator_args.markdown,
                    no_markdown: navigator_args.no_markdown,
                    json: navigator_args.format.is_json(),
                })
            }
            AutopilotCommand::Advance(advance_args) => {
                cli::autopilot::advance(cli::autopilot::AdvanceOptions {
                    run_dir: advance_args.run_dir,
                    root: advance_args.root,
                    limit: advance_args.limit,
                    dry_run: advance_args.dry_run,
                    out: advance_args.out,
                    markdown: advance_args.markdown,
                    no_markdown: advance_args.no_markdown,
                    json: advance_args.format.is_json(),
                })
            }
            AutopilotCommand::Act(act_args) => cli::autopilot::act(cli::autopilot::ActOptions {
                run_dir: act_args.run_dir,
                command: act_args.command,
                source: act_args.source,
                root: act_args.root,
                limit: act_args.limit,
                dry_run: act_args.dry_run,
                out: act_args.out,
                markdown: act_args.markdown,
                no_markdown: act_args.no_markdown,
                json: act_args.format.is_json(),
            }),
            AutopilotCommand::Loop(loop_args) => {
                cli::autopilot::run_loop(cli::autopilot::LoopOptions {
                    run_dir: loop_args.run_dir,
                    root: loop_args.root,
                    limit: loop_args.limit,
                    max_steps: loop_args.max_steps,
                    dry_run: loop_args.dry_run,
                    out: loop_args.out,
                    markdown: loop_args.markdown,
                    no_markdown: loop_args.no_markdown,
                    json: loop_args.format.is_json(),
                })
            }
            AutopilotCommand::Roadmap(roadmap_args) => {
                cli::autopilot::roadmap(cli::autopilot::RoadmapOptions {
                    prompt: if roadmap_args.prompt.is_empty() {
                        None
                    } else {
                        Some(roadmap_args.prompt.join(" "))
                    },
                    root: roadmap_args.root,
                    limit: roadmap_args.limit,
                    out: roadmap_args.out,
                    markdown: roadmap_args.markdown,
                    no_markdown: roadmap_args.no_markdown,
                    json: roadmap_args.format.is_json(),
                })
            }
            AutopilotCommand::Judge(judge_args) => {
                cli::autopilot::judge(cli::autopilot::JudgeOptions {
                    run_dir: judge_args.run_dir,
                    out: judge_args.out,
                    markdown: judge_args.markdown,
                    no_markdown: judge_args.no_markdown,
                    json: judge_args.format.is_json(),
                })
            }
            AutopilotCommand::Critique(critique_args) => {
                cli::autopilot::critique(cli::autopilot::CritiqueOptions {
                    run_dir: critique_args.run_dir,
                    plan: critique_args.plan,
                    out: critique_args.out,
                    markdown: critique_args.markdown,
                    no_markdown: critique_args.no_markdown,
                    json: critique_args.format.is_json(),
                })
            }
            AutopilotCommand::Playtest(playtest_args) => {
                cli::autopilot::playtest(cli::autopilot::PlaytestOptions {
                    run_dir: playtest_args.run_dir,
                    plan: playtest_args.plan,
                    out: playtest_args.out,
                    markdown: playtest_args.markdown,
                    no_markdown: playtest_args.no_markdown,
                    json: playtest_args.format.is_json(),
                })
            }
            AutopilotCommand::Simulate(simulate_args) => {
                cli::autopilot::simulate(cli::autopilot::SimulateOptions {
                    run_dir: simulate_args.run_dir,
                    plan: simulate_args.plan,
                    out: simulate_args.out,
                    markdown: simulate_args.markdown,
                    no_markdown: simulate_args.no_markdown,
                    json: simulate_args.format.is_json(),
                })
            }
            AutopilotCommand::Graph(graph_args) => {
                cli::autopilot::graph(cli::autopilot::GraphOptions {
                    run_dir: graph_args.run_dir,
                    plan: graph_args.plan,
                    out: graph_args.out,
                    markdown: graph_args.markdown,
                    no_markdown: graph_args.no_markdown,
                    json: graph_args.format.is_json(),
                })
            }
            AutopilotCommand::Balance(balance_args) => {
                cli::autopilot::balance(cli::autopilot::BalanceOptions {
                    run_dir: balance_args.run_dir,
                    plan: balance_args.plan,
                    out: balance_args.out,
                    markdown: balance_args.markdown,
                    no_markdown: balance_args.no_markdown,
                    json: balance_args.format.is_json(),
                })
            }
            AutopilotCommand::Impact(impact_args) => {
                cli::autopilot::impact(cli::autopilot::ImpactOptions {
                    run_dir: impact_args.run_dir,
                    plan: impact_args.plan,
                    out: impact_args.out,
                    markdown: impact_args.markdown,
                    no_markdown: impact_args.no_markdown,
                    json: impact_args.format.is_json(),
                })
            }
            AutopilotCommand::Contracts(contracts_args) => {
                cli::autopilot::contracts(cli::autopilot::ContractsOptions {
                    run_dir: contracts_args.run_dir,
                    plan: contracts_args.plan,
                    out: contracts_args.out,
                    markdown: contracts_args.markdown,
                    no_markdown: contracts_args.no_markdown,
                    json: contracts_args.format.is_json(),
                })
            }
            AutopilotCommand::Authority(authority_args) => {
                cli::autopilot::authority(cli::autopilot::AuthorityOptions {
                    run_dir: authority_args.run_dir,
                    plan: authority_args.plan,
                    out: authority_args.out,
                    markdown: authority_args.markdown,
                    no_markdown: authority_args.no_markdown,
                    json: authority_args.format.is_json(),
                })
            }
            AutopilotCommand::Ux(ux_args) => cli::autopilot::ux(cli::autopilot::UxOptions {
                run_dir: ux_args.run_dir,
                plan: ux_args.plan,
                out: ux_args.out,
                markdown: ux_args.markdown,
                no_markdown: ux_args.no_markdown,
                json: ux_args.format.is_json(),
            }),
            AutopilotCommand::CopyDeck(copy_args) => {
                cli::autopilot::copy_deck(cli::autopilot::CopyDeckOptions {
                    run_dir: copy_args.run_dir,
                    plan: copy_args.plan,
                    out: copy_args.out,
                    markdown: copy_args.markdown,
                    no_markdown: copy_args.no_markdown,
                    json: copy_args.format.is_json(),
                })
            }
            AutopilotCommand::Performance(performance_args) => {
                cli::autopilot::performance(cli::autopilot::PerformanceOptions {
                    run_dir: performance_args.run_dir,
                    plan: performance_args.plan,
                    out: performance_args.out,
                    markdown: performance_args.markdown,
                    no_markdown: performance_args.no_markdown,
                    json: performance_args.format.is_json(),
                })
            }
            AutopilotCommand::Accessibility(accessibility_args) => {
                cli::autopilot::accessibility(cli::autopilot::AccessibilityOptions {
                    run_dir: accessibility_args.run_dir,
                    plan: accessibility_args.plan,
                    out: accessibility_args.out,
                    markdown: accessibility_args.markdown,
                    no_markdown: accessibility_args.no_markdown,
                    json: accessibility_args.format.is_json(),
                })
            }
            AutopilotCommand::Policy(policy_args) => {
                cli::autopilot::policy(cli::autopilot::PolicyOptions {
                    run_dir: policy_args.run_dir,
                    plan: policy_args.plan,
                    out: policy_args.out,
                    markdown: policy_args.markdown,
                    no_markdown: policy_args.no_markdown,
                    json: policy_args.format.is_json(),
                })
            }
            AutopilotCommand::AssetBrief(asset_args) => {
                cli::autopilot::asset_brief(cli::autopilot::AssetBriefOptions {
                    run_dir: asset_args.run_dir,
                    plan: asset_args.plan,
                    out: asset_args.out,
                    markdown: asset_args.markdown,
                    no_markdown: asset_args.no_markdown,
                    json: asset_args.format.is_json(),
                })
            }
            AutopilotCommand::StyleGuide(style_args) => {
                cli::autopilot::style_guide(cli::autopilot::StyleGuideOptions {
                    run_dir: style_args.run_dir,
                    plan: style_args.plan,
                    out: style_args.out,
                    markdown: style_args.markdown,
                    no_markdown: style_args.no_markdown,
                    json: style_args.format.is_json(),
                })
            }
            AutopilotCommand::WorldBlueprint(world_args) => {
                cli::autopilot::world_blueprint(cli::autopilot::WorldBlueprintOptions {
                    run_dir: world_args.run_dir,
                    plan: world_args.plan,
                    out: world_args.out,
                    markdown: world_args.markdown,
                    no_markdown: world_args.no_markdown,
                    json: world_args.format.is_json(),
                })
            }
            AutopilotCommand::Onboarding(onboarding_args) => {
                cli::autopilot::onboarding(cli::autopilot::OnboardingOptions {
                    run_dir: onboarding_args.run_dir,
                    plan: onboarding_args.plan,
                    out: onboarding_args.out,
                    markdown: onboarding_args.markdown,
                    no_markdown: onboarding_args.no_markdown,
                    json: onboarding_args.format.is_json(),
                })
            }
            AutopilotCommand::Showcase(showcase_args) => {
                cli::autopilot::showcase(cli::autopilot::ShowcaseOptions {
                    run_dir: showcase_args.run_dir,
                    plan: showcase_args.plan,
                    out: showcase_args.out,
                    markdown: showcase_args.markdown,
                    no_markdown: showcase_args.no_markdown,
                    json: showcase_args.format.is_json(),
                })
            }
            AutopilotCommand::Telemetry(telemetry_args) => {
                cli::autopilot::telemetry(cli::autopilot::TelemetryOptions {
                    run_dir: telemetry_args.run_dir,
                    plan: telemetry_args.plan,
                    out: telemetry_args.out,
                    markdown: telemetry_args.markdown,
                    no_markdown: telemetry_args.no_markdown,
                    json: telemetry_args.format.is_json(),
                })
            }
            AutopilotCommand::Monetization(monetization_args) => {
                cli::autopilot::monetization(cli::autopilot::MonetizationOptions {
                    run_dir: monetization_args.run_dir,
                    plan: monetization_args.plan,
                    out: monetization_args.out,
                    markdown: monetization_args.markdown,
                    no_markdown: monetization_args.no_markdown,
                    json: monetization_args.format.is_json(),
                })
            }
            AutopilotCommand::Social(social_args) => {
                cli::autopilot::social(cli::autopilot::SocialOptions {
                    run_dir: social_args.run_dir,
                    plan: social_args.plan,
                    out: social_args.out,
                    markdown: social_args.markdown,
                    no_markdown: social_args.no_markdown,
                    json: social_args.format.is_json(),
                })
            }
            AutopilotCommand::Liveops(liveops_args) => {
                cli::autopilot::liveops(cli::autopilot::LiveopsOptions {
                    run_dir: liveops_args.run_dir,
                    plan: liveops_args.plan,
                    out: liveops_args.out,
                    markdown: liveops_args.markdown,
                    no_markdown: liveops_args.no_markdown,
                    json: liveops_args.format.is_json(),
                })
            }
            AutopilotCommand::Persistence(persistence_args) => {
                cli::autopilot::persistence(cli::autopilot::PersistenceOptions {
                    run_dir: persistence_args.run_dir,
                    plan: persistence_args.plan,
                    out: persistence_args.out,
                    markdown: persistence_args.markdown,
                    no_markdown: persistence_args.no_markdown,
                    json: persistence_args.format.is_json(),
                })
            }
            AutopilotCommand::Evidence(evidence_args) => {
                cli::autopilot::evidence(cli::autopilot::EvidenceOptions {
                    run_dir: evidence_args.run_dir,
                    evidence_dir: evidence_args.evidence_dir,
                    out: evidence_args.out,
                    markdown: evidence_args.markdown,
                    no_markdown: evidence_args.no_markdown,
                    create_dirs: !evidence_args.no_create_dirs,
                    json: evidence_args.format.is_json(),
                })
            }
            AutopilotCommand::RecordPlaytest(record_args) => {
                cli::autopilot::record_playtest(cli::autopilot::RecordPlaytestOptions {
                    run_dir: record_args.run_dir,
                    result: record_args.result,
                    evidence: record_args.evidence,
                    notes: record_args.note,
                    scenarios: record_args.scenario,
                    out: record_args.out,
                    markdown: record_args.markdown,
                    no_markdown: record_args.no_markdown,
                    json: record_args.format.is_json(),
                })
            }
            AutopilotCommand::EvidenceReview(review_args) => {
                cli::autopilot::evidence_review(cli::autopilot::EvidenceReviewOptions {
                    run_dir: review_args.run_dir,
                    out: review_args.out,
                    markdown: review_args.markdown,
                    no_markdown: review_args.no_markdown,
                    json: review_args.format.is_json(),
                })
            }
            AutopilotCommand::Health(health_args) => {
                cli::autopilot::health(cli::autopilot::HealthOptions {
                    run_dir: health_args.run_dir,
                    out: health_args.out,
                    markdown: health_args.markdown,
                    no_markdown: health_args.no_markdown,
                    json: health_args.format.is_json(),
                })
            }
            AutopilotCommand::RepairPlan(repair_args) => {
                cli::autopilot::repair_plan(cli::autopilot::RepairPlanOptions {
                    run_dir: repair_args.run_dir,
                    out: repair_args.out,
                    markdown: repair_args.markdown,
                    no_markdown: repair_args.no_markdown,
                    json: repair_args.format.is_json(),
                })
            }
            AutopilotCommand::Improve(improve_args) => {
                cli::autopilot::improve(cli::autopilot::ImproveOptions {
                    port: args.port,
                    run_dir: improve_args.run_dir,
                    plan: improve_args.plan,
                    out: improve_args.out,
                    markdown: improve_args.markdown,
                    no_markdown: improve_args.no_markdown,
                    recipes: improve_args.recipe,
                    max_recipes: improve_args.max_recipes,
                    smoke: improve_args.smoke,
                    json: improve_args.format.is_json(),
                })
            }
            AutopilotCommand::Compare(compare_args) => {
                cli::autopilot::compare(cli::autopilot::CompareOptions {
                    base_run: compare_args.base_run,
                    candidate_run: compare_args.candidate_run,
                    out: compare_args.out,
                    markdown: compare_args.markdown,
                    no_markdown: compare_args.no_markdown,
                    json: compare_args.format.is_json(),
                })
            }
            AutopilotCommand::Iterate(iterate_args) => {
                cli::autopilot::iterate(cli::autopilot::IterateOptions {
                    port: args.port,
                    run_dir: iterate_args.run_dir,
                    out: iterate_args.out,
                    markdown: iterate_args.markdown,
                    no_markdown: iterate_args.no_markdown,
                    max_steps: iterate_args.max_steps,
                    max_recipes: iterate_args.max_recipes,
                    smoke: iterate_args.smoke,
                    json: iterate_args.format.is_json(),
                })
            }
            AutopilotCommand::Sequence(sequence_args) => {
                cli::autopilot::sequence(cli::autopilot::SequenceOptions {
                    run_dirs: sequence_args.run_dirs,
                    out: sequence_args.out,
                    markdown: sequence_args.markdown,
                    no_markdown: sequence_args.no_markdown,
                    json: sequence_args.format.is_json(),
                })
            }
            AutopilotCommand::Architect(architect_args) => {
                cli::autopilot::architect(cli::autopilot::ArchitectOptions {
                    prompt: if architect_args.prompt.is_empty() {
                        None
                    } else {
                        Some(architect_args.prompt.join(" "))
                    },
                    root: architect_args.root,
                    limit: architect_args.limit,
                    out: architect_args.out,
                    markdown: architect_args.markdown,
                    no_markdown: architect_args.no_markdown,
                    smoke: architect_args.smoke,
                    json: architect_args.format.is_json(),
                })
            }
            AutopilotCommand::Kickoff(kickoff_args) => {
                cli::autopilot::kickoff(cli::autopilot::KickoffOptions {
                    port: args.port,
                    prompt: if kickoff_args.prompt.is_empty() {
                        None
                    } else {
                        Some(kickoff_args.prompt.join(" "))
                    },
                    studio: kickoff_args.studio,
                    scope: kickoff_args.scope,
                    root: kickoff_args.root,
                    limit: kickoff_args.limit,
                    out: kickoff_args.out,
                    smoke: kickoff_args.smoke,
                    json: kickoff_args.format.is_json(),
                })
            }
            AutopilotCommand::AuditSources(audit_args) => {
                cli::autopilot::audit_sources(cli::autopilot::AuditSourcesOptions {
                    run_dir: audit_args.run_dir,
                    plan: audit_args.plan,
                    out: audit_args.out,
                    markdown: audit_args.markdown,
                    no_markdown: audit_args.no_markdown,
                    json: audit_args.format.is_json(),
                })
            }
            AutopilotCommand::PlannerPack(planner_pack_args) => {
                cli::autopilot::planner_pack(cli::autopilot::PlannerPackOptions {
                    prompt: if planner_pack_args.prompt.is_empty() {
                        None
                    } else {
                        Some(planner_pack_args.prompt.join(" "))
                    },
                    run_dir: planner_pack_args.run_dir,
                    context: planner_pack_args.context,
                    out: planner_pack_args.out,
                    markdown: planner_pack_args.markdown,
                    no_markdown: planner_pack_args.no_markdown,
                    json: planner_pack_args.format.is_json(),
                })
            }
            AutopilotCommand::AdoptPlan(adopt_args) => {
                cli::autopilot::adopt_plan(cli::autopilot::AdoptPlanOptions {
                    plan: adopt_args.plan,
                    source_root: adopt_args.source_root,
                    context: adopt_args.context,
                    out: adopt_args.out,
                    json: adopt_args.format.is_json(),
                })
            }
            AutopilotCommand::Certify(certify_args) => {
                cli::autopilot::certify(cli::autopilot::CertifyOptions {
                    run_dir: certify_args.run_dir,
                    out: certify_args.out,
                    markdown: certify_args.markdown,
                    no_markdown: certify_args.no_markdown,
                    json: certify_args.format.is_json(),
                })
            }
            AutopilotCommand::Bundle(bundle_args) => {
                cli::autopilot::bundle(cli::autopilot::BundleOptions {
                    run_dir: bundle_args.run_dir,
                    out: bundle_args.out,
                    json: bundle_args.format.is_json(),
                })
            }
            AutopilotCommand::VerifyBundle(verify_args) => {
                cli::autopilot::verify_bundle(cli::autopilot::VerifyBundleOptions {
                    bundle: verify_args.bundle,
                    run_dir: verify_args.run_dir,
                    json: verify_args.format.is_json(),
                })
            }
            AutopilotCommand::Setup(setup_args) => {
                cli::autopilot::setup(cli::autopilot::SetupOptions {
                    port: args.port,
                    studio: setup_args.studio,
                    fix: setup_args.fix,
                    timeout_secs: setup_args.timeout,
                    poll_ms: setup_args.poll_ms,
                    required_capabilities: setup_args.require_capability,
                    out: setup_args.out,
                    markdown: setup_args.markdown,
                    no_markdown: setup_args.no_markdown,
                    json: setup_args.format.is_json(),
                })
            }
            AutopilotCommand::Ready(ready_args) => {
                cli::autopilot::ready(cli::autopilot::ReadyOptions {
                    port: args.port,
                    studio: ready_args.studio,
                    timeout_secs: ready_args.timeout,
                    poll_ms: ready_args.poll_ms,
                    required_capabilities: ready_args.require_capability,
                    json: ready_args.format.is_json(),
                })
            }
            AutopilotCommand::LiveGate(gate_args) => {
                cli::autopilot::live_gate(cli::autopilot::LiveGateOptions {
                    port: args.port,
                    run_dir: gate_args.run_dir,
                    session: gate_args.session,
                    studio: gate_args.studio,
                    approved: gate_args.approved,
                    skip_ready: gate_args.skip_ready,
                    timeout_secs: gate_args.timeout,
                    poll_ms: gate_args.poll_ms,
                    required_capabilities: gate_args.require_capability,
                    out: gate_args.out,
                    markdown: gate_args.markdown,
                    no_markdown: gate_args.no_markdown,
                    json: gate_args.format.is_json(),
                })
            }
            AutopilotCommand::Rehearsal(rehearsal_args) => {
                cli::autopilot::rehearsal(cli::autopilot::RehearsalOptions {
                    run_dir: rehearsal_args.run_dir,
                    out: rehearsal_args.out,
                    markdown: rehearsal_args.markdown,
                    no_markdown: rehearsal_args.no_markdown,
                    json: rehearsal_args.format.is_json(),
                })
            }
            AutopilotCommand::Closeout(closeout_args) => {
                cli::autopilot::closeout(cli::autopilot::CloseoutOptions {
                    run_dir: closeout_args.run_dir,
                    out: closeout_args.out,
                    markdown: closeout_args.markdown,
                    no_markdown: closeout_args.no_markdown,
                    json: closeout_args.format.is_json(),
                })
            }
            AutopilotCommand::Timeline(timeline_args) => {
                cli::autopilot::timeline(cli::autopilot::TimelineOptions {
                    run_dir: timeline_args.run_dir,
                    out: timeline_args.out,
                    markdown: timeline_args.markdown,
                    no_markdown: timeline_args.no_markdown,
                    json: timeline_args.format.is_json(),
                })
            }
            AutopilotCommand::Run(run_args) => cli::autopilot::run(cli::autopilot::RunOptions {
                port: args.port,
                prompt: if run_args.prompt.is_empty() {
                    None
                } else {
                    Some(run_args.prompt.join(" "))
                },
                studio: run_args.studio,
                scope: run_args.scope,
                out: run_args.out,
                recipe: run_args.recipe,
                from_manifest: run_args.from_manifest,
                yes: run_args.yes,
                validate: run_args.validate,
                rollback_on_error: run_args.rollback_on_error,
                force: run_args.force,
                only: run_args.only,
                exclude: run_args.exclude,
                smoke: run_args.smoke,
            }),
            AutopilotCommand::Explain(explain_args) => {
                cli::autopilot::explain(cli::autopilot::ExplainOptions {
                    plan: explain_args.plan,
                    json: explain_args.format.is_json(),
                })
            }
            AutopilotCommand::Plan(plan_args) => {
                cli::autopilot::plan(cli::autopilot::PlanOptions {
                    port: args.port,
                    prompt: if plan_args.prompt.is_empty() {
                        None
                    } else {
                        Some(plan_args.prompt.join(" "))
                    },
                    studio: plan_args.studio,
                    scope: plan_args.scope,
                    out: plan_args.out,
                    max_read_depth: plan_args.max_read_depth,
                    include_scripts: plan_args.include_scripts,
                    include_assets: plan_args.include_assets,
                    recipe: plan_args.recipe,
                    from_manifest: plan_args.from_manifest,
                    json: plan_args.format.is_json(),
                })
            }
            AutopilotCommand::Context(context_args) => {
                cli::autopilot::context(cli::autopilot::ContextOptions {
                    port: args.port,
                    studio: context_args.studio,
                    path: context_args.path,
                    out: context_args.out,
                    include_paths: context_args.include_paths,
                    include_read: context_args.include_read,
                    read_depth: context_args.read_depth,
                    json: context_args.format.is_json(),
                })
            }
            AutopilotCommand::Survey(survey_args) => {
                cli::autopilot::survey(cli::autopilot::SurveyOptions {
                    port: args.port,
                    studio: survey_args.studio,
                    path: survey_args.path,
                    context: survey_args.context,
                    out: survey_args.out,
                    markdown: survey_args.markdown,
                    no_markdown: survey_args.no_markdown,
                    include_paths: survey_args.include_paths,
                    include_read: survey_args.include_read,
                    read_depth: survey_args.read_depth,
                    json: survey_args.format.is_json(),
                })
            }
            AutopilotCommand::Reconcile(reconcile_args) => {
                cli::autopilot::reconcile(cli::autopilot::ReconcileOptions {
                    run_dir: reconcile_args.run_dir,
                    survey: reconcile_args.survey,
                    context: reconcile_args.context,
                    out: reconcile_args.out,
                    markdown: reconcile_args.markdown,
                    no_markdown: reconcile_args.no_markdown,
                    json: reconcile_args.format.is_json(),
                })
            }
            AutopilotCommand::Scout(scout_args) => {
                cli::autopilot::scout(cli::autopilot::ScoutOptions {
                    prompt: if scout_args.prompt.is_empty() {
                        None
                    } else {
                        Some(scout_args.prompt.join(" "))
                    },
                    survey: scout_args.survey,
                    context: scout_args.context,
                    root: scout_args.root,
                    limit: scout_args.limit,
                    scope: scout_args.scope,
                    out: scout_args.out,
                    markdown: scout_args.markdown,
                    no_markdown: scout_args.no_markdown,
                    json: scout_args.format.is_json(),
                })
            }
            AutopilotCommand::Session(session_args) => {
                cli::autopilot::session(cli::autopilot::SessionOptions {
                    port: args.port,
                    prompt: if session_args.prompt.is_empty() {
                        None
                    } else {
                        Some(session_args.prompt.join(" "))
                    },
                    scout: session_args.scout,
                    survey: session_args.survey,
                    context: session_args.context,
                    root: session_args.root,
                    run_dir: session_args.run_dir,
                    scope: session_args.scope,
                    limit: session_args.limit,
                    assume: session_args.assume,
                    smoke: session_args.smoke,
                    out: session_args.out,
                    markdown: session_args.markdown,
                    no_markdown: session_args.no_markdown,
                    json: session_args.format.is_json(),
                })
            }
            AutopilotCommand::Preview(preview_args) => {
                cli::autopilot::preview(cli::autopilot::PreviewOptions {
                    port: args.port,
                    studio: preview_args.studio,
                    plan: preview_args.plan,
                    out: preview_args.out,
                    live: preview_args.live,
                    force: preview_args.force,
                    only: preview_args.only,
                    exclude: preview_args.exclude,
                    json: preview_args.format.is_json(),
                })
            }
            AutopilotCommand::Apply(apply_args) => {
                cli::autopilot::apply(cli::autopilot::ApplyOptions {
                    port: args.port,
                    studio: apply_args.studio,
                    plan: apply_args.plan,
                    out: apply_args.out,
                    yes: apply_args.yes,
                    validate: apply_args.validate,
                    rollback_on_error: apply_args.rollback_on_error,
                    force: apply_args.force,
                    only: apply_args.only,
                    exclude: apply_args.exclude,
                    smoke: apply_args.smoke,
                    json: apply_args.format.is_json(),
                })
            }
            AutopilotCommand::Report(report_args) => {
                cli::autopilot::report(cli::autopilot::ReportOptions {
                    run_dir: report_args.run_dir,
                    json: report_args.format.is_json(),
                })
            }
        },
        Command::Sync { command } => match command {
            SyncCommand::Pull {
                studio,
                path,
                out,
                depth,
                overwrite,
                format,
            } => cli::sync_pull::run(
                args.port,
                studio,
                path,
                out,
                depth,
                overwrite,
                format.is_json(),
            ),
        },
        Command::SyncFolder {
            studio,
            folder,
            to,
            manifest,
            watch,
            dry_run,
            delete,
            force,
        } => cli::sync_folder::run(
            args.port, studio, folder, to, manifest, watch, dry_run, delete, force,
        ),
        Command::Batch {
            file,
            dry_run,
            continue_on_error,
        } => cli::batch::run(args.port, file, dry_run, continue_on_error),
        Command::Package {
            studio,
            path,
            out,
            depth,
            overwrite,
            command,
        } => match command {
            Some(PackageCommand::Inspect { file, format }) => {
                cli::package::inspect_run(file, format.is_json())
            }
            Some(PackageCommand::Import {
                studio,
                file,
                to,
                if_exists,
                dry_run,
                rollback_on_error,
                image_rehost,
                format,
            }) => cli::package::import_run(
                args.port,
                studio,
                file,
                to,
                if_exists.as_str().to_string(),
                dry_run,
                rollback_on_error,
                image_rehost_options(image_rehost, dry_run, format.is_json()),
                format.is_json(),
            ),
            Some(PackageCommand::Update {
                studio,
                file,
                to,
                owned_only,
                preserve_local,
                replace_owned,
                conflict_report,
                dry_run,
                force,
                format,
            }) => cli::package::update_run(
                args.port,
                studio,
                file,
                to,
                cli::package::PackageUpdateFlags {
                    owned_only,
                    preserve_local,
                    replace_owned,
                    conflict_report,
                    dry_run,
                    force,
                    json: format.is_json(),
                },
            ),
            Some(PackageCommand::Verify {
                file,
                studio,
                to,
                if_exists,
                format,
            }) => cli::package::verify_run(
                args.port,
                studio,
                file,
                to,
                if_exists.as_str().to_string(),
                format.is_json(),
            ),
            Some(PackageCommand::Pack { file, out }) => cli::package::pack_run(file, out),
            Some(PackageCommand::Unpack {
                file,
                out,
                overwrite,
            }) => cli::package::unpack_run(file, out, overwrite),
            None => cli::package::export_run(
                args.port,
                studio,
                path.ok_or_else(|| {
                    AppError::Other("--path is required for package export".into())
                })?,
                out.ok_or_else(|| AppError::Other("--out is required for package export".into()))?,
                depth,
                overwrite,
            ),
        },
        Command::Transaction { command } => match command {
            TransactionCommand::Snapshot {
                studio,
                path,
                out,
                format,
            } => cli::transaction::snapshot_run(args.port, studio, path, out, format.is_json()),
            TransactionCommand::Restore {
                studio,
                file,
                to,
                if_exists,
                format,
            } => cli::transaction::restore_run(
                args.port,
                studio,
                file,
                to,
                if_exists.as_str().to_string(),
                format.is_json(),
            ),
        },
        Command::History {
            studio,
            command,
            format,
        } => match command {
            Some(HistoryCommand::Show { id }) => {
                cli::history::show(args.port, studio, id, format.is_json())
            }
            None => cli::history::list(args.port, studio, format.is_json()),
        },
        Command::Undo {
            studio,
            id,
            yes,
            format,
        } => cli::history::undo(args.port, studio, id, yes, format.is_json()),
        Command::Deps {
            studio,
            path,
            out,
            format,
        } => cli::deps::run(args.port, studio, path, out, format.is_json()),
        Command::PublishCheck {
            studio,
            path,
            package_path,
            format,
        } => cli::publish_check::run(args.port, studio, path, package_path, format.is_json()),
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

impl OutputFormat {
    fn is_json(&self) -> bool {
        matches!(self, OutputFormat::Json)
    }
}

impl OnOff {
    fn as_bool(&self) -> bool {
        matches!(self, OnOff::On)
    }
}

impl IfExists {
    fn as_str(&self) -> &'static str {
        match self {
            IfExists::Fail => "fail",
            IfExists::Replace => "replace",
            IfExists::Merge => "merge",
            IfExists::Rename => "rename",
        }
    }
}

impl CreatorType {
    fn as_str(&self) -> &'static str {
        match self {
            CreatorType::Group => "group",
            CreatorType::User => "user",
        }
    }
}

impl PlanChangeKind {
    fn as_str(&self) -> &'static str {
        match self {
            PlanChangeKind::Added => "added",
            PlanChangeKind::Modified => "modified",
            PlanChangeKind::Deleted => "deleted",
            PlanChangeKind::Reference => "reference",
        }
    }
}

fn image_rehost_options(
    args: ImageRehostArgs,
    dry_run: bool,
    quiet: bool,
) -> Option<cli::rehost_images::ImageRehostOptions> {
    args.rehost_images
        .then(|| cli::rehost_images::ImageRehostOptions {
            creator_id: args.creator_id,
            creator_type: args.creator_type.map(|value| value.as_str().to_string()),
            profile: args.profile,
            api_key: args.api_key,
            source_api_key: args.source_api_key,
            wait_timeout_secs: args.rehost_timeout,
            dry_run,
            quiet,
        })
}

fn run_upload(port: u16, kind: cli::upload::UploadKind, args: UploadAssetArgs) -> AppResult<()> {
    cli::upload::run(cli::upload::UploadOptions {
        port,
        studio: args.studio,
        kind,
        file: args.file,
        creator_id: args.creator_id,
        creator_type: args.creator_type.map(|value| value.as_str().to_string()),
        profile: args.profile,
        name: args.name,
        description: args.description,
        api_key: args.api_key,
        wait: args.wait,
        wait_timeout_secs: args.wait_timeout,
        import_to: args.import_to,
        json_output: args.format.is_json(),
    })
}
