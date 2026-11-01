//! Query-local parsing and evidence-ranked symptom locations shared by locate and explore.
//! locate 与 explore 共用的请求内按需解析及症状候选排序。
//!
//! online: symptom text and unbuilt edits require current bytes, not the last published tree.
//! 症状文字和未构建的修改必须读当前字节，不能用最后一次发布的树代替。

use std::collections::BTreeSet;
use std::path::Path;

use super::source_index::{SourceFile, contract_lines_from, load_sources_in, parse_functions};

/// Current bytes, with function parsing requested only for files relevant to the query.
/// 当前字节；只有与查询相关的文件才请求解析函数。
pub(super) struct QuerySources {
    pub(super) files: Vec<SourceFile>,
    parsed: BTreeSet<usize>,
}

impl QuerySources {
    pub(super) fn current(root: &Path) -> Result<Self, String> {
        Self::within(root, &[])
    }

    pub(super) fn within(root: &Path, scopes: &[String]) -> Result<Self, String> {
        Ok(Self {
            files: load_sources_in(root, scopes, |_, _| false)?,
            parsed: BTreeSet::new(),
        })
    }

    pub(super) fn parse_matching(&mut self, keep: impl Fn(&SourceFile) -> bool) {
        for (id, file) in self.files.iter_mut().enumerate() {
            if !self.parsed.contains(&id) && keep(file) {
                file.functions = parse_functions(&file.source);
                self.parsed.insert(id);
            }
        }
    }
}

/// A scored function and the evidence that contributed; scores are retrieval hints, not verdicts.
/// 计分函数及贡献该分数的证据；分数是检索提示，不是判定。
pub(super) struct Candidate {
    pub(super) file_id: usize,
    pub(super) function_id: usize,
    pub(super) score: usize,
    pub(super) reasons: Vec<String>,
    is_test: bool,
    matched_terms: usize,
}

/// Distinct query terms, splitting identifier separators and retaining non-ASCII words.
/// 去重的查询词，拆开标识符分隔符，保留非 ASCII 词。
pub(super) fn words(query: &str) -> Vec<String> {
    const STOPWORDS: &[&str] = &[
        "the", "and", "for", "that", "this", "with", "when", "from", "into", "not", "its", "was",
        "are", "has", "have", "been", "which", "what", "where", "there", "then", "than", "but",
        "you", "our", "can", "will", "should", "would", "could", "does", "did", "doing", "one",
        "two",
    ];
    let mut terms = BTreeSet::new();
    for word in query.to_lowercase().split(|c: char| !c.is_alphanumeric()) {
        if word.len() >= 3 && !STOPWORDS.contains(&word) {
            let stem = if word.len() > 4 {
                word.strip_suffix("ies")
                    .map(|base| format!("{base}y"))
                    .or_else(|| {
                        word.strip_suffix('s')
                            .filter(|_| !word.ends_with("ss"))
                            .map(str::to_owned)
                    })
            } else {
                None
            };
            terms.insert(stem.unwrap_or_else(|| word.to_owned()));
        }
    }
    terms.into_iter().collect()
}

fn contains_term(text: &str, term: &str) -> bool {
    text.contains(term)
        || term
            .strip_suffix('y')
            .is_some_and(|base| text.contains(&format!("{base}ies")))
}

pub(super) fn mentions(text: &str, terms: &[String]) -> bool {
    let lower = text.to_lowercase();
    terms.iter().any(|term| contains_term(&lower, term))
}

/// Rank each function's own docs/body, so another function's comment cannot certify a hit.
/// 按每个函数自己的文档和函数体排序，另一函数的注释不能为它的命中作证。
pub(super) fn rank(sources: &[SourceFile], query: &str, terms: &[String]) -> Vec<Candidate> {
    let phrase = query.trim().to_lowercase();
    let mut rows = Vec::new();
    for (file_id, file) in sources.iter().enumerate() {
        let lines: Vec<_> = file.source.lines().collect();
        for (function_id, function) in file.functions.iter().enumerate() {
            let name = function.name.to_lowercase();
            let doc = contract_lines_from(&lines, function.line)
                .into_iter()
                .map(|(_, text)| text)
                .collect::<Vec<_>>()
                .join(" ")
                .to_lowercase();
            let body = lines[function.line.saturating_sub(1).min(lines.len())
                ..function.end_line.min(lines.len())]
                .join("\n")
                .to_lowercase();
            let mut score = 0;
            let mut reasons = Vec::new();
            let mut matched = BTreeSet::new();
            for (text, weight, label) in [
                (&name, 5, "name matches"),
                (&doc, 3, "its doc overlaps"),
                (&body, 2, "body/message matches"),
            ] {
                let hits: Vec<_> = terms
                    .iter()
                    .filter(|term| contains_term(text, term))
                    .collect();
                if !hits.is_empty() {
                    score += hits.len() * weight;
                    matched.extend(hits.iter().copied());
                    reasons.push(format!(
                        "{label} `{}`",
                        hits.iter()
                            .map(|s| s.as_str())
                            .collect::<Vec<_>>()
                            .join("`, `")
                    ));
                }
            }
            if score == 0 {
                continue;
            }
            // Covering every distinctive word beats repeating a single common word.
            // 覆盖全部有区分度的词，胜过重复单个常见词。
            score += matched.len() * 10;
            if doc.contains(&phrase) || body.contains(&phrase) {
                score += 6;
            }
            let before = &lines[..function.line.saturating_sub(1).min(lines.len())];
            let is_test = file.relative.starts_with("tests/")
                || file.relative.contains("_tests.")
                || file.relative.contains("/tests/")
                || before
                    .iter()
                    .rev()
                    .take_while(|line| {
                        let line = line.trim();
                        line.starts_with("#[") || line.starts_with("///")
                    })
                    .any(|line| line.contains("test]"));
            if is_test {
                reasons.push("test file/function".into());
            }
            rows.push(Candidate {
                file_id,
                function_id,
                score,
                reasons,
                is_test,
                matched_terms: matched.len(),
            });
        }
    }
    rows.sort_by(|a, b| {
        b.score
            .cmp(&a.score)
            .then(a.file_id.cmp(&b.file_id))
            .then(a.function_id.cmp(&b.function_id))
    });
    // Reserve a production lead when all the top hits are test assertions. Tests remain evidence.
    // 当首批命中全是测试断言时，为生产实现留一个首位；测试仍保留为证据。
    if let Some(index) = rows
        .iter()
        .position(|row| !row.is_test && row.matched_terms >= rows[0].matched_terms)
    {
        let row = rows.remove(index);
        rows.insert(0, row);
    }
    rows
}
