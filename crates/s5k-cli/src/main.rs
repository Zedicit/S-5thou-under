//! `s5k`: understand Studio 5000 L5X projects from the command line.

#![forbid(unsafe_code)]

use std::fs::File;
use std::io::{self, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Instant;

use anyhow::Context;
use clap::{Parser, Subcommand};
use indicatif::{ProgressBar, ProgressStyle};
use s5k_l5x::{Parsed, Progress};
use s5k_model::{Language, Module, Name, Project, RoutineBody, Scope, Severity, TaskKind};

#[derive(Parser)]
#[command(name = "s5k", version, about = "Understand Studio 5000 L5X projects")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Summarise an L5X file: contents, execution schedule, I/O modules, diagnostics.
    Inspect {
        /// Path to the .L5X file.
        file: PathBuf,
        /// Read only the export header (instant, even for huge files).
        #[arg(long)]
        header_only: bool,
        /// Print the whole parsed model as JSON instead of a summary.
        #[arg(long, conflicts_with = "header_only")]
        json: bool,
        /// List every diagnostic.
        #[arg(short, long)]
        verbose: bool,
        /// No progress bar.
        #[arg(short, long)]
        quiet: bool,
    },
}

/// Exit codes (FR-CLI-003).
const EXIT_DIAG_ERROR: u8 = 1;
const EXIT_FAILURE: u8 = 3;

fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(cli) {
        Ok(code) => code,
        Err(err) => {
            #[allow(clippy::print_stderr)]
            {
                eprintln!("error: {err:#}");
            }
            ExitCode::from(EXIT_FAILURE)
        }
    }
}

fn run(cli: Cli) -> anyhow::Result<ExitCode> {
    match cli.command {
        Command::Inspect {
            file,
            header_only,
            json,
            verbose,
            quiet,
        } => {
            let mut out = BufWriter::new(io::stdout().lock());
            if header_only {
                let header = s5k_l5x::read_header(open(&file)?)
                    .with_context(|| format!("reading {}", file.display()))?;
                writeln!(out, "{}", serde_json::to_string_pretty(&header)?)?;
                return Ok(ExitCode::SUCCESS);
            }
            let started = Instant::now();
            let parsed = parse_with_progress(&file, quiet)?;
            let elapsed = started.elapsed();
            if json {
                let doc = serde_json::json!({
                    "truncated": parsed.truncated,
                    "diagnostics": parsed.diagnostics,
                    "project": parsed.project,
                });
                serde_json::to_writer_pretty(&mut out, &doc)?;
                writeln!(out)?;
            } else {
                summary(&mut out, &parsed, verbose)?;
                writeln!(out, "\nParsed in {:.2} s.", elapsed.as_secs_f64())?;
            }
            out.flush()?;
            let failed = parsed.diagnostics.max_severity() == Some(Severity::Error);
            Ok(if failed {
                ExitCode::from(EXIT_DIAG_ERROR)
            } else {
                ExitCode::SUCCESS
            })
        }
    }
}

fn open(file: &Path) -> anyhow::Result<BufReader<File>> {
    let f = File::open(file).with_context(|| format!("opening {}", file.display()))?;
    Ok(BufReader::with_capacity(1 << 20, f))
}

struct Bar(ProgressBar);

impl Progress for Bar {
    fn advance(&self, bytes: u64) {
        self.0.set_position(bytes);
    }
}

fn parse_with_progress(file: &Path, quiet: bool) -> anyhow::Result<Parsed> {
    let len = std::fs::metadata(file).map(|m| m.len()).unwrap_or(0);
    // indicatif hides itself when stderr is not a terminal.
    let bar = if quiet {
        ProgressBar::hidden()
    } else {
        ProgressBar::new(len)
    };
    bar.set_style(
        ProgressStyle::with_template(
            "{spinner} reading [{bar:40}] {bytes}/{total_bytes} ({bytes_per_sec}, eta {eta})",
        )?
        .progress_chars("=> "),
    );
    let parsed = s5k_l5x::read_project(open(file)?, &Bar(bar.clone()))
        .with_context(|| format!("reading {}", file.display()))?;
    bar.finish_and_clear();
    Ok(parsed)
}

fn summary(out: &mut impl Write, parsed: &Parsed, verbose: bool) -> io::Result<()> {
    let p = &parsed.project;
    let header = p.header.as_ref();

    // Identity line.
    let name = p
        .controller
        .as_ref()
        .map(|c| c.name.to_string())
        .or_else(|| header.and_then(|h| h.target_name.clone()))
        .unwrap_or_else(|| "-".into());
    let processor = p
        .controller
        .as_ref()
        .and_then(|c| c.processor_type.clone())
        .unwrap_or_else(|| "unknown processor".into());
    let firmware = p
        .controller
        .as_ref()
        .and_then(|c| {
            c.major_rev
                .map(|maj| format!("{maj}.{}", c.minor_rev.unwrap_or(0)))
        })
        .unwrap_or_else(|| "?".into());
    writeln!(out, "{name}  ({processor}, firmware {firmware})")?;
    if let Some(h) = header {
        writeln!(
            out,
            "  export: {:?}{} from Studio 5000 v{}",
            h.target_type
                .clone()
                .unwrap_or(s5k_model::ExportTarget::Other("?".into())),
            if h.contains_context {
                " with context"
            } else {
                ""
            },
            h.software_revision.as_deref().unwrap_or("?"),
        )?;
    }
    if parsed.truncated {
        writeln!(
            out,
            "  WARNING: the file could not be read to the end; results are partial."
        )?;
    }

    // Counts.
    let protected_aois = p.aois.iter().filter(|a| a.source_protected).count();
    let controller_tags = p.tags_in(Scope::Controller).count();
    writeln!(out, "\nContents")?;
    writeln!(out, "  data types   {}", p.data_types.len())?;
    writeln!(
        out,
        "  AOIs         {} ({protected_aois} source-protected)",
        p.aois.len()
    )?;
    writeln!(
        out,
        "  tags         {} ({controller_tags} controller scope)",
        p.tags.len()
    )?;
    writeln!(out, "  programs     {}", p.programs.len())?;
    writeln!(out, "  routines     {}", p.routines.len())?;
    writeln!(out, "               {}", language_breakdown(p))?;
    writeln!(out, "  tasks        {}", p.tasks.len())?;
    writeln!(out, "  modules      {}", p.modules.len())?;
    writeln!(out, "  param links  {}", p.parameter_connections.len())?;

    schedule(out, p)?;
    modules(out, p)?;

    // Diagnostics.
    let d = &parsed.diagnostics;
    writeln!(
        out,
        "\nDiagnostics: {} error(s), {} warning(s), {} info{}",
        d.count(Severity::Error),
        d.count(Severity::Warning),
        d.count(Severity::Info),
        if verbose || d.is_empty() {
            ""
        } else {
            "  (use -v to list)"
        },
    )?;
    if verbose {
        for diag in d.iter() {
            let at = diag
                .offset
                .map(|o| format!(" @ byte {o}"))
                .unwrap_or_default();
            writeln!(
                out,
                "  {:?} {}{at}: {}",
                diag.severity, diag.code, diag.message
            )?;
        }
    }
    Ok(())
}

fn language_breakdown(p: &Project) -> String {
    let (mut rll, mut st, mut fbd, mut sfc, mut opaque, mut other) = (0, 0, 0, 0, 0, 0);
    for r in &p.routines {
        if matches!(r.body, RoutineBody::Opaque(_)) {
            opaque += 1;
            continue;
        }
        match r.language {
            Language::Ladder => rll += 1,
            Language::StructuredText => st += 1,
            Language::FunctionBlock => fbd += 1,
            Language::Sfc => sfc += 1,
            Language::Other(_) => other += 1,
        }
    }
    let mut s = format!("ladder {rll}, ST {st}, FBD {fbd}, SFC {sfc}, protected {opaque}");
    if other > 0 {
        s.push_str(&format!(", other {other}"));
    }
    s
}

/// Task → program → main routine: the entry points of everything that runs.
fn schedule(out: &mut impl Write, p: &Project) -> io::Result<()> {
    writeln!(out, "\nExecution schedule (task -> programs in scan order)")?;
    let mut scheduled = vec![false; p.programs.len()];
    for t in &p.tasks {
        let kind = match &t.kind {
            TaskKind::Continuous => "continuous".to_owned(),
            TaskKind::Periodic => format!("periodic {} ms", t.rate_ms.unwrap_or(0.0)),
            TaskKind::Event => format!(
                "event ({})",
                t.event_trigger.as_deref().unwrap_or("unknown trigger")
            ),
            TaskKind::Other(k) => k.clone(),
        };
        let prio = t
            .priority
            .map(|v| format!(", priority {v}"))
            .unwrap_or_default();
        let inhibit = if t.inhibited { "  INHIBITED" } else { "" };
        writeln!(out, "  {}  [{kind}{prio}]{inhibit}", t.name)?;
        if t.scheduled_programs.is_empty() {
            writeln!(out, "      (no programs)")?;
        }
        for (n, prog_name) in t.scheduled_programs.iter().enumerate() {
            match p.program_by_name(prog_name.as_str()) {
                Some((id, prog)) => {
                    scheduled[id.index()] = true;
                    let main = prog.main_routine.as_ref().map(Name::as_str).unwrap_or("-");
                    let fault = prog
                        .fault_routine
                        .as_ref()
                        .map(|f| format!(", fault: {f}"))
                        .unwrap_or_default();
                    let disabled = if prog.disabled { "  DISABLED" } else { "" };
                    let routines = prog.routines.len();
                    writeln!(
                        out,
                        "    {}. {}  (main: {main}{fault}; routines: {routines}){disabled}",
                        n + 1,
                        prog.name
                    )?;
                }
                None => writeln!(out, "    {}. {prog_name}  (not in this file)", n + 1)?,
            }
        }
    }
    let unscheduled: Vec<_> = p
        .programs
        .iter()
        .zip(&scheduled)
        .filter(|(prog, s)| !**s && !prog.use_as_folder)
        .map(|(prog, _)| prog.name.as_str())
        .collect();
    if !unscheduled.is_empty() {
        writeln!(
            out,
            "  Not scheduled by any task: {}",
            unscheduled.join(", ")
        )?;
    }
    Ok(())
}

fn module_location(m: &Module) -> String {
    let parent = m.parent_module.as_ref().map(Name::as_str).unwrap_or("?");
    match m.ports.iter().find(|port| port.upstream) {
        Some(port) => {
            let addr = port.address.as_deref().unwrap_or("?");
            match port.kind.as_deref() {
                Some("ICP") => format!("{parent} slot {addr}"),
                Some(kind) => format!("{parent} {} {addr}", kind.to_ascii_lowercase()),
                None => format!("{parent} {addr}"),
            }
        }
        None => "-".into(),
    }
}

fn modules(out: &mut impl Write, p: &Project) -> io::Result<()> {
    if p.modules.is_empty() {
        return Ok(());
    }
    writeln!(out, "\nI/O modules")?;
    let width = p
        .modules
        .iter()
        .map(|m| m.name.as_str().len())
        .max()
        .unwrap_or(0);
    for m in &p.modules {
        let has_in = m.connections.iter().any(|c| c.has_input);
        let has_out = m.connections.iter().any(|c| c.has_output);
        let io = match (has_in, has_out) {
            (true, true) => "inputs + outputs",
            (true, false) => "inputs",
            (false, true) => "outputs",
            (false, false) => "",
        };
        let inhibit = if m.inhibited { "  INHIBITED" } else { "" };
        let line = format!(
            "  {:width$}  {:12}  {:18}  {io}{inhibit}",
            m.name.as_str(),
            m.catalog_number.as_deref().unwrap_or("?"),
            module_location(m),
        );
        writeln!(out, "{}", line.trim_end())?;
    }
    Ok(())
}
