//! Recursive-descent mapping from L5X elements to `s5k_model::Project`.
//!
//! One method per element kind; each consumes its element completely.
//! Unknown elements are skipped with a warning (FR-PAR-014); elements we know
//! but deliberately don't model are skipped silently. Attributes we don't use
//! (`Radix`, layout coordinates ...) are ignored without a warning, since
//! warning on them would bury useful diagnostics.

use std::io::BufRead;

use s5k_model::{
    AoiDefinition, AoiId, AoiParameter, BitTarget, ConsumedInfo, ControllerInfo, DataType,
    Diagnostic, Diagnostics, ExportUse, Language, Member, Module, ModuleConnection, ModulePort,
    Name, OpaqueReason, ParameterConnection, ParameterUsage, Program, ProgramId, Project,
    RawElement, Routine, RoutineBody, RoutineId, RoutineOwner, Rung, Scope, Severity, StLine, Tag,
    TagKind, Task, TaskKind,
};

use crate::xml::{El, Node, Xml};
use crate::{Error, Progress, codes};

/// Elements we understand and intentionally do not model (yet).
const IGNORED: &[&str] = &[
    // Controller level
    "RedundancyInfo",
    "Security",
    "SafetyInfo",
    "CST",
    "WallClockTime",
    "Trends",
    "DataLogs",
    "TimeSynchronize",
    "EthernetPorts",
    "QuickWatchLists",
    "CommPorts",
    "AlarmDefinitions",
    "ControllerDeviceInfo",
    // Common children
    "Comments",
    "Data",
    "DefaultData",
    "ForceData",
    "ProduceInfo",
    "RevisionNote",
    "AdditionalHelpText",
    "ExtendedProperties",
    "EKey",
    "CustomProperties",
    "AlarmConditions",
    "Signature",
    "SignatureHistory",
];

pub(crate) struct Parser<'p, R: BufRead> {
    pub xml: Xml<R>,
    pub project: Project,
    pub diags: Diagnostics,
    progress: &'p dyn Progress,
}

fn flag(el: &El, key: &str) -> bool {
    el.get(key).is_some_and(|s| s.eq_ignore_ascii_case("true"))
}

fn export_use(el: &El) -> ExportUse {
    match el.get("Use") {
        Some("Target") => ExportUse::Target,
        Some("Context") => ExportUse::Context,
        _ => ExportUse::Unspecified,
    }
}

fn opt_name(el: &El, key: &str) -> Option<Name> {
    el.get(key).filter(|s| !s.is_empty()).map(Name::new)
}

fn opt_string(el: &El, key: &str) -> Option<String> {
    el.get(key).filter(|s| !s.is_empty()).map(str::to_owned)
}

fn next_id(len: usize) -> u32 {
    // Arenas beyond u32::MAX items are not realistic for L5X files (that would
    // need a file far larger than any controller can hold).
    u32::try_from(len).unwrap_or(u32::MAX)
}

impl<'p, R: BufRead> Parser<'p, R> {
    pub fn new(xml: Xml<R>, progress: &'p dyn Progress) -> Self {
        Self {
            xml,
            project: Project::default(),
            diags: Diagnostics::default(),
            progress,
        }
    }

    // ------------------------------------------------------------ helpers

    fn diag(&mut self, severity: Severity, code: &str, message: String, offset: u64) {
        self.diags
            .push(Diagnostic::new(severity, code, message, Some(offset)));
    }

    /// Skip a child element: silently if known-ignored, else with a warning.
    fn other(&mut self, el: &El, parent: &str) -> Result<(), Error> {
        if !IGNORED.contains(&el.name.as_str()) {
            self.diag(
                Severity::Warning,
                codes::UNKNOWN_ELEMENT,
                format!("unknown element <{}> inside <{parent}> skipped", el.name),
                el.offset,
            );
        }
        self.xml.skip(el)
    }

    /// The `Name` attribute, or a warning and `None` (caller skips the element).
    fn required_name(&mut self, el: &El) -> Option<Name> {
        let name = opt_name(el, "Name");
        if name.is_none() {
            self.diag(
                Severity::Warning,
                codes::MISSING_NAME,
                format!("<{}> without a Name attribute skipped", el.name),
                el.offset,
            );
        }
        name
    }

    fn number<T: std::str::FromStr>(&mut self, el: &El, key: &str) -> Option<T> {
        let raw = el.get(key)?.trim();
        if raw.is_empty() {
            return None;
        }
        match raw.parse() {
            Ok(v) => Some(v),
            Err(_) => {
                let msg = format!(
                    "<{}> attribute {key}={raw:?} is not a valid number",
                    el.name
                );
                self.diag(Severity::Warning, codes::BAD_ATTRIBUTE, msg, el.offset);
                None
            }
        }
    }

    /// `Dimensions="10 5 2"` / `Dimension="10"`; zero entries mean scalar.
    fn dimensions(&mut self, el: &El, key: &str) -> Vec<u32> {
        let Some(raw) = el.get(key) else {
            return Vec::new();
        };
        let mut dims = Vec::new();
        for part in raw.split([' ', ',']).filter(|s| !s.is_empty()) {
            match part.parse::<u32>() {
                Ok(0) => {}
                Ok(d) => dims.push(d),
                Err(_) => {
                    let msg = format!(
                        "<{}> attribute {key}={raw:?} is not a dimension list",
                        el.name
                    );
                    self.diag(Severity::Warning, codes::BAD_ATTRIBUTE, msg, el.offset);
                    return Vec::new();
                }
            }
        }
        dims
    }

    fn tick(&self) {
        self.progress.advance(self.xml.position());
    }

    // ------------------------------------------------------------ document

    /// Parse the children of the `RSLogix5000Content` root.
    pub fn root(&mut self, root: &El) -> Result<(), Error> {
        while let Some(el) = self.xml.next_elem(root)? {
            match el.name.as_str() {
                "Controller" => self.controller(&el)?,
                _ => self.other(&el, &root.name)?,
            }
        }
        Ok(())
    }

    /// Read to end of file after the root; anything there is reported.
    pub fn trailing(&mut self) -> Result<(), Error> {
        while let Some(el) = self.xml.next_top_level()? {
            self.diag(
                Severity::Warning,
                codes::UNKNOWN_ELEMENT,
                format!("element <{}> after the root element ignored", el.name),
                el.offset,
            );
            self.xml.skip(&el)?;
        }
        Ok(())
    }

    fn controller(&mut self, el: &El) -> Result<(), Error> {
        let mut info = ControllerInfo {
            name: opt_name(el, "Name").unwrap_or_default(),
            processor_type: opt_string(el, "ProcessorType"),
            major_rev: self.number(el, "MajorRev"),
            minor_rev: self.number(el, "MinorRev"),
            description: None,
            export_use: export_use(el),
        };
        while let Some(child) = self.xml.next_elem(el)? {
            self.tick();
            match child.name.as_str() {
                "Description" => info.description = self.xml.localized_text(&child)?,
                "DataTypes" => self.collection(&child, "DataType", Self::data_type)?,
                "Modules" => self.collection(&child, "Module", Self::module)?,
                "AddOnInstructionDefinitions" => {
                    while let Some(item) = self.xml.next_elem(&child)? {
                        self.tick();
                        match item.name.as_str() {
                            "AddOnInstructionDefinition" => self.aoi(&item, false)?,
                            "EncodedData" => self.aoi(&item, true)?,
                            _ => self.other(&item, &child.name)?,
                        }
                    }
                }
                "Tags" => self.tags(&child, Scope::Controller)?,
                "Programs" => self.collection(&child, "Program", Self::program)?,
                "Tasks" => self.collection(&child, "Task", Self::task)?,
                "ParameterConnections" => {
                    self.collection(&child, "ParameterConnection", Self::parameter_connection)?
                }
                _ => self.other(&child, &el.name)?,
            }
        }
        self.project.controller = Some(info);
        Ok(())
    }

    /// Children named `item` go to `f`; anything else is reported and skipped.
    fn collection(
        &mut self,
        parent: &El,
        item: &str,
        f: fn(&mut Self, &El) -> Result<(), Error>,
    ) -> Result<(), Error> {
        while let Some(el) = self.xml.next_elem(parent)? {
            self.tick();
            if el.name == item {
                f(self, &el)?;
            } else {
                self.other(&el, &parent.name)?;
            }
        }
        Ok(())
    }

    // ------------------------------------------------------------ data types

    fn data_type(&mut self, el: &El) -> Result<(), Error> {
        let Some(name) = self.required_name(el) else {
            return self.xml.skip(el);
        };
        let mut dt = DataType {
            name,
            family: opt_string(el, "Family"),
            class: opt_string(el, "Class"),
            description: None,
            members: Vec::new(),
            export_use: export_use(el),
        };
        while let Some(child) = self.xml.next_elem(el)? {
            match child.name.as_str() {
                "Description" => dt.description = self.xml.localized_text(&child)?,
                "Members" => {
                    while let Some(m) = self.xml.next_elem(&child)? {
                        if m.name == "Member" {
                            if let Some(member) = self.member(&m)? {
                                dt.members.push(member);
                            }
                        } else {
                            self.other(&m, &child.name)?;
                        }
                    }
                }
                "Dependencies" => self.xml.skip(&child)?,
                _ => self.other(&child, &el.name)?,
            }
        }
        self.project.data_types.push(dt);
        Ok(())
    }

    fn member(&mut self, el: &El) -> Result<Option<Member>, Error> {
        let Some(name) = self.required_name(el) else {
            self.xml.skip(el)?;
            return Ok(None);
        };
        let bit_of = match (opt_name(el, "Target"), self.number::<u8>(el, "BitNumber")) {
            (Some(host), Some(bit)) => Some(BitTarget { host, bit }),
            _ => None,
        };
        let mut member = Member {
            name,
            data_type: opt_name(el, "DataType").unwrap_or_default(),
            dimension: self.number(el, "Dimension").unwrap_or(0),
            hidden: flag(el, "Hidden"),
            bit_of,
            external_access: opt_string(el, "ExternalAccess"),
            description: None,
        };
        while let Some(child) = self.xml.next_elem(el)? {
            match child.name.as_str() {
                "Description" => member.description = self.xml.localized_text(&child)?,
                _ => self.other(&child, &el.name)?,
            }
        }
        Ok(Some(member))
    }

    // ------------------------------------------------------------ modules

    fn module(&mut self, el: &El) -> Result<(), Error> {
        let Some(name) = self.required_name(el) else {
            return self.xml.skip(el);
        };
        let mut module = Module {
            name,
            catalog_number: opt_string(el, "CatalogNumber"),
            parent_module: opt_name(el, "ParentModule"),
            parent_port_id: self.number(el, "ParentModPortId"),
            inhibited: flag(el, "Inhibited"),
            ports: Vec::new(),
            connections: Vec::new(),
            config_type: None,
            description: None,
        };
        while let Some(child) = self.xml.next_elem(el)? {
            match child.name.as_str() {
                "Description" => module.description = self.xml.localized_text(&child)?,
                "Ports" => {
                    while let Some(port) = self.xml.next_elem(&child)? {
                        if port.name == "Port" {
                            module.ports.push(ModulePort {
                                id: self.number(&port, "Id").unwrap_or(0),
                                address: opt_string(&port, "Address"),
                                kind: opt_string(&port, "Type"),
                                upstream: flag(&port, "Upstream"),
                            });
                            self.xml.skip(&port)?;
                        } else {
                            self.other(&port, &child.name)?;
                        }
                    }
                }
                "Communications" => self.communications(&child, &mut module)?,
                _ => self.other(&child, &el.name)?,
            }
        }
        self.project.modules.push(module);
        Ok(())
    }

    fn communications(&mut self, el: &El, module: &mut Module) -> Result<(), Error> {
        while let Some(child) = self.xml.next_elem(el)? {
            match child.name.as_str() {
                "ConfigTag" => {
                    module.config_type = self
                        .xml
                        .find_attr(&child, "Structure", "DataType")?
                        .map(Name::new);
                }
                "Connections" => {
                    while let Some(conn) = self.xml.next_elem(&child)? {
                        if conn.name == "Connection" {
                            let c = self.connection(&conn)?;
                            module.connections.push(c);
                        } else {
                            self.other(&conn, &child.name)?;
                        }
                    }
                }
                _ => self.other(&child, &el.name)?,
            }
        }
        Ok(())
    }

    fn connection(&mut self, el: &El) -> Result<ModuleConnection, Error> {
        let mut c = ModuleConnection {
            name: opt_string(el, "Name"),
            input_type: None,
            output_type: None,
            has_input: false,
            has_output: false,
        };
        while let Some(child) = self.xml.next_elem(el)? {
            match child.name.as_str() {
                "InputTag" => {
                    c.has_input = true;
                    c.input_type = self
                        .xml
                        .find_attr(&child, "Structure", "DataType")?
                        .map(Name::new);
                }
                "OutputTag" => {
                    c.has_output = true;
                    c.output_type = self
                        .xml
                        .find_attr(&child, "Structure", "DataType")?
                        .map(Name::new);
                }
                _ => self.other(&child, &el.name)?,
            }
        }
        Ok(c)
    }

    // ------------------------------------------------------------ AOIs

    fn aoi(&mut self, el: &El, encoded: bool) -> Result<(), Error> {
        let Some(name) = self.required_name(el) else {
            return self.xml.skip(el);
        };
        let id = AoiId(next_id(self.project.aois.len()));
        let mut aoi = AoiDefinition {
            name,
            revision: opt_string(el, "Revision"),
            description: None,
            parameters: Vec::new(),
            routines: Vec::new(),
            dependencies: Vec::new(),
            source_protected: encoded,
            export_use: export_use(el),
        };
        if encoded {
            self.diag(
                Severity::Info,
                codes::SOURCE_PROTECTED,
                format!("AOI {} is source-protected; its logic is opaque", aoi.name),
                el.offset,
            );
        }
        while let Some(child) = self.xml.next_elem(el)? {
            match child.name.as_str() {
                "Description" => aoi.description = self.xml.localized_text(&child)?,
                "Parameters" => {
                    while let Some(p) = self.xml.next_elem(&child)? {
                        if p.name == "Parameter" {
                            if let Some(param) = self.aoi_parameter(&p)? {
                                aoi.parameters.push(param);
                            }
                        } else {
                            self.other(&p, &child.name)?;
                        }
                    }
                }
                "LocalTags" => {
                    while let Some(t) = self.xml.next_elem(&child)? {
                        if t.name == "LocalTag" {
                            self.tag(&t, Scope::Aoi(id))?;
                        } else {
                            self.other(&t, &child.name)?;
                        }
                    }
                }
                "Routines" => {
                    aoi.routines = self.routines(&child, RoutineOwner::Aoi(id))?;
                }
                "Dependencies" => {
                    while let Some(d) = self.xml.next_elem(&child)? {
                        if let Some(n) = opt_name(&d, "Name") {
                            aoi.dependencies.push(n);
                        }
                        self.xml.skip(&d)?;
                    }
                }
                // An encoded AOI carries its encrypted payload as text and
                // possibly further unknown elements; nothing to report there.
                _ if encoded => self.xml.skip(&child)?,
                _ => self.other(&child, &el.name)?,
            }
        }
        self.project.aois.push(aoi);
        Ok(())
    }

    fn aoi_parameter(&mut self, el: &El) -> Result<Option<AoiParameter>, Error> {
        let Some(name) = self.required_name(el) else {
            self.xml.skip(el)?;
            return Ok(None);
        };
        let mut p = AoiParameter {
            name,
            data_type: opt_name(el, "DataType"),
            usage: ParameterUsage::from_attr(el.get("Usage").unwrap_or("Input")),
            required: flag(el, "Required"),
            visible: flag(el, "Visible"),
            dimensions: self.dimensions(el, "Dimensions"),
            alias_for: opt_string(el, "AliasFor"),
            description: None,
        };
        while let Some(child) = self.xml.next_elem(el)? {
            match child.name.as_str() {
                "Description" => p.description = self.xml.localized_text(&child)?,
                _ => self.other(&child, &el.name)?,
            }
        }
        Ok(Some(p))
    }

    // ------------------------------------------------------------ tags

    fn tags(&mut self, el: &El, scope: Scope) -> Result<(), Error> {
        while let Some(t) = self.xml.next_elem(el)? {
            self.tick();
            if t.name == "Tag" {
                self.tag(&t, scope)?;
            } else {
                self.other(&t, &el.name)?;
            }
        }
        Ok(())
    }

    fn tag(&mut self, el: &El, scope: Scope) -> Result<(), Error> {
        let Some(name) = self.required_name(el) else {
            return self.xml.skip(el);
        };
        let mut tag = Tag {
            name,
            scope,
            kind: TagKind::from_attr(el.get("TagType").unwrap_or("Base")),
            data_type: opt_name(el, "DataType"),
            dimensions: self.dimensions(el, "Dimensions"),
            alias_for: opt_string(el, "AliasFor"),
            usage: el.get("Usage").map(ParameterUsage::from_attr),
            consumed_from: None,
            constant: flag(el, "Constant"),
            external_access: opt_string(el, "ExternalAccess"),
            class: opt_string(el, "Class"),
            description: None,
            export_use: export_use(el),
        };
        while let Some(child) = self.xml.next_elem(el)? {
            match child.name.as_str() {
                "Description" => tag.description = self.xml.localized_text(&child)?,
                "ConsumeInfo" => {
                    tag.consumed_from = Some(ConsumedInfo {
                        producer: opt_string(&child, "Producer"),
                        remote_tag: opt_string(&child, "RemoteTag"),
                    });
                    self.xml.skip(&child)?;
                }
                _ => self.other(&child, &el.name)?,
            }
        }
        self.project.tags.push(tag);
        Ok(())
    }

    fn parameter_connection(&mut self, el: &El) -> Result<(), Error> {
        match (opt_string(el, "EndPoint1"), opt_string(el, "EndPoint2")) {
            (Some(end_point1), Some(end_point2)) => {
                self.project
                    .parameter_connections
                    .push(ParameterConnection {
                        end_point1,
                        end_point2,
                    });
            }
            _ => self.diag(
                Severity::Warning,
                codes::BAD_ATTRIBUTE,
                "<ParameterConnection> without both EndPoint1 and EndPoint2 skipped".into(),
                el.offset,
            ),
        }
        self.xml.skip(el)
    }

    // ------------------------------------------------------------ programs

    fn program(&mut self, el: &El) -> Result<(), Error> {
        let Some(name) = self.required_name(el) else {
            return self.xml.skip(el);
        };
        let id = ProgramId(next_id(self.project.programs.len()));
        let mut program = Program {
            name,
            main_routine: opt_name(el, "MainRoutineName"),
            fault_routine: opt_name(el, "FaultRoutineName"),
            disabled: flag(el, "Disabled"),
            use_as_folder: flag(el, "UseAsFolder"),
            class: opt_string(el, "Class"),
            children: Vec::new(),
            routines: Vec::new(),
            description: None,
            export_use: export_use(el),
        };
        while let Some(child) = self.xml.next_elem(el)? {
            self.tick();
            match child.name.as_str() {
                "Description" => program.description = self.xml.localized_text(&child)?,
                "Tags" => self.tags(&child, Scope::Program(id))?,
                "Routines" => {
                    program.routines = self.routines(&child, RoutineOwner::Program(id))?
                }
                "ChildPrograms" => {
                    while let Some(c) = self.xml.next_elem(&child)? {
                        if let Some(n) = opt_name(&c, "Name") {
                            program.children.push(n);
                        }
                        self.xml.skip(&c)?;
                    }
                }
                _ => self.other(&child, &el.name)?,
            }
        }
        self.project.programs.push(program);
        Ok(())
    }

    // ------------------------------------------------------------ routines

    fn routines(&mut self, el: &El, owner: RoutineOwner) -> Result<Vec<RoutineId>, Error> {
        let mut ids = Vec::new();
        while let Some(r) = self.xml.next_elem(el)? {
            self.tick();
            let routine = match r.name.as_str() {
                "Routine" => self.routine(&r, owner)?,
                "EncodedData" => self.encoded_routine(&r, owner)?,
                _ => {
                    self.other(&r, &el.name)?;
                    None
                }
            };
            if let Some(routine) = routine {
                ids.push(RoutineId(next_id(self.project.routines.len())));
                self.project.routines.push(routine);
            }
        }
        Ok(ids)
    }

    fn encoded_routine(&mut self, el: &El, owner: RoutineOwner) -> Result<Option<Routine>, Error> {
        let Some(name) = self.required_name(el) else {
            self.xml.skip(el)?;
            return Ok(None);
        };
        self.diag(
            Severity::Info,
            codes::SOURCE_PROTECTED,
            format!("routine {name} is source-protected; its logic is opaque"),
            el.offset,
        );
        self.xml.skip(el)?;
        Ok(Some(Routine {
            name,
            owner,
            language: Language::from_attr(el.get("Type").unwrap_or("")),
            description: None,
            body: RoutineBody::Opaque(OpaqueReason::SourceProtected),
            offset: el.offset,
            export_use: export_use(el),
        }))
    }

    fn routine(&mut self, el: &El, owner: RoutineOwner) -> Result<Option<Routine>, Error> {
        let Some(name) = self.required_name(el) else {
            self.xml.skip(el)?;
            return Ok(None);
        };
        let mut routine = Routine {
            name,
            owner,
            language: Language::from_attr(el.get("Type").unwrap_or("")),
            description: None,
            body: RoutineBody::Empty,
            offset: el.offset,
            export_use: export_use(el),
        };
        while let Some(child) = self.xml.next_elem(el)? {
            match child.name.as_str() {
                "Description" => routine.description = self.xml.localized_text(&child)?,
                "RLLContent" => routine.body = RoutineBody::Ladder(self.rungs(&child)?),
                "STContent" => routine.body = RoutineBody::StructuredText(self.st_lines(&child)?),
                "FBDContent" => {
                    routine.body = RoutineBody::FunctionBlock(self.raw_children(&child)?)
                }
                "SFCContent" => routine.body = RoutineBody::Sfc(self.raw_children(&child)?),
                "EncodedData" => {
                    self.diag(
                        Severity::Info,
                        codes::SOURCE_PROTECTED,
                        format!(
                            "routine {} is source-protected; its logic is opaque",
                            routine.name
                        ),
                        child.offset,
                    );
                    routine.body = RoutineBody::Opaque(OpaqueReason::SourceProtected);
                    self.xml.skip(&child)?;
                }
                _ => self.other(&child, &el.name)?,
            }
        }
        Ok(Some(routine))
    }

    fn rungs(&mut self, el: &El) -> Result<Vec<Rung>, Error> {
        let mut rungs = Vec::new();
        while let Some(r) = self.xml.next_elem(el)? {
            if r.name != "Rung" {
                self.other(&r, &el.name)?;
                continue;
            }
            let mut rung = Rung {
                number: self.number(&r, "Number").unwrap_or(next_id(rungs.len())),
                kind: r.get("Type").unwrap_or("N").to_owned(),
                text: String::new(),
                comment: None,
                offset: r.offset,
            };
            if rung.kind != "N" {
                self.diag(
                    Severity::Info,
                    codes::PENDING_EDIT,
                    format!(
                        "rung {} has edit type {:?} (pending online edit)",
                        rung.number, rung.kind
                    ),
                    r.offset,
                );
            }
            while let Some(child) = self.xml.next_elem(&r)? {
                match child.name.as_str() {
                    "Text" => rung.text = self.xml.text(&child)?.trim().to_owned(),
                    "Comment" => rung.comment = self.xml.localized_text(&child)?,
                    _ => self.other(&child, &r.name)?,
                }
            }
            rungs.push(rung);
        }
        Ok(rungs)
    }

    fn st_lines(&mut self, el: &El) -> Result<Vec<StLine>, Error> {
        let mut lines = Vec::new();
        while let Some(l) = self.xml.next_elem(el)? {
            if l.name != "Line" {
                self.other(&l, &el.name)?;
                continue;
            }
            let number = self.number(&l, "Number").unwrap_or(next_id(lines.len()));
            // Not trimmed: indentation is meaningful to readers of ST.
            let text = self.xml.text(&l)?;
            lines.push(StLine { number, text });
        }
        Ok(lines)
    }

    fn raw_children(&mut self, el: &El) -> Result<Vec<RawElement>, Error> {
        let mut out = Vec::new();
        while let Some(child) = self.xml.next_elem(el)? {
            out.push(self.raw(child)?);
        }
        Ok(out)
    }

    fn raw(&mut self, el: El) -> Result<RawElement, Error> {
        let mut raw = RawElement {
            text: String::new(),
            children: Vec::new(),
            name: String::new(),
            attrs: Vec::new(),
        };
        while let Some(node) = self.xml.next_child(&el)? {
            match node {
                Node::Text(t) => raw.text.push_str(&t),
                Node::Elem(child) => raw.children.push(self.raw(child)?),
            }
        }
        raw.name = el.name;
        raw.attrs = el.attrs;
        Ok(raw)
    }

    // ------------------------------------------------------------ tasks

    fn task(&mut self, el: &El) -> Result<(), Error> {
        let Some(name) = self.required_name(el) else {
            return self.xml.skip(el);
        };
        let mut task = Task {
            name,
            kind: TaskKind::from_attr(el.get("Type").unwrap_or("")),
            rate_ms: self.number(el, "Rate"),
            priority: self.number(el, "Priority"),
            watchdog_ms: self.number(el, "Watchdog"),
            inhibited: flag(el, "InhibitTask"),
            event_trigger: None,
            event_tag: None,
            scheduled_programs: Vec::new(),
            description: None,
        };
        while let Some(child) = self.xml.next_elem(el)? {
            match child.name.as_str() {
                "Description" => task.description = self.xml.localized_text(&child)?,
                "ScheduledPrograms" => {
                    while let Some(p) = self.xml.next_elem(&child)? {
                        if let Some(n) = opt_name(&p, "Name") {
                            task.scheduled_programs.push(n);
                        }
                        self.xml.skip(&p)?;
                    }
                }
                "EventInfo" => {
                    task.event_trigger = opt_string(&child, "EventTrigger");
                    task.event_tag = opt_string(&child, "EventTag");
                    self.xml.skip(&child)?;
                }
                _ => self.other(&child, &el.name)?,
            }
        }
        self.project.tasks.push(task);
        Ok(())
    }
}
