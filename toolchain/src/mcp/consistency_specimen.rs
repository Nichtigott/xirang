//! The specimen comparison: one ledger anchor's shape against its siblings' declared shapes.
//! 标本比对：一条台账 anchor 的形状与它兄弟已声明的形状。
//!
//! Child of `consistency`, so it reaches the parent's parser helpers through `super`.
//! `consistency` 的子模块，经 `super` 取得父模块的解析助手。
//!
//! online: this comparison reads each sibling's declared fields and its own text, which the published record does not carry.
//! online：这次比较读的是每个兄弟已声明的字段与它自己的文本，而发布记录不携带这两样。

use super::*;

/// Compare the shape one ledger entry certifies against the shapes of its object's siblings.
/// 把台账某一条条目所采信的**形状**与它那个对象的兄弟逐个比对。
///
/// The baseline is the ledger's own choice, not this tool's: an adoption is a lease, so the entry in
/// force is the newest line for that anchor, and a lapsed one is reported as lapsed before anything
/// is compared — the bytes a person confirmed have moved, so what is compared below is today's
/// bytes and the answer says which of the two it read.
/// 基准是台账自己选的，不是本工具选的：采信是租约，因此生效的是该 anchor 的最后一条，而失效的那条会在
/// 比对之前先被报成失效——人确认过的字节已经动过，因此下面比的是今天的字节，而答案会说清它读的是哪一个。
pub(super) fn specimen_comparison(root: &Path, anchor: &str) -> Result<String, String> {
    let entries = crate::mcp::adopted::entries(root)?;
    let history = entries
        .iter()
        .filter(|entry| entry.anchor == anchor)
        .collect::<Vec<_>>();
    let Some(effective) = history.last() else {
        return Ok(format!(
            "no ledger entry names `{anchor}` in {} — the ledger holds {} entry(ies), and a specimen \
             comparison is against the entry in force (an adoption is a lease: the newest line \
             wins). `adopted` lists the anchors.\n",
            root.display(),
            entries.len()
        ));
    };
    // Per entry: a missing covered file is this specimen's state, never a failed comparison (A01).
    // 逐条：缺失的被覆盖文件是这个标本的状态，不是一次失败的比对（A01）。
    let reading = crate::mcp::adopted::read_lease(root, effective);
    let mut remaining: Vec<String> = Vec::new();
    let mut differing = 0usize;
    let mut unread = 0usize;
    let mut lines = vec![format!(
        "specimen {anchor} — ledger revision {} is the one in force (an adoption is a lease: the \
         newest line wins)",
        history.len()
    )];
    match &reading {
        Ok(reading) => match verdict_of(effective, &reading.current) {
            AdoptionVerdict::Provisional => lines.push(format!(
                "state      provisional — certifies: {} (adopted {} by {})",
                effective.certifies, effective.at, effective.verifier
            )),
            AdoptionVerdict::Lapsed { .. } => lines.push(format!(
                "state      lapsed across covered files — the bytes a person confirmed have moved, so the shape \
                 below is today's bytes, not the certified ones, and this needs a **person** rather than \
                 an edit (adopted {} by {})",
                effective.at, effective.verifier
            )),
        },
        Err(cause) => lines.push(format!(
            "state      unreadable — {cause}: the bytes this lease rests on could not be read, so the \
             lease cannot be judged and this needs a **person** rather than an edit (adopted {} by {})",
            effective.at, effective.verifier
        )),
    }
    lines.push(format!("covers     {}", effective.files.join(", ")));
    // The specimen's shape comes from the bytes the lease rests on, through the kernel's parser.
    // 标本的形状取自租约依托的那些字节，经内核的解析器读出。
    let current: &[(String, String)] = match &reading {
        Ok(reading) => &reading.current,
        Err(_) => &[],
    };
    let mut shape = Vec::new();
    let mut unreadable = Vec::new();
    for (file, text) in current {
        match one_face(text) {
            Ok(face) => {
                for field in declared_shape(&face) {
                    let seen = shape.iter().any(|kept: &DeclaredField| {
                        kept.key == field.key && kept.value == field.value
                    });
                    if !seen {
                        shape.push(field);
                    }
                }
            }
            Err(reason) => unreadable.push(format!("{file}: {reason}")),
        }
    }
    let shape_render = if reading.is_ok() {
        shape_line(&shape)
    } else {
        "unknown (the covered bytes could not be read)".to_owned()
    };
    lines.push(format!("shape      {shape_render}"));
    if !unreadable.is_empty() {
        lines.push(format!("unreadable {}", unreadable.join("; ")));
    }
    let parent = anchor
        .trim_end_matches('/')
        .rsplit_once('/')
        .map(|(above, _)| above.to_owned());
    let Some(parent) = parent else {
        lines.push(format!(
            "no siblings: `{anchor}` is a root path, and this comparison needs a parent to gather \
             the sibling set from"
        ));
        lines.extend(specimen_bounds(root));
        return Ok(format!("{}\n", lines.join("\n")));
    };
    lines.extend(crate::mcp::adopted::family_leases(root, &entries, &parent)?);
    let members = match crate::mcp::workspace::scope(root)? {
        Scope::Package(namespace) => vec![Member::package(root, namespace)],
        Scope::Workspace(members) => members,
        Scope::Unresolvable(reason) => {
            lines.push(format!("no siblings: {reason}"));
            lines.extend(specimen_bounds(root));
            return Ok(format!("{}\n", lines.join("\n")));
        }
    };
    let sources = load_sources(root)?;
    for member in &members {
        let (evidence, faces, unparsable) = member_faces(member, RECORD_CANNOT_ANSWER)?;
        lines.push(evidence);
        // A file the derivation refused is missing from the sibling set, and an answer that counted
        // the set without saying so would read as complete.
        // 推导拒绝掉的文件不在同族集合里，而一个不说明这点就清点集合的答案会被读成完整的。
        if !unparsable.trim().is_empty() {
            lines.push(format!(
                "member {}: {} (such a file is not in the sibling set below)",
                member.name,
                unparsable.trim_end()
            ));
        }
        let siblings = siblings(&faces, &parent)
            .into_iter()
            .filter(|face| face.path.trim_end_matches('/') != anchor.trim_end_matches('/'))
            .collect::<Vec<_>>();
        if siblings.is_empty() {
            lines.push(format!(
                "family {parent} · member {} · 0 sibling(s) (no other face sits directly under it)",
                member.name
            ));
            continue;
        }
        let mut rows: Vec<String> = Vec::new();
        for face in &siblings {
            let label = face
                .path
                .rsplit('/')
                .next()
                .unwrap_or(&face.path)
                .to_owned();
            let sibling_shape = match source_text(&sources, &face.source) {
                Some(text) => match one_face(text) {
                    Ok(parsed) => declared_shape(&parsed),
                    Err(reason) => {
                        unread += 1;
                        rows.push(format!("  unreadable  {label}: {reason}"));
                        remaining
                            .push(format!("read src/{}", face.source.trim_start_matches("./")));
                        continue;
                    }
                },
                None => {
                    unread += 1;
                    rows.push(format!(
                        "  unreadable  {label}: its source `{}` is not in this tree's index",
                        face.source
                    ));
                    continue;
                }
            };
            let gaps = shape_gaps(&shape, &sibling_shape);
            if gaps.is_empty() {
                continue;
            }
            differing += 1;
            rows.push(format!("  outlier     {label}: {}", gaps.join("; ")));
            // W4-6: the row names the field; the bytes that carry it are what a reader then opens a
            // file to find. The outlier's own line comes first and the specimen's (the majority's)
            // beside it, because "lacks X" is only actionable next to the declaration that has X.
            // W4-6：这一行点名了字段，而承载它的字节正是读者随后要打开文件去找的东西。离群者自己的行在前、
            // 标本（多数派）的在旁，因为"缺 X"只有在有 X 的那条声明旁边才是可行动的。
            let fields = gap_fields(&gaps);
            let mut shown: Vec<String> = Vec::new();
            if let Some(text) = source_text(&sources, &face.source) {
                let path = format!("src/{}", face.source.trim_start_matches("./"));
                shown.extend(lines_naming(text, &path, &fields, EXCERPT_LIMIT));
                // A field that is *absent* has no line to quote, so the sibling's own declaration
                // block stands in for it: "this is what it declares, next to what it does not" is
                // the pair a reader needs, and without it the row would show only the specimen's
                // half and leave the file to be opened anyway.
                // **缺**掉的字段没有行可引，于是用这个兄弟自己的声明块代替它："这是它声明的，旁边是它没声明的"
                // 才是读者需要的那一对；没有它，这一行只会显示标本那一半，文件还是得打开。
                if shown.is_empty() {
                    shown.extend(declaration_block(
                        text,
                        &path,
                        EXCERPT_LIMIT.saturating_sub(shown.len()),
                    ));
                }
            }
            for (file, text) in current {
                shown.extend(lines_naming(
                    text,
                    file,
                    &fields,
                    EXCERPT_LIMIT.saturating_sub(shown.len()),
                ));
            }
            if let Some(command) = read_command(&shown) {
                remaining.push(command);
            }
            remaining.push(format!("--call check --face {anchor}"));
            // Audit `W5-4`: a comparison that only names the deviating sibling makes the reader do
            // the repair research. The two things a repair needs are already in this answer — what
            // the majority declares (the specimen's own shape) and which sibling deviates — so the
            // row carries the **request** that performs it, in the write path's own spelling, and
            // the reader pastes it rather than reconstructing it.
            // 审计 `W5-4`：只点名离群兄弟的比对，把修复的调研留给了读者。一次修复需要的两件事本来就在这份
            // 答案里——多数派声明了什么（标本自己的形状）与哪个兄弟偏离了——因此这一行带上**执行它的那个
            // 请求**，用写入路径自己的拼写，读者粘贴而不是重建。
            // A repair request is only handed back for a face the write path **may** rewrite: `edit`
            // refuses a hand-written file by design (only generated faces are XiRang's to rewrite),
            // so a `fix` line there would be a line that fails when pasted. The hand-written case gets
            // the same facts in the form that works — what to declare, in which file.
            // 只有写入路径**可以**重写的面才交回修复请求：`edit` 按设计拒绝手写的文件（只有生成的面才是
            // XiRang 可改的），因此那种情况下给一条 `fix` 就是给一条粘贴即失败的行。手写的情形拿到的是
            // 同样的事实、以行得通的形式：要声明什么、在哪个文件。
            let text = source_text(&sources, &face.source);
            match repair_request(&shape, face, &gaps, text) {
                Some(fix) => {
                    rows.push(format!(
                        "  fix         {}",
                        serde_json::to_string(&fix).unwrap_or_default()
                    ));
                    remaining.push(format!(
                        "--call apply --action edit --node {} --apply true",
                        face.path
                    ));
                }
                None => {
                    if let Some(manual) = manual_repair(&shape, &gaps, &face.source, &face.kind) {
                        rows.push(format!("  fix         {manual}"));
                    }
                }
            }
            rows.extend(shown);
        }
        lines.push(format!(
            "family {parent} · member {} · {} sibling(s)",
            member.name,
            siblings.len()
        ));
        // The majority is the specimen **because** the comparison calls it the baseline; saying so
        // with a count is what makes the `fix` rows below a repair rather than an opinion.
        // 多数派就是标本**因为**这次比对把它当基线；用一个计数把它说出来，才让下面的 `fix` 行成为修复
        // 而不是一种意见。
        lines.push(format!(
            "majority   {} of {} sibling(s) carry this shape; shape {}",
            siblings.len() - differing - unread,
            siblings.len(),
            shape_line(&shape)
        ));
        lines.extend(rows);
        lines.push(format!(
            "conformance: {differing} of {} sibling(s) do not carry the specimen's shape{}",
            siblings.len(),
            if unread == 0 {
                String::new()
            } else {
                format!(" ({unread} could not be read and are counted as neither)")
            }
        ));
    }
    lines.push("closure below covers shape only; lease confirmation is independent".to_owned());
    lines.extend(specimen_bounds(root));
    if let Some(census) = tree_census(root) {
        lines.insert(0, census);
    }
    lines.push(
        crate::mcp::tools::closure(differing == 0 && unread == 0, &remaining)
            .trim_end()
            .to_owned(),
    );
    Ok(format!("{}\n", lines.join("\n")))
}

/// What the specimen comparison does not read, and the next call it points at.
/// 标本比对该读不到什么，以及它指向的下一次调用。
pub(super) fn specimen_bounds(root: &Path) -> Vec<String> {
    // Audit `W2-2`: this is a **constant** block, worth reading once. From the second comparison in
    // the same session it is one line pointing at the first — the same rule the write path's
    // `consequences` block follows (`session::first_time`, keyed by root so two trees never share it).
    // 审计 `W2-2`：这是一个**常量**块，值得读一次。同一会话里的第二次比对起，它就是一行指向第一次的回指
    // ——与写入路径的 `consequences` 块同一条规则（`session::first_time`，键带根，因此两棵树不共享）。
    if !crate::mcp::session::first_time(&format!("consistency-bounds:{}", root.display())) {
        return vec![
            "not covered: unchanged from this session's earlier comparison — the columns, the              derived-tree caveat and the `api` boundary are the ones printed then"
                .to_owned(),
        ];
    }
    vec![
        "quoted lines are an excerpt, not the file: at most 10 per deviating sibling, trimmed, and cut at 200 \
         characters with `…` — they are the lines the deviation is about, so `read {path, line}` is \
         still what a reader opens when the surrounding code is the question".to_owned(),
        "not covered by this comparison: it reads the shape the ledger's files declare (parts, \
         exports, handle_traits, part_traits) through the kernel's face parser and compares each \
         sibling's own file against it, so a shape stated in a shared helper, produced by a macro \
         this tree does not spell, or carried only by runtime behaviour is invisible; `parts` is \
         compared by presence alone because it names each object's own parts type, while each label \
         the specimen states in `exports`, `handle_traits` and `part_traits` has to appear in the \
         sibling's list — a label the sibling states and the specimen does not is that sibling's own \
         business; a sibling that declares **more** fields than the specimen is not reported, \
         because the specimen is the baseline and the question is whether it was followed; and a \
         deviation is a place to look, not a defect — whether the design should still be this way is \
         the ledger's own question"
            .to_owned(),
        "next   `read {path, line}` for a deviating sibling's body, `conformance {anchor}` for the \
         ledger's own verdict on this specimen"
            .to_owned(),
    ]
}
