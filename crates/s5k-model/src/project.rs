//! The structural project model produced by input readers (milestone M1).
//!
//! Everything lives in flat arenas (`Vec`s) on [`Project`] and refers to other
//! items through typed IDs, so the model is `Send + Sync`, cheap to serialize
//! and safe to analyse in parallel. Routine bodies are kept as raw text or raw
//! element trees here; `s5k-logic` parses them.

use serde::{Deserialize, Serialize};

use crate::{ExportHeader, Name};

macro_rules! id_type {
    ($(#[$m:meta])* $name:ident) => {
        $(#[$m])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
        pub struct $name(pub u32);

        impl $name {
            /// Index into the owning arena.
            pub fn index(self) -> usize {
                self.0 as usize
            }
        }
    };
}

id_type!(
    /// Index into [`Project::data_types`].
    DataTypeId
);
id_type!(
    /// Index into [`Project::aois`].
    AoiId
);
id_type!(
    /// Index into [`Project::tags`].
    TagId
);
id_type!(
    /// Index into [`Project::programs`].
    ProgramId
);
id_type!(
    /// Index into [`Project::routines`].
    RoutineId
);
id_type!(
    /// Index into [`Project::tasks`].
    TaskId
);
id_type!(
    /// Index into [`Project::modules`].
    ModuleId
);

/// A whole (or partially exported) Logix project.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Project {
    /// Root element metadata, if the reader found it.
    pub header: Option<ExportHeader>,
    /// Controller attributes.
    pub controller: Option<ControllerInfo>,
    /// User-defined types, in file order.
    pub data_types: Vec<DataType>,
    /// Add-On Instruction definitions, in file order.
    pub aois: Vec<AoiDefinition>,
    /// Tags of every scope. [`Tag::scope`] says where each is declared.
    pub tags: Vec<Tag>,
    /// Programs, in file order.
    pub programs: Vec<Program>,
    /// Routines of programs and AOIs. [`Routine::owner`] says which.
    pub routines: Vec<Routine>,
    /// Tasks, in file order.
    pub tasks: Vec<Task>,
    /// I/O modules, in file order.
    pub modules: Vec<Module>,
    /// Program parameter connections (v24+), from controller `ParameterConnections`.
    pub parameter_connections: Vec<ParameterConnection>,
}

/// A connection between two program parameters, e.g. `\Sequencer.FillCmd` ↔ `\Valves.Open`.
/// These are cross-program data paths that the output trace must follow (ADR-0002).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterConnection {
    /// `EndPoint1` operand text.
    pub end_point1: String,
    /// `EndPoint2` operand text.
    pub end_point2: String,
}

impl Project {
    /// Tags declared in a given scope, in file order.
    pub fn tags_in(&self, scope: Scope) -> impl Iterator<Item = (TagId, &Tag)> {
        self.tags
            .iter()
            .enumerate()
            .filter(move |(_, t)| t.scope == scope)
            .map(|(i, t)| (TagId(i as u32), t))
    }

    /// Look up a program by name (case-insensitive).
    pub fn program_by_name(&self, name: &str) -> Option<(ProgramId, &Program)> {
        let key = Name::new(name);
        self.programs
            .iter()
            .enumerate()
            .find(|(_, p)| p.name == key)
            .map(|(i, p)| (ProgramId(i as u32), p))
    }

    /// Look up a routine by owner and name (case-insensitive).
    pub fn routine_by_name(
        &self,
        owner: RoutineOwner,
        name: &str,
    ) -> Option<(RoutineId, &Routine)> {
        let key = Name::new(name);
        self.routines
            .iter()
            .enumerate()
            .find(|(_, r)| r.owner == owner && r.name == key)
            .map(|(i, r)| (RoutineId(i as u32), r))
    }
}

/// Whether an element is the subject of a partial export or only context for it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ExportUse {
    /// No `Use` attribute (normal full-project export).
    #[default]
    Unspecified,
    /// `Use="Target"`: the exported item.
    Target,
    /// `Use="Context"`: included only so the target can be understood.
    Context,
}

/// Controller attributes from the `Controller` element.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ControllerInfo {
    /// Controller name.
    pub name: Name,
    /// Catalog number, e.g. `1756-L85E`.
    pub processor_type: Option<String>,
    /// Firmware major revision.
    pub major_rev: Option<u32>,
    /// Firmware minor revision.
    pub minor_rev: Option<u32>,
    /// Description text, if any.
    pub description: Option<String>,
    /// Whether this is the export target or context.
    pub export_use: ExportUse,
}

// ---------------------------------------------------------------- data types

/// A user-defined type (`DataType` element).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataType {
    /// Type name.
    pub name: Name,
    /// `Family`: `NoFamily` or `StringFamily`.
    pub family: Option<String>,
    /// `Class`: `User`, `ProductDefined`, `IO` ...
    pub class: Option<String>,
    /// Description text.
    pub description: Option<String>,
    /// Members, including hidden BOOL host members.
    pub members: Vec<Member>,
    /// Partial-export role.
    pub export_use: ExportUse,
}

/// A member of a UDT.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Member {
    /// Member name.
    pub name: Name,
    /// Declared type name as written (`BIT` for packed BOOLs).
    pub data_type: Name,
    /// Array length; 0 means scalar.
    pub dimension: u32,
    /// Hidden members are BOOL host `SINT`s generated by Studio 5000.
    pub hidden: bool,
    /// For packed BOOL members: host member and bit number.
    pub bit_of: Option<BitTarget>,
    /// `ExternalAccess` attribute.
    pub external_access: Option<String>,
    /// Description text.
    pub description: Option<String>,
}

/// Location of a packed BOOL member inside its hidden host member.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BitTarget {
    /// Name of the hidden host member.
    pub host: Name,
    /// Bit number within the host.
    pub bit: u8,
}

// ---------------------------------------------------------------- AOIs

/// An Add-On Instruction definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AoiDefinition {
    /// AOI name; also the data type name of its instances.
    pub name: Name,
    /// `Revision`, e.g. `1.0`.
    pub revision: Option<String>,
    /// Description text.
    pub description: Option<String>,
    /// Parameters, in declaration order (including `EnableIn`/`EnableOut`).
    pub parameters: Vec<AoiParameter>,
    /// Routines (`Logic`, `Prescan`, `Postscan`, `EnableInFalse`).
    /// Local tags are stored in [`Project::tags`] with [`Scope::Aoi`].
    pub routines: Vec<RoutineId>,
    /// Names of AOIs this AOI depends on (from `Dependencies`).
    pub dependencies: Vec<Name>,
    /// Logic is encrypted (`EncodedData` / source protection).
    pub source_protected: bool,
    /// Partial-export role.
    pub export_use: ExportUse,
}

/// Direction of an AOI parameter or program parameter.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ParameterUsage {
    /// Copied in at call time.
    Input,
    /// Copied out after the call.
    Output,
    /// Passed by reference: writes inside affect the argument directly.
    InOut,
    /// Program parameter readable from other programs.
    Public,
    /// Unrecognised value.
    Other(String),
}

impl ParameterUsage {
    /// Map an L5X `Usage` attribute.
    pub fn from_attr(v: &str) -> Self {
        match v {
            "Input" => Self::Input,
            "Output" => Self::Output,
            "InOut" => Self::InOut,
            "Public" => Self::Public,
            other => Self::Other(other.to_owned()),
        }
    }
}

/// One AOI parameter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AoiParameter {
    /// Parameter name.
    pub name: Name,
    /// Declared type.
    pub data_type: Option<Name>,
    /// Direction.
    pub usage: ParameterUsage,
    /// Must be supplied as an argument at the call site.
    pub required: bool,
    /// Shown on the instruction face.
    pub visible: bool,
    /// Array dimensions, empty for scalars.
    pub dimensions: Vec<u32>,
    /// For alias parameters: the local tag/member aliased.
    pub alias_for: Option<String>,
    /// Description text.
    pub description: Option<String>,
}

// ---------------------------------------------------------------- tags

/// Where a tag is declared. Determines name resolution (ADR-0002, DOMAIN_NOTES §4.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Scope {
    /// Controller scope: visible everywhere except inside AOIs.
    Controller,
    /// Program scope.
    Program(ProgramId),
    /// AOI local tag.
    Aoi(AoiId),
}

/// The `TagType` attribute.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TagKind {
    /// Ordinary tag with its own storage.
    Base,
    /// Another name for a tag or member; see [`Tag::alias_for`].
    Alias,
    /// Published to other controllers.
    Produced,
    /// Received from another controller.
    Consumed,
    /// Unrecognised value.
    Other(String),
}

impl TagKind {
    /// Map an L5X `TagType` attribute.
    pub fn from_attr(v: &str) -> Self {
        match v {
            "Base" => Self::Base,
            "Alias" => Self::Alias,
            "Produced" => Self::Produced,
            "Consumed" => Self::Consumed,
            other => Self::Other(other.to_owned()),
        }
    }
}

/// A tag declaration (controller, program or AOI-local scope).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tag {
    /// Tag name.
    pub name: Name,
    /// Declaration scope.
    pub scope: Scope,
    /// Base, alias, produced or consumed.
    pub kind: TagKind,
    /// Declared type (aliases usually have none; it comes from the target).
    pub data_type: Option<Name>,
    /// Array dimensions, empty for scalars.
    pub dimensions: Vec<u32>,
    /// For aliases: the target operand text, e.g. `Local:4:O.Data.3`.
    pub alias_for: Option<String>,
    /// Program parameter direction (v24+), if this tag is a parameter.
    pub usage: Option<ParameterUsage>,
    /// For consumed tags: producing controller and remote tag name.
    pub consumed_from: Option<ConsumedInfo>,
    /// `Constant="true"`: logic may not write it.
    pub constant: bool,
    /// `ExternalAccess` attribute (HMI/SCADA visibility).
    pub external_access: Option<String>,
    /// `Class`, e.g. `Safety` or `Standard`.
    pub class: Option<String>,
    /// Description text.
    pub description: Option<String>,
    /// Partial-export role.
    pub export_use: ExportUse,
}

/// Source of a consumed tag.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsumedInfo {
    /// `Producer`: name of the module (remote controller) in the I/O tree.
    pub producer: Option<String>,
    /// `RemoteTag`: tag name in the producing controller.
    pub remote_tag: Option<String>,
}

// ---------------------------------------------------------------- programs, routines

/// A program (`Program` element).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Program {
    /// Program name.
    pub name: Name,
    /// Routine executed each time the program is scanned.
    pub main_routine: Option<Name>,
    /// Routine executed on a major fault in this program.
    pub fault_routine: Option<Name>,
    /// Disabled programs are never scanned.
    pub disabled: bool,
    /// Folder-only program (no logic of its own).
    pub use_as_folder: bool,
    /// `Class`, e.g. `Safety`.
    pub class: Option<String>,
    /// Child programs (program hierarchy, v24+).
    pub children: Vec<Name>,
    /// Routines owned by this program.
    pub routines: Vec<RoutineId>,
    /// Description text.
    pub description: Option<String>,
    /// Partial-export role.
    pub export_use: ExportUse,
}

/// Who owns a routine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RoutineOwner {
    /// A routine of a program.
    Program(ProgramId),
    /// A routine of an AOI definition.
    Aoi(AoiId),
}

/// Language of a routine.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Language {
    /// Relay ladder logic.
    Ladder,
    /// Structured text.
    StructuredText,
    /// Function block diagram.
    FunctionBlock,
    /// Sequential function chart.
    Sfc,
    /// Unrecognised `Type` attribute.
    Other(String),
}

impl Language {
    /// Map an L5X routine `Type` attribute.
    pub fn from_attr(v: &str) -> Self {
        match v {
            "RLL" => Self::Ladder,
            "ST" => Self::StructuredText,
            "FBD" => Self::FunctionBlock,
            "SFC" => Self::Sfc,
            other => Self::Other(other.to_owned()),
        }
    }
}

/// A routine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Routine {
    /// Routine name.
    pub name: Name,
    /// Owning program or AOI.
    pub owner: RoutineOwner,
    /// Declared language.
    pub language: Language,
    /// Description text.
    pub description: Option<String>,
    /// Body, unparsed.
    pub body: RoutineBody,
    /// Byte offset of the `Routine` element in the source file.
    pub offset: u64,
    /// Partial-export role.
    pub export_use: ExportUse,
}

/// A routine body as read from the file. Parsed further by `s5k-logic`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RoutineBody {
    /// Ladder rungs in neutral text.
    Ladder(Vec<Rung>),
    /// Structured text lines.
    StructuredText(Vec<StLine>),
    /// Function block sheets as raw element trees.
    FunctionBlock(Vec<RawElement>),
    /// SFC content as raw element trees (steps, transitions, links ...).
    Sfc(Vec<RawElement>),
    /// Content that cannot be read.
    Opaque(OpaqueReason),
    /// The routine has no content element.
    Empty,
}

/// Why a routine body is opaque.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum OpaqueReason {
    /// Encrypted (`EncodedData`, source protection).
    SourceProtected,
    /// Language or content element not understood by this version.
    Unsupported(String),
}

/// One ladder rung.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rung {
    /// `Number` attribute.
    pub number: u32,
    /// `Type` attribute: `N` normal; other values come from pending online edits.
    pub kind: String,
    /// Neutral text, e.g. `XIC(Start)OTE(Run);`.
    pub text: String,
    /// Rung comment.
    pub comment: Option<String>,
    /// Byte offset of the `Rung` element.
    pub offset: u64,
}

/// One structured text line.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StLine {
    /// `Number` attribute (0-based).
    pub number: u32,
    /// Line text.
    pub text: String,
}

/// A generic XML subtree, used for FBD/SFC content until `s5k-logic` models it.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RawElement {
    /// Element name.
    pub name: String,
    /// Attributes in document order.
    pub attrs: Vec<(String, String)>,
    /// Concatenated text and CDATA directly inside this element.
    pub text: String,
    /// Child elements.
    pub children: Vec<RawElement>,
}

impl RawElement {
    /// Value of an attribute, if present.
    pub fn attr(&self, key: &str) -> Option<&str> {
        self.attrs
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
    }
}

// ---------------------------------------------------------------- tasks

/// How a task is triggered.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskKind {
    /// Runs continuously in the background.
    Continuous,
    /// Runs every [`Task::rate_ms`].
    Periodic,
    /// Runs on an event trigger.
    Event,
    /// Unrecognised value.
    Other(String),
}

impl TaskKind {
    /// Map an L5X `Type` attribute (case-insensitive).
    pub fn from_attr(v: &str) -> Self {
        match v.to_ascii_uppercase().as_str() {
            "CONTINUOUS" => Self::Continuous,
            "PERIODIC" => Self::Periodic,
            "EVENT" => Self::Event,
            _ => Self::Other(v.to_owned()),
        }
    }
}

/// A task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    /// Task name.
    pub name: Name,
    /// Trigger kind.
    pub kind: TaskKind,
    /// Period for periodic tasks, in milliseconds.
    pub rate_ms: Option<f64>,
    /// Priority (1 highest, 15 lowest).
    pub priority: Option<u32>,
    /// Watchdog in milliseconds.
    pub watchdog_ms: Option<f64>,
    /// Inhibited tasks never run.
    pub inhibited: bool,
    /// Event trigger (`EventTrigger`) and tag (`EventTag`), for event tasks.
    pub event_trigger: Option<String>,
    /// Tag that triggers an event task, if any.
    pub event_tag: Option<String>,
    /// Programs scheduled in this task, in execution order.
    pub scheduled_programs: Vec<Name>,
    /// Description text.
    pub description: Option<String>,
}

// ---------------------------------------------------------------- modules

/// An I/O module (or the controller itself, or a remote adapter/controller).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Module {
    /// Module name. The controller's own module is usually `Local`.
    pub name: Name,
    /// Catalog number, e.g. `1756-OB16E`.
    pub catalog_number: Option<String>,
    /// Name of the parent module in the I/O tree.
    pub parent_module: Option<Name>,
    /// Port of the parent this module is connected through.
    pub parent_port_id: Option<u32>,
    /// Inhibited modules exchange no data.
    pub inhibited: bool,
    /// Ports (backplane slots, Ethernet addresses ...).
    pub ports: Vec<ModulePort>,
    /// Connections and their I/O tags.
    pub connections: Vec<ModuleConnection>,
    /// Data type of the configuration tag (`ConfigTag`), if exported.
    pub config_type: Option<Name>,
    /// Description text.
    pub description: Option<String>,
}

/// A module port.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModulePort {
    /// Port ID.
    pub id: u32,
    /// Slot number or network address.
    pub address: Option<String>,
    /// Port type, e.g. `ICP` (backplane) or `Ethernet`.
    pub kind: Option<String>,
    /// Whether the port faces the parent.
    pub upstream: bool,
}

/// A module connection with its input/output tags.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleConnection {
    /// Connection name, e.g. `Output` or `StandardDataDriven`.
    pub name: Option<String>,
    /// Data type of the connection's input tag, when present.
    pub input_type: Option<Name>,
    /// Data type of the connection's output tag, when present.
    pub output_type: Option<Name>,
    /// Whether an `InputTag` element exists.
    pub has_input: bool,
    /// Whether an `OutputTag` element exists (the module drives physical outputs).
    pub has_output: bool,
}
