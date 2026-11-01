//! `xirang worker`: run one job document read from stdin and print the result. This is the worker
//! side of the remote protocol; a transport (ssh, a VM guest agent) only has to move the text.
//! `xirang worker`：从 stdin 读一份任务文档、运行、打印结果。这是远程协议的 worker 一侧；
//! 传输层（ssh、虚拟机内的 agent）只负责搬这段文本。

use std::io::{Read, Write};

/// `xirang worker`.
/// `xirang worker`。
pub(crate) fn worker(out: &mut dyn Write) -> Result<(), String> {
    let mut job = String::new();
    std::io::stdin()
        .read_to_string(&mut job)
        .map_err(|error| format!("cannot read the job from stdin: {error}"))?;
    let result = crate::multilang::worker_job(&job)?;
    writeln!(out, "{result}").map_err(|error| format!("cannot write: {error}"))
}
