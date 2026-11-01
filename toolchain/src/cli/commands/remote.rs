//! `xirang remote --worker <command> [--test] [--locked] [path]`: hand one build or test job to a
//! worker command and print its summary. The command is any program that reads the job on stdin and
//! writes one result document on stdout.
//! `xirang remote --worker <command>`：把一次 build 或 test 任务交给 worker 命令并打印摘要。
//! 该命令可以是任何“从 stdin 读任务、往 stdout 写一份结果”的程序。

use std::io::Write;
use std::path::Path;

use crate::multilang::Transfer;

/// The transfer root the command uses when `--root` names none.
/// 命令在 `--root` 没有点名时使用的回传根。
const DEFAULT_TRANSFER: &str = "build";

fn one_action() -> String {
    "give at most one of `--test`, `--evidence`, `--fetch`, `--list` and `--fetch-all`".to_owned()
}

/// `xirang remote --worker <command> [--test] [--locked] [path]`.
/// `xirang remote --worker <command> [--test] [--locked] [path]`。
pub(crate) fn remote(
    arguments: &mut dyn Iterator<Item = String>,
    out: &mut dyn Write,
) -> Result<(), String> {
    let mut command = None;
    let mut chosen: Option<&str> = None;
    let mut locked = false;
    let mut paths = Vec::new();
    let mut jobs = 1usize;
    let mut ssh = None;
    let mut known_hosts = None;
    let mut timeout = None;
    let mut retries = 0usize;
    let mut fetch = None;
    let mut into = None;
    let mut list = None;
    let mut fetch_all = None;
    let mut transfer = None;
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--worker" => command = arguments.next(),
            "--test" => {
                if chosen.replace("test").is_some() {
                    return Err(one_action());
                }
            }
            "--evidence" => {
                if chosen.replace("evidence").is_some() {
                    return Err(one_action());
                }
            }
            "--fetch" => {
                if chosen.replace("fetch").is_some() {
                    return Err(one_action());
                }
                fetch = arguments.next();
            }
            "--list" => {
                if chosen.replace("list").is_some() {
                    return Err(one_action());
                }
                list = arguments.next();
            }
            "--fetch-all" => {
                if chosen.replace("fetch-all").is_some() {
                    return Err(one_action());
                }
                fetch_all = arguments.next();
            }
            "--root" => {
                let value = arguments.next().ok_or_else(|| {
                    "`--root` needs a directory name under `target/xirang`".to_owned()
                })?;
                transfer = Some(value);
            }
            "--into" => into = arguments.next().map(std::path::PathBuf::from),
            "--locked" => locked = true,
            "--ssh" => ssh = arguments.next(),
            "--known-hosts" => known_hosts = arguments.next().map(std::path::PathBuf::from),
            "--timeout" => {
                let value = arguments
                    .next()
                    .ok_or_else(|| "`--timeout` needs a number of seconds".to_owned())?;
                let seconds = value
                    .parse::<u64>()
                    .map_err(|_| format!("`--timeout {value}` is not a number of seconds"))?;
                if seconds == 0 || seconds > 3600 {
                    return Err("`--timeout` takes 1 to 3600 seconds".to_owned());
                }
                timeout = Some(seconds);
            }
            "--retries" => {
                let value = arguments
                    .next()
                    .ok_or_else(|| "`--retries` needs a number".to_owned())?;
                retries = value
                    .parse::<usize>()
                    .map_err(|_| format!("`--retries {value}` is not a number"))?;
                if retries > 5 {
                    return Err("`--retries` takes at most 5".to_owned());
                }
            }
            "--jobs" => {
                let value = arguments
                    .next()
                    .ok_or_else(|| "`--jobs` needs a number".to_owned())?;
                jobs = value
                    .parse::<usize>()
                    .map_err(|_| format!("`--jobs {value}` is not a number of workers"))?;
            }
            other if other.starts_with('-') => {
                return Err(format!(
                    "unknown argument `{other}`; `xirang remote` takes `--worker <command>`, \
                     `--test`, `--evidence`, `--fetch <file>`, `--fetch-all <dir>`, \
                     `--list <dir>`, `--into <dir>`, `--locked`, `--jobs <n>`, \
                     `--timeout <s>`, `--retries <n>`, `--ssh <host>`, `--known-hosts <file>`, \
                     `--root <dir>` and one or more directories"
                ));
            }
            other => paths.push(std::path::PathBuf::from(other)),
        }
    }
    if ssh.is_some() && command.is_some() {
        return Err(
            "give either `--worker <command>` or `--ssh <host>`, not both; `--ssh` is a preset \
             over `--worker`"
                .to_owned(),
        );
    }
    if known_hosts.is_some() && ssh.is_none() {
        return Err(
            "`--known-hosts` pins host keys for `--ssh`; it means nothing with `--worker`"
                .to_owned(),
        );
    }
    if let Some(host) = ssh {
        if host.is_empty()
            || !host.chars().all(|character| {
                character.is_ascii_alphanumeric()
                    || matches!(character, '_' | '-' | '.' | ':' | '@')
            })
        {
            return Err(format!(
                "`--ssh` takes a host name; `{host}` has characters this preset does not pass                  to ssh"
            ));
        }
        let mut preset = String::from("ssh -o BatchMode=yes -o StrictHostKeyChecking=yes");
        if let Some(file) = known_hosts {
            let text = file.display().to_string();
            if text.chars().any(char::is_whitespace) || text.contains('\'') {
                return Err(format!(
                    "`--known-hosts {text}` has spaces or quotes; the preset builds a shell command, so name a path that needs no quoting"
                ));
            }
            if !file.is_file() {
                return Err(format!("`--known-hosts {text}` is not a file"));
            }
            preset.push_str(&format!(
                " -o UserKnownHostsFile={text} -o GlobalKnownHostsFile=/dev/null"
            ));
        }
        preset.push_str(&format!(" {host} xirang worker"));
        command = Some(preset);
    }
    let command = command
        .ok_or_else(|| "`xirang remote` needs `--worker <command>` or `--ssh <host>`".to_owned())?;
    let action = chosen.unwrap_or("build");
    if action == "fetch" && fetch.is_none() {
        return Err(format!(
            "`--fetch` names the file to fetch, relative to `<project>/target/xirang/{}`",
            transfer.as_deref().unwrap_or(DEFAULT_TRANSFER)
        ));
    }
    if action == "fetch" && into.is_none() {
        return Err("`--fetch` needs `--into <directory>` to write the file to".to_owned());
    }
    if action == "fetch-all" && fetch_all.is_none() {
        return Err(format!(
            "`--fetch-all` names the directory to fetch, relative to `<project>/target/xirang/{}`",
            transfer.as_deref().unwrap_or(DEFAULT_TRANSFER)
        ));
    }
    if action == "fetch-all" && into.is_none() {
        return Err("`--fetch-all` needs `--into <directory>` to write the files to".to_owned());
    }
    if action == "list" && list.is_none() {
        return Err(format!(
            "`--list` names the directory to list, relative to `<project>/target/xirang/{}`",
            transfer.as_deref().unwrap_or(DEFAULT_TRANSFER)
        ));
    }
    if !matches!(action, "fetch" | "fetch-all" | "list") && transfer.is_some() {
        return Err(
            "`--root` only means something with `--fetch`, `--fetch-all` or `--list`".to_owned(),
        );
    }
    if !matches!(action, "fetch" | "fetch-all") && into.is_some() {
        return Err("`--into` only means something with `--fetch` or `--fetch-all`".to_owned());
    }
    if matches!(action, "fetch" | "fetch-all" | "list") && paths.len() > 1 {
        return Err("this action reads one project; give one directory".to_owned());
    }
    let control = crate::multilang::Control {
        timeout: timeout.map(std::time::Duration::from_secs),
        retries,
    };
    if paths.len() > 1 {
        writeln!(
            out,
            "{}",
            crate::multilang::remote_many(&paths, &command, action, locked, jobs, control)?
        )
        .map_err(|error| format!("cannot write: {error}"))?;
        return Ok(());
    }
    let root = Path::new(
        paths
            .first()
            .map_or(".", |path| path.to_str().unwrap_or(".")),
    );
    if action == "list" {
        writeln!(
            out,
            "{}",
            crate::multilang::remote_listing(
                root,
                &command,
                list.as_deref().unwrap_or_default(),
                transfer.as_deref().unwrap_or(DEFAULT_TRANSFER),
                control,
            )?
        )
        .map_err(|error| format!("cannot write: {error}"))?;
        return Ok(());
    }
    if action == "fetch-all" {
        let into = into
            .as_deref()
            .ok_or_else(|| "`--fetch-all` needs `--into <directory>`".to_owned())?;
        writeln!(
            out,
            "{}",
            crate::multilang::remote_fetch_all(
                root,
                &command,
                fetch_all.as_deref().unwrap_or_default(),
                into,
                transfer.as_deref().unwrap_or(DEFAULT_TRANSFER),
                control,
            )?
        )
        .map_err(|error| format!("cannot write: {error}"))?;
        return Ok(());
    }
    writeln!(
        out,
        "{}",
        crate::multilang::remote_worker(
            root,
            &command,
            action,
            locked,
            control,
            Transfer {
                path: fetch.as_deref(),
                into: into.as_deref(),
                root: transfer.as_deref().unwrap_or(DEFAULT_TRANSFER),
            },
        )?
    )
    .map_err(|error| format!("cannot write: {error}"))
}
