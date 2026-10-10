//! Structural parse tests against the synthetic fixtures (milestone M1).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use s5k_l5x::{NoProgress, Parsed, Progress, codes, read_project};
use s5k_model::*;

const MULTI: &str = include_str!("../../../tests/fixtures/multi_program_output.L5X");
const MINIMAL: &str = include_str!("../../../tests/fixtures/minimal_controller.L5X");
const PARTIAL: &str = include_str!("../../../tests/fixtures/partial_routine_export.L5X");

fn parse(xml: &str) -> Parsed {
    read_project(xml.as_bytes(), &NoProgress).expect("fixture is L5X")
}

fn has_code(p: &Parsed, code: &str) -> bool {
    p.diagnostics.iter().any(|d| d.code == code)
}

fn tag<'a>(p: &'a Project, scope: Scope, name: &str) -> &'a Tag {
    p.tags_in(scope)
        .map(|(_, t)| t)
        .find(|t| t.name == Name::new(name))
        .unwrap_or_else(|| panic!("tag {name} not found in {scope:?}"))
}

#[test]
fn fr_par_001_full_project_counts() {
    let p = parse(MULTI);
    assert!(!p.truncated);
    let pr = &p.project;
    assert_eq!(pr.data_types.len(), 1);
    assert_eq!(pr.aois.len(), 2);
    assert_eq!(pr.modules.len(), 4);
    assert_eq!(pr.programs.len(), 3);
    assert_eq!(pr.tasks.len(), 3);
    // 2 AOI routines + 1 + 4 + 3 program routines.
    assert_eq!(pr.routines.len(), 10);
    assert_eq!(pr.tags_in(Scope::Controller).count(), 7);
}

#[test]
fn fr_par_002_controller_header() {
    let p = parse(MULTI).project;
    let c = p.controller.unwrap();
    assert_eq!(c.name, Name::new("Conveyor_Line"));
    assert_eq!(c.processor_type.as_deref(), Some("1756-L83E"));
    assert_eq!((c.major_rev, c.minor_rev), (Some(32), Some(11)));
    assert_eq!(
        p.header.unwrap().software_revision.as_deref(),
        Some("32.02")
    );
}

#[test]
fn fr_par_003_udt_members_and_packed_bools() {
    let p = parse(MULTI).project;
    let udt = &p.data_types[0];
    assert_eq!(udt.name, Name::new("motor_udt"));
    assert_eq!(udt.members.len(), 5);
    let host = &udt.members[0];
    assert!(host.hidden);
    let cmd = &udt.members[1];
    assert_eq!(cmd.data_type, Name::new("BIT"));
    let bit = cmd.bit_of.as_ref().unwrap();
    assert_eq!((bit.host.as_str(), bit.bit), ("ZZZZZZZZZZMotor_UDT0", 0));
    assert_eq!(cmd.description.as_deref(), Some("Run command"));
    assert_eq!(udt.members[4].dimension, 4);
}

#[test]
fn fr_par_004_aoi_parameters_locals_routines() {
    let p = parse(MULTI).project;
    let (id, aoi) = p
        .aois
        .iter()
        .enumerate()
        .find(|(_, a)| a.name == Name::new("MotorCtl"))
        .unwrap();
    assert!(!aoi.source_protected);
    assert_eq!(aoi.revision.as_deref(), Some("2.1"));
    assert_eq!(aoi.parameters.len(), 4);
    let motor = &aoi.parameters[3];
    assert_eq!(motor.usage, ParameterUsage::InOut);
    assert!(motor.required);
    let local = tag(&p, Scope::Aoi(AoiId(id as u32)), "Delay");
    assert_eq!(local.data_type, Some(Name::new("TIMER")));
    let names: Vec<_> = aoi
        .routines
        .iter()
        .map(|r| p.routines[r.index()].name.as_str())
        .collect();
    assert_eq!(names, ["Logic", "EnableInFalse"]);
    assert_eq!(
        p.routines[aoi.routines[0].index()].owner,
        RoutineOwner::Aoi(AoiId(id as u32))
    );
}

#[test]
fn fr_par_005_tag_kinds_aliases_consumed_dimensions() {
    let p = parse(MULTI).project;
    let out = tag(&p, Scope::Controller, "Conveyor_Run_Out");
    assert_eq!(out.kind, TagKind::Alias);
    assert_eq!(out.alias_for.as_deref(), Some("Local:4:O.Data.3"));
    assert_eq!(out.description.as_deref(), Some("Conveyor motor contactor"));

    let up = tag(&p, Scope::Controller, "Upstream_Ready");
    assert_eq!(up.kind, TagKind::Consumed);
    let from = up.consumed_from.as_ref().unwrap();
    assert_eq!(from.producer.as_deref(), Some("Upstream_PLC"));
    assert_eq!(from.remote_tag.as_deref(), Some("Ready_To_Send"));

    assert!(tag(&p, Scope::Controller, "MaxSpeed").constant);
    assert_eq!(tag(&p, Scope::Controller, "Faults").dimensions, [10, 2]);
}

#[test]
fn fr_par_006_programs_parameters_and_connections() {
    let p = parse(MULTI).project;
    let (seq_id, seq) = p.program_by_name("sequencer").unwrap();
    assert_eq!(seq.main_routine, Some(Name::new("Main")));
    assert_eq!(seq.fault_routine, Some(Name::new("Fault")));
    let run_cmd = tag(&p, Scope::Program(seq_id), "RunCmd");
    assert_eq!(run_cmd.usage, Some(ParameterUsage::Output));

    let (_, safety) = p.program_by_name("Safety_Perms").unwrap();
    assert_eq!(safety.description.as_deref(), Some("Line permissives"));

    assert_eq!(p.parameter_connections.len(), 1);
    let c = &p.parameter_connections[0];
    assert_eq!(c.end_point1, "\\Sequencer.RunCmd");
    assert_eq!(c.end_point2, "\\Outputs.RunReq");
}

#[test]
fn fr_par_007_partial_routine_export() {
    let p = parse(PARTIAL);
    assert!(p.diagnostics.is_empty(), "{:?}", p.diagnostics);
    let pr = &p.project;
    let h = pr.header.as_ref().unwrap();
    assert_eq!(h.target_type, Some(ExportTarget::Routine));
    assert!(h.contains_context);
    assert_eq!(
        pr.controller.as_ref().unwrap().export_use,
        ExportUse::Context
    );
    assert_eq!(pr.programs[0].export_use, ExportUse::Context);
    assert_eq!(pr.routines.len(), 1);
    assert_eq!(pr.routines[0].export_use, ExportUse::Target);
    assert_eq!(pr.tags[0].export_use, ExportUse::Context);
}

#[test]
fn fr_par_008_tasks_schedule_programs_in_order() {
    let p = parse(MULTI).project;
    let main = &p.tasks[0];
    assert_eq!(main.kind, TaskKind::Continuous);
    let order: Vec<_> = main.scheduled_programs.iter().map(Name::as_str).collect();
    assert_eq!(order, ["Safety_Perms", "Sequencer", "Outputs"]);
    let fast = &p.tasks[1];
    assert_eq!(
        (fast.kind.clone(), fast.rate_ms, fast.priority),
        (TaskKind::Periodic, Some(10.0), Some(5))
    );
    let ev = &p.tasks[2];
    assert_eq!(ev.kind, TaskKind::Event);
    assert!(ev.inhibited);
    assert_eq!(ev.event_trigger.as_deref(), Some("EVENT Instruction Only"));
}

#[test]
fn fr_par_009_routine_bodies_for_all_languages() {
    let p = parse(MULTI).project;
    let (out_id, _) = p.program_by_name("Outputs").unwrap();
    let (_, main) = p
        .routine_by_name(RoutineOwner::Program(out_id), "Main")
        .unwrap();
    let RoutineBody::Ladder(rungs) = &main.body else {
        panic!("expected ladder");
    };
    assert_eq!(rungs.len(), 3);
    assert_eq!(
        rungs[1].text,
        "XIC(Conv_Motor.Cmd)[XIC(Line_Permissive),XIC(Faults[0,1].5)]OTE(Conveyor_Run_Out);"
    );
    assert_eq!(rungs[1].comment.as_deref(), Some("Energise the contactor."));

    let (seq_id, _) = p.program_by_name("Sequencer").unwrap();
    let (_, st) = p
        .routine_by_name(RoutineOwner::Program(seq_id), "Seq_ST")
        .unwrap();
    assert_eq!(st.language, Language::StructuredText);
    let RoutineBody::StructuredText(lines) = &st.body else {
        panic!("expected ST");
    };
    assert_eq!(lines.len(), 6);
    assert_eq!(
        lines[1].text, "    0:  RunCmd := 0;",
        "indentation is preserved"
    );

    let (_, sfc) = p
        .routine_by_name(RoutineOwner::Program(seq_id), "Seq_SFC")
        .unwrap();
    let RoutineBody::Sfc(elems) = &sfc.body else {
        panic!("expected SFC");
    };
    assert_eq!(elems.len(), 5);
    assert_eq!(elems[0].attr("InitialStep"), Some("true"));
    let condition = &elems[1].children[0].children[0].children[0];
    assert_eq!(condition.text, "StartPB");

    let (_, fbd) = p
        .routine_by_name(RoutineOwner::Program(out_id), "Speed")
        .unwrap();
    let RoutineBody::FunctionBlock(sheets) = &fbd.body else {
        panic!("expected FBD");
    };
    assert_eq!(sheets[0].children.len(), 3);
    assert_eq!(sheets[0].children[2].attr("FromID"), Some("0"));
}

#[test]
fn fr_par_010_source_protected_content_is_opaque() {
    let p = parse(MULTI);
    let pr = &p.project;
    let pid = pr
        .aois
        .iter()
        .find(|a| a.name == Name::new("VendorPID"))
        .unwrap();
    assert!(pid.source_protected);
    assert_eq!(
        pid.parameters.len(),
        2,
        "parameters of protected AOIs stay visible"
    );

    let (out_id, _) = pr.program_by_name("Outputs").unwrap();
    let (_, r) = pr
        .routine_by_name(RoutineOwner::Program(out_id), "Vendor_Tuning")
        .unwrap();
    assert!(matches!(
        r.body,
        RoutineBody::Opaque(OpaqueReason::SourceProtected)
    ));
    assert!(has_code(&p, codes::SOURCE_PROTECTED));
}

#[test]
fn fr_par_011_modules_ports_and_io_connections() {
    let p = parse(MULTI).project;
    let dout = p
        .modules
        .iter()
        .find(|m| m.name == Name::new("DO_Slot4"))
        .unwrap();
    assert_eq!(dout.catalog_number.as_deref(), Some("1756-OB16E"));
    assert_eq!(dout.parent_module, Some(Name::new("Local")));
    assert_eq!(dout.parent_port_id, Some(1));
    assert_eq!(dout.ports[0].address.as_deref(), Some("4"));
    assert_eq!(dout.config_type, Some(Name::new("AB:1756_DO_Fused:C:0")));
    let conn = &dout.connections[0];
    assert!(conn.has_output && conn.has_input);
    assert_eq!(conn.output_type, Some(Name::new("AB:1756_DO:O:0")));

    let remote = p
        .modules
        .iter()
        .find(|m| m.name == Name::new("Upstream_PLC"))
        .unwrap();
    assert_eq!(remote.ports[1].address.as_deref(), Some("192.168.1.20"));
}

#[test]
fn fr_par_014_unknown_elements_warn_and_parsing_continues() {
    let p = parse(MULTI);
    let unknown: Vec<_> = p
        .diagnostics
        .iter()
        .filter(|d| d.code == codes::UNKNOWN_ELEMENT)
        .collect();
    assert_eq!(unknown.len(), 1, "{unknown:?}");
    assert!(unknown[0].message.contains("FutureFeature"));
    assert_eq!(unknown[0].severity, Severity::Warning);
    // Elements after the unknown one are still read.
    assert!(!p.truncated);
    assert!(has_code(&p, codes::PENDING_EDIT));
    assert_eq!(p.diagnostics.max_severity(), Some(Severity::Warning));
}

#[test]
fn fr_par_015_progress_is_reported_and_monotonic() {
    use std::cell::RefCell;
    struct Rec(RefCell<Vec<u64>>);
    impl Progress for Rec {
        fn advance(&self, bytes: u64) {
            self.0.borrow_mut().push(bytes);
        }
    }
    let rec = Rec(RefCell::new(Vec::new()));
    read_project(MULTI.as_bytes(), &rec).unwrap();
    let seen = rec.0.into_inner();
    assert!(seen.len() > 10);
    assert!(seen.windows(2).all(|w| w[0] <= w[1]));
    assert_eq!(*seen.last().unwrap(), MULTI.len() as u64);
}

#[test]
fn minimal_fixture_parses_cleanly() {
    let p = parse(MINIMAL);
    assert!(p.diagnostics.is_empty(), "{:?}", p.diagnostics);
    assert_eq!(p.project.routines.len(), 3);
}

/// NFR-REL-001: no panics on any prefix of a valid file, and every prefix
/// that contains the root start tag yields a (partial) project.
#[test]
fn nfr_rel_001_truncated_input_never_panics() {
    let bytes = MULTI.as_bytes();
    let root_end = MULTI.find("<Controller").unwrap();
    let complete_at = MULTI.rfind("</RSLogix5000Content>").unwrap() + "</RSLogix5000Content>".len();
    let mut step = 1;
    let mut cut = 0;
    while cut < bytes.len() {
        let result = read_project(&bytes[..cut], &NoProgress);
        if cut > root_end && cut < complete_at {
            let parsed = result.expect("root was read, so a partial project is returned");
            assert!(
                parsed.truncated,
                "cut at {cut} should be reported as truncated"
            );
            assert!(has_code(&parsed, codes::MALFORMED_XML));
        }
        cut += step;
        step = if cut < 2000 { 1 } else { 7 };
    }
    assert!(!parse(MULTI).truncated);
}

#[test]
fn malformed_xml_keeps_completed_elements() {
    let cut = MULTI.find("<Programs>").unwrap();
    let broken = format!("{}<Programs><Program Name=\"X\"></Wrong>", &MULTI[..cut]);
    let p = read_project(broken.as_bytes(), &NoProgress).unwrap();
    assert!(p.truncated);
    assert_eq!(p.project.data_types.len(), 1);
    assert_eq!(p.project.modules.len(), 4);
    assert_eq!(p.project.tags_in(Scope::Controller).count(), 7);
    assert_eq!(p.diagnostics.max_severity(), Some(Severity::Error));
}

/// NFR-REL-001: random byte corruption never panics (a cheap stand-in for
/// fuzzing until a cargo-fuzz target runs in CI).
#[test]
fn nfr_rel_001_random_corruption_never_panics() {
    const MUTANTS: &[u8] = b"<>/=\"'&;[]! \n\0\xff\xc3";
    let mut seed: u64 = 0x5eed_1234_abcd_0001;
    let mut rand = move || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed
    };
    for _ in 0..3000 {
        let mut bytes = MULTI.as_bytes().to_vec();
        for _ in 0..(1 + rand() % 4) {
            let at = (rand() as usize) % bytes.len();
            bytes[at] = MUTANTS[(rand() as usize) % MUTANTS.len()];
        }
        // Any outcome is fine except a panic.
        let _ = read_project(&bytes[..], &NoProgress);
    }
}
