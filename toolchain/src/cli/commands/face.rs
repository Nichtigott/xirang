//! `xirang face`: the CLI surface of the shared authoring service.
//! `xirang face`：共享创作服务的 CLI 执行面。
//!
//! The command owns argv only. Which language is chosen, where the file goes, what the declaration
//! looks like, and whether a field is carried are all decided by
//! [`crate::authoring_service`], the same service the Studio form and the MCP bridge call. This is
//! the surface the repository did not have: the CLI could scaffold a project and add a dependency,
//! but had no way to author a registration face in any language.
//! 本命令只拥有 argv。选择哪种语言、文件落在哪里、声明长什么样、某个字段是否被携带，全部由
//! [`crate::authoring_service`] 决定——也就是 Studio 表单与 MCP 桥调用的同一个服务。这正是仓库过去
//! 没有的执行面：CLI 能搭项目、加依赖，却没有办法用任何语言创作注册面。

use std::io::Write;
use std::path::PathBuf;

use xirang_kernel::authoring::{AuthoringOperation, RegistrationUpdate};
use xirang_kernel::identity::NodeId;
use xirang_kernel::interchange::LanguageId;

use crate::authoring_service::{AuthoringRequest, AuthoringService, update_from_json};

const USAGE: &str = "usage: xirang face add <module> [--parent <node-id|root>] [--lang <language>] \
                     [--file <path>] [--fields <json>] [--project <dir>] [--write]\n       \
                     xirang face edit <node-id|path> [--lang <language>] [--fields <json>] \
                     [--project <dir>] [--write]\n       \
                     xirang face show <path> [--project <dir>]\n       \
                     xirang face languages";

/// Parse the argument list and plan (or write) one authoring request.
/// 解析参数并计划（或写入）一次创作请求。
pub(crate) fn face(
    args: &mut impl Iterator<Item = String>,
    out: &mut dyn Write,
) -> Result<(), String> {
    let action = args
        .next()
        .ok_or_else(|| format!("`xirang face` needs an action; {USAGE}"))?;
    match action.as_str() {
        "-h" | "--help" | "help" => {
            writeln!(out, "{USAGE}").map_err(|error| format!("cannot write usage: {error}"))
        }
        "languages" => languages(out),
        "add" => add(args, out),
        "edit" => edit(args, out),
        "show" => show(args, out),
        other => Err(format!("unknown `xirang face` action '{other}'; {USAGE}")),
    }
}

fn languages(out: &mut dyn Write) -> Result<(), String> {
    let service = AuthoringService::new(".", "xirang.default");
    for table in service.capabilities() {
        writeln!(
            out,
            "{}  extensions: {}  fields: {}",
            table.language.as_str(),
            table.file_extensions.join(", "),
            table
                .fields
                .iter()
                .map(|field| field.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        )
        .map_err(|error| format!("cannot write the language table: {error}"))?;
    }
    Ok(())
}

fn add(args: &mut impl Iterator<Item = String>, out: &mut dyn Write) -> Result<(), String> {
    let mut module: Option<String> = None;
    let mut parent: Option<NodeId> = None;
    let mut language: Option<LanguageId> = None;
    let mut file: Option<String> = None;
    let mut fields = String::new();
    let mut project = PathBuf::from(".");
    let mut write = false;
    while let Some(argument) = args.next() {
        match argument.as_str() {
            "-h" | "--help" => {
                writeln!(out, "{USAGE}").map_err(|error| format!("cannot write usage: {error}"))?;
                return Ok(());
            }
            "--write" => write = true,
            "--parent" => {
                let value = args.next().ok_or("--parent needs a node id or `root`")?;
                parent = Some(parse_parent(&value)?);
            }
            "--lang" => {
                let value = args.next().ok_or("--lang needs a language")?;
                language = Some(
                    LanguageId::parse_language(&value)
                        .ok_or_else(|| format!("`{value}` is not a language id"))?,
                );
            }
            "--file" => file = Some(args.next().ok_or("--file needs a path")?),
            "--fields" => fields = args.next().ok_or("--fields needs a JSON object")?,
            "--project" => {
                project = PathBuf::from(args.next().ok_or("--project needs a directory")?)
            }
            _ if argument.starts_with('-') => {
                return Err(format!("unexpected option '{argument}'; {USAGE}"));
            }
            _ if module.is_none() => module = Some(argument),
            _ => return Err(format!("unexpected argument '{argument}'; {USAGE}")),
        }
    }
    let (root, namespace) = crate::cli::resolve_package(&project.to_string_lossy())?;
    let service = AuthoringService::new(root, namespace);
    let mut request = AuthoringRequest::create();
    request.module = module;
    request.parent = parent;
    request.language = language;
    request.destination = file;
    request.registration = parse_fields(&fields)?;
    run_face(service, request, write, out)
}

fn edit(args: &mut impl Iterator<Item = String>, out: &mut dyn Write) -> Result<(), String> {
    let mut target: Option<String> = None;
    let mut language: Option<LanguageId> = None;
    let mut fields = String::new();
    let mut project = PathBuf::from(".");
    let mut write = false;
    while let Some(argument) = args.next() {
        match argument.as_str() {
            "-h" | "--help" => {
                writeln!(out, "{USAGE}").map_err(|error| format!("cannot write usage: {error}"))?;
                return Ok(());
            }
            "--write" => write = true,
            "--lang" => {
                let value = args.next().ok_or("--lang needs a language")?;
                language = Some(
                    LanguageId::parse_language(&value)
                        .ok_or_else(|| format!("`{value}` is not a language id"))?,
                );
            }
            "--fields" => fields = args.next().ok_or("--fields needs a JSON object")?,
            "--project" => {
                project = PathBuf::from(args.next().ok_or("--project needs a directory")?)
            }
            _ if argument.starts_with('-') => {
                return Err(format!("unexpected option '{argument}'; {USAGE}"));
            }
            _ if target.is_none() => target = Some(argument),
            _ => return Err(format!("unexpected argument '{argument}'; {USAGE}")),
        }
    }
    let target =
        target.ok_or_else(|| format!("`xirang face edit` needs a node id or path; {USAGE}"))?;
    let (root, namespace) = crate::cli::resolve_package(&project.to_string_lossy())?;
    let service = AuthoringService::new(root, namespace);
    let mut request = match target.parse::<NodeId>() {
        Ok(node) => AuthoringRequest::modify(node),
        Err(_) => AuthoringRequest {
            language: None,
            operation: AuthoringOperation::Modify,
            node: None,
            module: None,
            parent: None,
            destination: Some(target),
            registration: RegistrationUpdate::new(),
        },
    };
    request.language = language;
    request.registration = parse_fields(&fields)?;
    run_face(service, request, write, out)
}

fn show(args: &mut impl Iterator<Item = String>, out: &mut dyn Write) -> Result<(), String> {
    let mut file: Option<String> = None;
    let mut project = PathBuf::from(".");
    while let Some(argument) = args.next() {
        match argument.as_str() {
            "-h" | "--help" => {
                writeln!(out, "{USAGE}").map_err(|error| format!("cannot write usage: {error}"))?;
                return Ok(());
            }
            "--project" => {
                project = PathBuf::from(args.next().ok_or("--project needs a directory")?)
            }
            _ if argument.starts_with('-') => {
                return Err(format!("unexpected option '{argument}'; {USAGE}"));
            }
            _ if file.is_none() => file = Some(argument),
            _ => return Err(format!("unexpected argument '{argument}'; {USAGE}")),
        }
    }
    let file = file.ok_or_else(|| format!("`xirang face show` needs a path; {USAGE}"))?;
    let (root, namespace) = crate::cli::resolve_package(&project.to_string_lossy())?;
    let update = AuthoringService::new(root, namespace).read_registration(&file)?;
    for (field, value) in update.named() {
        writeln!(out, "{}\t{value}", field.as_str())
            .map_err(|error| format!("cannot write the field table: {error}"))?;
    }
    Ok(())
}

fn parse_parent(value: &str) -> Result<NodeId, String> {
    if value.eq_ignore_ascii_case("root") {
        return Ok(xirang_kernel::ROOT_NODE_ID);
    }
    value
        .parse::<NodeId>()
        .map_err(|_| format!("`{value}` is not a node id; pass a 32-digit id or `root`"))
}

fn parse_fields(fields: &str) -> Result<RegistrationUpdate, String> {
    if fields.trim().is_empty() {
        return Ok(RegistrationUpdate::new());
    }
    let value: serde_json::Value =
        serde_json::from_str(fields).map_err(|error| format!("--fields is not JSON: {error}"))?;
    update_from_json(&value)
}

fn run_face(
    service: AuthoringService,
    request: AuthoringRequest,
    write: bool,
    out: &mut dyn Write,
) -> Result<(), String> {
    let plan = service.plan(&request)?;
    writeln!(
        out,
        "{} plan: {} ({} file(s))",
        plan.operation.as_str(),
        plan.language.as_str(),
        plan.files.len()
    )
    .map_err(|error| format!("cannot write the plan: {error}"))?;
    for file in &plan.files {
        let action = if file.remove {
            "remove"
        } else if file.creates() {
            "create"
        } else if file.changes() {
            "edit"
        } else {
            "keep"
        };
        writeln!(out, "  {action} {}", file.path)
            .map_err(|error| format!("cannot write the plan: {error}"))?;
        for edit in &file.edits {
            writeln!(
                out,
                "    bytes {}..{} -> {} byte(s)",
                edit.start,
                edit.end,
                edit.replacement.len()
            )
            .map_err(|error| format!("cannot write the plan: {error}"))?;
        }
    }
    for diagnostic in &plan.diagnostics {
        writeln!(out, "  note: {}", diagnostic.message)
            .map_err(|error| format!("cannot write the plan: {error}"))?;
    }
    for obligation in &plan.obligations {
        writeln!(
            out,
            "  must: {} — {}",
            obligation.kind.as_str(),
            obligation.detail
        )
        .map_err(|error| format!("cannot write the plan: {error}"))?;
    }
    if !write {
        writeln!(out, "nothing written; pass --write to apply this plan")
            .map_err(|error| format!("cannot write the plan: {error}"))?;
        return Ok(());
    }
    let applied = service.apply(&plan)?;
    writeln!(out, "{}", applied.message).map_err(|error| format!("cannot write: {error}"))?;
    Ok(())
}
