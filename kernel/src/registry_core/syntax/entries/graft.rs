//! The `graft_plan!` / `static_graft_plan!` entry grammar.
//! `graft_plan!` / `static_graft_plan!` 入口语法。
//!
//! A graft cut names both sides the same way — either two Rust expressions or
//! two strings — so the generated table never mixes a resolved identity with
//! an unresolved name.
//! 一条 graft 切口的两侧命名方式必须一致——要么两个 Rust 表达式，要么两个字符串——
//! 生成表因此不会混合已解析身份与未解析名称。

use proc_macro2::{Delimiter, TokenStream, TokenTree};
use syn::spanned::Spanned;
use syn::visit::Visit;

use super::super::face::{compact, split_typed_range};
use super::super::{FaceSyntaxError, SyntaxLocation, compact_tokens, location, syntax_error};

/// The source span of a span-carrying token.
/// 一个携带 span 的 token 的源码跨度。
fn endpoint_span(span: proc_macro2::Span) -> GraftEndpointSpan {
    let end = span.end();
    GraftEndpointSpan {
        start: location(span),
        end: SyntaxLocation {
            line: end.line,
            column: end.column + 1,
        },
    }
}

/// A host-declared external graft cut discovered before code generation.
/// 在代码生成前从宿主入口发现的一条外部 graft 切口。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GraftSyntax {
    /// The cut target: a logical path, or the Rust expression text of a typed
    /// cut. For a range this is the *start* endpoint, never `"start to end"`.
    /// 切口目标：逻辑路径，或类型化切口的 Rust 表达式原文。区间切口这里是**起点**，
    /// 绝不是 `"start to end"`。
    pub cut: String,
    /// The far endpoint of a sibling range, as data.
    /// 兄弟区间的远端端点，以数据形式携带。
    ///
    /// Why the joined-string encoding was wrong: a logical path is free-form
    /// text, so once `"a to b"` is stored in `cut` nothing downstream can tell a
    /// range from a single path that literally contains `" to "` — the four
    /// consumers that re-split `cut` cut such a path in half. The boundary is
    /// this field: only the parser may decide that a `to` token was a range
    /// separator, and every consumer reads the endpoints from `cut`/`cut_end`.
    /// Pinned by `a_path_containing_the_range_word_is_a_single_cut` (this file)
    /// and `a_string_range_keeps_both_endpoints` (`build_method`).
    /// 拼接字符串的编码错在哪：逻辑路径是自由文本，一旦把 `"a to b"` 存进 `cut`，
    /// 下游就再也分不清区间与一条字面含有 `" to "` 的路径——四处重新拆分 `cut` 的
    /// 消费方会把这种路径拦腰截断。边界就是本字段：只有解析器能判定某个 `to` token
    /// 是区间分隔符，所有消费方都从 `cut`/`cut_end` 读取端点。
    /// 由本文件的 `a_path_containing_the_range_word_is_a_single_cut` 与
    /// `build_method` 的 `a_string_range_keeps_both_endpoints` 钉住。
    pub cut_end: Option<String>,
    /// The replacement side, written the same way as `cut`: a logical selector, a
    /// Rust expression, or an identity.
    /// 替换侧，与 `cut` 使用同一种写法：逻辑选择器、Rust 表达式或身份。
    pub graft: String,
    /// Whether the replacement covers the whole subtree at the cut target rather
    /// than only its node.
    /// 替换是否覆盖切口目标的整棵子树，而不只是该节点自身。
    pub full: bool,
    /// Where the declaration was written, for diagnostics that point at it.
    /// 声明写在哪，供指向它的诊断使用。
    pub location: SyntaxLocation,
    /// The `cfg` gate the declaration carries, if any, exactly as written — with the
    /// gates of the modules around it combined in as `all(…)`, because a declaration
    /// inside a gated module inherits that gate and the build step, not this parser,
    /// is the one that evaluates it (audit `LGC-LG-29`).
    /// 声明携带的 `cfg` 门控（若有），按原文保留——并把它周围各模块的门控以 `all(…)` 并入：
    /// 被门控模块里的声明继承那道门控，而求值的是构建步骤而不是本解析器（审计 `LGC-LG-29`）。
    pub cfg: Option<String>,
    /// Present when `cut(...)` / `graft(...)` supplied Rust expressions. The
    /// renderer emits those expressions verbatim, so the compiler — and any
    /// editor that resolves Rust paths — sees the real target instead of a
    /// string the tooling would have to interpret.
    /// 当 `cut(...)` / `graft(...)` 给出 Rust 表达式时存在。渲染器会原样发射这些
    /// 表达式，因此编译器和任何能解析 Rust 路径的编辑器看到的都是真实目标，
    /// 而不是需要工具自己解释的字符串。
    pub expressions: Option<GraftExpressions>,
    /// The source span of each endpoint, when the parser could record one. A move rewrites the
    /// endpoint the author actually wrote — which form and which of several same-line references —
    /// so the span is a fact about the source, not a re-derivation from the normalized path.
    /// 每个端点在源码里的跨度（解析器能记录时）。一次搬动要改的是作者**实际写下的**那个端点——
    /// 哪种形式、同一行里的哪一处——因此跨度是关于源码的事实，不能从归一化后的路径重新推导。
    pub spans: Option<GraftSpans>,
}

/// One endpoint's source span.
/// 一个端点的源码跨度。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GraftEndpointSpan {
    /// One-based start (line, column).
    /// 1 起始的起点（行、列）。
    pub start: SyntaxLocation,
    /// One-based exclusive end (line, column).
    /// 1 起始的排他终点（行、列）。
    pub end: SyntaxLocation,
}

/// Every endpoint a typed graft declaration wrote, with its source span.
/// 一条类型化 graft 声明写下的每个端点，及其源码跨度。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GraftSpans {
    /// The cut target.
    /// 切口目标。
    pub cut: GraftEndpointSpan,
    /// The far endpoint of a range, when one was written.
    /// 区间远端端点（若写了区间）。
    pub cut_end: Option<GraftEndpointSpan>,
    /// The replacement expression.
    /// 替换表达式。
    pub graft: GraftEndpointSpan,
}

/// Rust expressions used by a typed graft cut, in source order.
/// 类型化 graft 切口使用的 Rust 表达式，按源码顺序。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GraftExpressions {
    /// The cut target's Rust expression text.
    /// 切口目标的 Rust 表达式原文。
    ///
    /// For a typed range this is the start endpoint; the far one is `cut_end`.
    /// 类型化区间时这是起点，远端在 `cut_end`。
    pub cut: String,
    /// The far endpoint of a typed range, as data.
    /// 类型化区间的远端端点，以数据形式携带。
    pub cut_end: Option<String>,
    /// The replacement's Rust expression text.
    /// 替换件的 Rust 表达式原文。
    pub graft: String,
}

/// Collects the graft declarations an item-position `static_graft_plan!` states.
/// 收集条目位置的 `static_graft_plan!` 所声明的 graft 切口。
struct GraftVisitor<'a> {
    entries: &'a mut Vec<GraftSyntax>,
    error: Option<FaceSyntaxError>,
    /// The `cfg` gates of the modules this visitor is currently inside, outermost
    /// first. A declaration inside a gated module inherits them, because a gate the
    /// build can evaluate belongs to the declaration it guards — not to the parser,
    /// which cannot evaluate any of them (audit `LGC-LG-29`).
    /// 访问器当前所在各模块的 `cfg` 门控，最外层在前。被门控模块里的声明会继承它们：构建能求值
    /// 的门控属于它守护的那条声明，而不属于解析器——解析器一个门控也求值不了（审计 `LGC-LG-29`）。
    module_cfgs: Vec<String>,
    imports: super::super::imports::Imports,
}
impl<'ast> Visit<'ast> for GraftVisitor<'_> {
    fn visit_file(&mut self, file: &'ast syn::File) {
        super::super::imports::collect(&file.items, &mut self.imports);
        syn::visit::visit_file(self, file);
    }

    fn visit_item_const(&mut self, item: &'ast syn::ItemConst) {
        if self.error.is_some() {
            return;
        }
        let gates = item
            .attrs
            .iter()
            .filter(|a| a.path().is_ident("cfg"))
            .filter_map(|a| a.parse_args::<TokenStream>().ok())
            .map(|t| compact(&t))
            .collect::<Vec<_>>();
        if gates.iter().any(|gate| cfg_is_test_only(gate)) {
            return;
        }
        match super::native_graft::entries(item, &self.imports) {
            Ok(Some(entries)) => {
                let own = combined_cfg(&gates, None);
                for mut entry in entries {
                    entry.cfg = combined_cfg(&self.module_cfgs, own.as_deref());
                    self.entries.push(entry);
                }
            }
            Ok(None) => {}
            Err(error) => self.error = Some(error),
        }
    }
    fn visit_item_fn(&mut self, _: &'ast syn::ItemFn) {}

    /// Only a declaration written as an item belongs in the build's plan: a
    /// `runtime_graft_plan!` inside a function is a runtime expression, and a plan
    /// inside `#[cfg(test)] mod tests` describes tests. Both used to be
    /// captured, shipping test cuts into the release table and pinning faces
    /// against pruning.
    /// 只有写成条目的声明才属于构建计划：函数里的 `runtime_graft_plan!`（旧名 `graft_plan!`）是运行时表达式，
    /// `#[cfg(test)] mod tests` 里的计划描述的是测试。两者过去都会被收集，把测试
    /// 切口带进发布表，并让注册面躲过剪枝。
    fn visit_item_macro(&mut self, item: &'ast syn::ItemMacro) {
        self.visit_macro_entry(item);
    }

    /// Walk a module unless the compiler can only drop it, and carry its gate onto
    /// what it contains.
    /// 只要编译器只可能丢弃它就走进去，并把它的门控下传给其中的内容。
    ///
    /// The gate has to be read, not merely noticed: *any* `cfg` attribute used to
    /// skip the whole module body, so a plan inside `#[cfg(feature = "fast")]` was
    /// dropped even when the feature was on — the slot silently kept running the
    /// base implementation while the source said otherwise, and nothing reported
    /// the missing cut (audit `LGC-LG-29`). Only a test-only expression is a gate
    /// this parse can settle, because a build plan is not a test build; every other
    /// gate is *kept* and attached to the declarations inside the module, where the
    /// build step evaluates it with the same rule it already applies to an
    /// entry-level gate (`build_method::graft_view::host_graft_entries`).
    /// 门控要**读**，不能只注意到：过去**任何** `cfg` 属性都会让整个模块体被跳过，于是
    /// `#[cfg(feature = "fast")]` 里的计划即便特性开着也被丢掉——槽位静默地继续跑基座实现，而
    /// 源码写着相反的话，没有任何诊断报出少掉的切口（审计 `LGC-LG-29`）。只有"仅测试"的表达式是
    /// 本次解析能定论的，因为构建计划不是测试构建；其余门控一律**保留**并附到模块内的声明上，
    /// 由构建步骤用它与条目级门控完全相同的那条规则求值
    /// （`build_method::graft_view::host_graft_entries`）。
    fn visit_item_mod(&mut self, item: &'ast syn::ItemMod) {
        let gates = module_cfgs(item);
        if gates.iter().any(|gate| cfg_is_test_only(gate)) {
            return;
        }
        let saved_imports = self.imports.clone();
        if let Some((_, items)) = &item.content {
            super::super::imports::collect(items, &mut self.imports);
        }
        let depth = self.module_cfgs.len();
        self.module_cfgs.extend(gates);
        syn::visit::visit_item_mod(self, item);
        self.module_cfgs.truncate(depth);
        self.imports = saved_imports;
    }
}

/// The `cfg` gates written on a module, in source order.
/// 写在模块上的 `cfg` 门控，按源码顺序。
fn module_cfgs(item: &syn::ItemMod) -> Vec<String> {
    item.attrs
        .iter()
        .filter(|attribute| attribute.path().is_ident("cfg"))
        .filter_map(|attribute| {
            attribute
                .parse_args::<TokenStream>()
                .ok()
                .map(|tokens| compact(&tokens))
        })
        .collect()
}

/// Whether a `cfg` expression is true in a test build and in no other.
/// 一个 `cfg` 表达式是否只在测试构建里为真。
///
/// The same three-state reading `conventions::size` uses for the same question, and
/// for the same reason: `not(test)` and `feature = "…"` are not test-only, so a
/// module carrying them must be visited. `test` alone is test-only; a conjunction is
/// test-only as soon as one conjunct is; a disjunction only when every branch is —
/// an empty `any()` is true in no build at all, not in a test one. An expression this
/// parse cannot read is not one it may skip: keeping it hands the gate to the build
/// step, which refuses what it cannot evaluate.
/// 与 `conventions::size` 对同一个问题使用的同一套三态读法，理由也相同：`not(test)` 与
/// `feature = "…"` 不是仅测试，带着它们的模块必须被访问。单独的 `test` 是仅测试；合取里只要
/// 有一个合取项是仅测试，整体就是；析取要求每个分支都是——空 `any()` 在任何构建里都不成立，
/// 而不是只在测试构建里成立。读不出来的表达式不是可以跳过的：保留它等于把门控交给构建步骤，
/// 而构建步骤会拒绝它求不了值的东西。
fn cfg_is_test_only(expression: &str) -> bool {
    syn::parse_str::<syn::Meta>(expression).is_ok_and(|meta| cfg_meta_is_test_only(&meta))
}

/// Whether one parsed `cfg` operand is test-only.
/// 一个已解析的 `cfg` 操作数是否仅测试。
fn cfg_meta_is_test_only(meta: &syn::Meta) -> bool {
    match meta {
        syn::Meta::Path(path) => path.is_ident("test"),
        syn::Meta::List(list) => {
            let Ok(nested) = list.parse_args_with(
                syn::punctuated::Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated,
            ) else {
                return false;
            };
            if list.path.is_ident("all") {
                nested.iter().any(cfg_meta_is_test_only)
            } else if list.path.is_ident("any") {
                !nested.is_empty() && nested.iter().all(cfg_meta_is_test_only)
            } else {
                false
            }
        }
        syn::Meta::NameValue(_) => false,
    }
}

/// `entry`'s own gate combined with the gates of the modules around it.
/// `entry` 自己的门控与包围它的各模块门控的组合。
///
/// `None` means "no gate at all", which is not the same as an empty gate: the
/// downstream evaluator treats a missing gate as unconditional and refuses one it
/// cannot read.
/// `None` 表示"完全没有门控"，这与空门控不同：下游求值器把缺失的门控当作无条件，而读不出来的
/// 门控会被它拒绝。
fn combined_cfg(module_cfgs: &[String], own: Option<&str>) -> Option<String> {
    let mut gates = module_cfgs.to_vec();
    gates.extend(own.map(str::to_owned));
    match gates.len() {
        0 => None,
        1 => gates.pop(),
        _ => Some(format!("all({})", gates.join(", "))),
    }
}

impl<'a> GraftVisitor<'a> {
    fn visit_macro_entry(&mut self, item: &syn::ItemMacro) {
        let cfg = item.attrs.iter().find_map(|attribute| {
            attribute
                .path()
                .is_ident("cfg")
                .then(|| {
                    attribute
                        .parse_args::<TokenStream>()
                        .ok()
                        .map(|tokens| compact(&tokens))
                })
                .flatten()
        });
        let mac = &item.mac;
        let Some(name) = mac
            .path
            .segments
            .last()
            .map(|segment| segment.ident.to_string())
        else {
            return;
        };
        if !matches!(
            name.as_str(),
            "runtime_graft_plan" | "graft_plan" | "static_graft_plan"
        ) {
            return;
        }
        let tokens = mac.tokens.clone().into_iter().collect::<Vec<_>>();
        let mut index = 0;
        while index < tokens.len() {
            if !matches!(&tokens[index], TokenTree::Ident(value) if value == "cut") {
                index += 1;
                continue;
            }
            index += 1;
            let path_token = tokens.get(index).cloned();
            index += 1;
            // `cut(<expr>)` / `cut(<expr> to <expr>)` names the target with
            // Rust expressions, so the compiler and any editor that resolves
            // Rust paths see the real face instead of a string the tooling
            // would have to interpret.
            // `cut(<expr>)` / `cut(<expr> to <expr>)` 用 Rust 表达式命名目标，
            // 让编译器和任何能解析 Rust 路径的编辑器看到真实的注册面，而不是
            // 需要工具自己解释的字符串。
            // The span travels with the pair: a typed cut's own location is the parenthesis group
            // that spells it, not the macro invocation that carries it. Publishing the macro's
            // location for every entry (what this used to do) put two cuts on one line — the line of
            // `static_graft_plan!(` — and every reader of `graft_plan.tsv` or of a refusal message
            // then pointed at the wrong place (audit 2026-10-06).
            // 位置随这一对一起走：类型化切口自己的位置是拼出它的那个括号组，而不是承载它的宏调用。
            // 过去给每条条目都发布宏的位置，于是两条切口落在同一行——`static_graft_plan!(` 那一行——
            // 而 `graft_plan.tsv` 或拒绝文案的每个读者都被指到了错的地方（审计 2026-10-06）。
            let typed_cut = match &path_token {
                Some(TokenTree::Group(group)) if group.delimiter() == Delimiter::Parenthesis => {
                    let inner = group.stream().into_iter().collect::<Vec<_>>();
                    let pair = match split_typed_range(inner) {
                        Some((start, finish)) => (start, Some(finish)),
                        None => (compact_tokens(group.stream()), None),
                    };
                    Some((pair, group.span()))
                }
                _ => None,
            };
            let (cut, end, cut_span, typed) = match typed_cut {
                Some(((start, finish), span)) => (start, finish, span, true),
                None => {
                    let (path_tokens, path_span, path_text) = match path_token {
                        Some(TokenTree::Group(group)) => (
                            group.stream().into_iter().collect::<Vec<_>>(),
                            group.span(),
                            None,
                        ),
                        Some(TokenTree::Literal(literal)) => {
                            let text = syn::parse_str::<syn::LitStr>(&literal.to_string())
                                .map(|value| value.value())
                                .map_err(|_| {
                                    syntax_error(literal.span(), "graft cut path must be a string")
                                });
                            let Ok(text) = text else {
                                self.error = text.err();
                                return;
                            };
                            (Vec::new(), literal.span(), Some(text))
                        }
                        _ => {
                            self.error = Some(syntax_error(
                                mac.span(),
                                "graft cut expects `[path]`, a path string, or `(<expression>)`",
                            ));
                            return;
                        }
                    };
                    let range = path_text.is_none()
                        && path_tokens.len() == 3
                        && matches!(&path_tokens[1], TokenTree::Ident(value) if value == "to");
                    let resolved = if range {
                        let start = syn::parse2::<syn::LitStr>(path_tokens[0].clone().into())
                            .map_err(|_| {
                                syntax_error(path_span, "graft range start must be a string")
                            });
                        let finish = syn::parse2::<syn::LitStr>(path_tokens[2].clone().into())
                            .map_err(|_| {
                                syntax_error(path_span, "graft range end must be a string")
                            });
                        match (start, finish) {
                            (Ok(start), Ok(finish)) => Ok((start.value(), Some(finish.value()))),
                            (Err(error), _) | (_, Err(error)) => Err(error),
                        }
                    } else {
                        Ok((
                            path_text.unwrap_or_else(|| {
                                syn::parse2::<syn::LitStr>(
                                    path_tokens.clone().into_iter().collect(),
                                )
                                .map(|value| value.value())
                                .unwrap_or_else(|_| {
                                    path_tokens
                                        .iter()
                                        .map(ToString::to_string)
                                        .collect::<String>()
                                })
                            }),
                            None,
                        ))
                    };
                    let (cut, end) = match resolved {
                        Ok(pair) => pair,
                        Err(error) => {
                            self.error = Some(error);
                            return;
                        }
                    };
                    (cut, end, path_span, false)
                }
            };
            if cut.is_empty() {
                self.error = Some(syntax_error(cut_span, "graft cut path is empty"));
                return;
            }
            let full =
                matches!(tokens.get(index), Some(TokenTree::Ident(value)) if value == "full");
            if full {
                index += 1;
                // A range already names every slot it covers, so `full` has no single subtree to
                // replace: the two together are refused **by name** rather than interpreted.
                // Measured: this shape was accepted and emitted `from_id_range(…, true)`, whose
                // meaning — per target, or the run as a whole? — is written down nowhere, and a
                // construct whose meaning the tool cannot vouch for must not be written into a
                // host's source. Both directions of this grammar share the wording.
                // 区间已经点名它覆盖的每一个槽位，因此 `full` 没有"某一棵子树"可替换：两者一起出现时**按名
                // 拒绝**，而不是替它解释。实测：这一形曾被接受并发出 `from_id_range(…, true)`，而它的含义
                // ——逐目标，还是整段？——哪里都没写；而一个含义保证不了的构造，不该被写进宿主的源码。本语法的
                // 两个方向共用这份文案。
                if end.is_some() {
                    self.error = Some(syntax_error(cut_span, range_full_refusal()));
                    return;
                }
            }
            if !matches!(tokens.get(index), Some(TokenTree::Ident(value)) if value == "graft") {
                // `full` written **after** `graft` lands here. Saying where it belongs is the
                // difference between "the grammar is wrong" and a one-token fix, and the measured
                // spelling readers reach for is exactly `… graft(…) full`.
                // 把 `full` 写在 `graft` **之后**会落到这里。点出它该写在哪，是"语法不对"与"改一个词"之间的
                // 差别；而实测读者最容易写出的拼法正是 `… graft(…) full`。
                let misplaced_full =
                    matches!(tokens.get(index), Some(TokenTree::Ident(value)) if value == "full");
                self.error = Some(syntax_error(
                    cut_span,
                    if misplaced_full {
                        "`full` is written **before** `graft`: `cut(…) [to …] full graft(…)`"
                    } else {
                        "graft cut expects `graft <implementation>`"
                    },
                ));
                return;
            }
            index += 1;
            let graft_token = tokens.get(index).cloned();
            let mut graft_span = None;
            let (graft, graft_expr) = match graft_token {
                Some(TokenTree::Group(group)) if group.delimiter() == Delimiter::Parenthesis => {
                    graft_span = Some(group.span());
                    let text = compact_tokens(group.stream());
                    (text.clone(), Some(text))
                }
                Some(TokenTree::Literal(value)) => {
                    let Ok(text) = syn::parse_str::<syn::LitStr>(&value.to_string()) else {
                        self.error = Some(syntax_error(
                            value.span(),
                            "graft implementation must be a string literal or `(<expression>)`",
                        ));
                        return;
                    };
                    (text.value(), None)
                }
                _ => {
                    self.error = Some(syntax_error(
                        cut_span,
                        "graft expects a string literal or `(<expression>)`",
                    ));
                    return;
                }
            };
            // A cut names both sides the same way, so the generated table
            // never has to mix a resolved identity with an unresolved name.
            // 一条切口的命名方式必须一致，生成表因此不会混合已解析身份与未解析名称。
            // Both spellings normalize to the declaration form here too: the array grammar used
            // to keep `crate::catalog::<parent>::<KIND>` verbatim while the const grammar normalized
            // it, so a catalog cut was resolvable in one host shape and "unresolved" in the other.
            // 两种拼法在这里也归一成声明形式：数组语法过去把 `crate::catalog::<父>::<KIND>` 原样保留，
            // 而 const 语法会归一，于是同一条目录切口在一种宿主形状里可解析、在另一种里"unresolved"。
            let expressions = match (typed, graft_expr) {
                (true, Some(graft)) => Some(GraftExpressions {
                    cut: crate::syntax::catalog_reference(&cut),
                    cut_end: end.as_deref().map(crate::syntax::catalog_reference),
                    graft: crate::syntax::catalog_reference(&graft),
                }),
                (false, None) => None,
                _ => {
                    self.error = Some(syntax_error(
                        cut_span,
                        "a graft cut must name both sides with Rust expressions or both with strings",
                    ));
                    return;
                }
            };
            // A trailing `full` is refused rather than ignored. The grammar puts `full` **before**
            // `graft`, and silently dropping it changes what the entry means: the cut then covers the
            // node instead of the whole subtree. Measured — this spelling was *accepted*, with
            // `full: false`, so the author's `full` had no effect and nothing said so.
            // 尾随的 `full` 要拒绝而不是忽略。语法把 `full` 放在 `graft` **之前**，而静默丢掉它会改变这条条目
            // 的含义：切口于是只覆盖那个节点，而不是整棵子树。实测——这种拼法曾被**接受**且 `full: false`，
            // 于是作者的 `full` 毫无作用，而且没有任何东西说明这件事。
            if let Some(TokenTree::Ident(stray)) = tokens.get(index + 1)
                && stray == "full"
            {
                self.error = Some(syntax_error(
                    stray.span(),
                    "`full` is written **before** `graft`: `cut(…) [to …] full graft(…)`",
                ));
                return;
            }
            // The two endpoints stay separate fields; joining them into `cut`
            // would make a path that literally contains `" to "` indistinguish-
            // able from a range at every later consumer.
            // 两个端点保持独立字段；把它们拼进 `cut` 会让字面含有 `" to "` 的路径
            // 在之后每个消费方那里都与区间无法区分。
            let is_typed_range = typed && end.is_some();
            self.entries.push(GraftSyntax {
                cut,
                cut_end: end,
                graft,
                full,
                location: location(cut_span),
                cfg: combined_cfg(&self.module_cfgs, cfg.as_deref()),
                expressions,
                spans: (!is_typed_range && typed).then(|| {
                    // The macro grammar hands each endpoint as a parenthesized group; the span
                    // includes the delimiters, and the planner trims them when it addresses the
                    // expression inside. A typed range keeps `None`: the planner refuses a range
                    // that names a moving face before it asks for spans.
                    // 宏语法把每个端点作为括号组交出；跨度包含分隔符，规划器在定位内部表达式时去掉它们。
                    // 类型化区间保留 `None`：规划器在索取跨度之前就拒绝点名被搬面的区间。
                    GraftSpans {
                        cut: endpoint_span(cut_span),
                        cut_end: None,
                        graft: endpoint_span(graft_span.unwrap_or(cut_span)),
                    }
                }),
            });
            index += 1;
        }
    }
}

/// Write one graft declaration the way the macro grammar reads it.
/// 按宏语法的读法写出**一条** graft 声明。
///
/// This is the **writing** half of the pair whose reading half is `graft_entries`, and it lives
/// beside it so the two spellings cannot drift: the pin renders a declaration, parses it back with
/// `graft_entries`, and requires the same structure.
/// 这是与 `graft_entries` 成对的那**写出**的一半，摆在它旁边正是为了让两种拼写无法漂移：钉子渲染一条
/// 声明、用 `graft_entries` 解析回来，并要求结构相同。
///
/// **It renders the typed-expression form only, and refuses everything else.** `cut(<expr>)` is what
/// a tool that knows a face's Rust path can write without guessing; the logical-path forms (`cut
/// [..]`, `cut "a string"`, ranges of string literals) are shapes whose spelling has a reader's
/// meaning this function cannot verify, so it refuses them rather than emitting something that
/// parses into a different declaration than the caller meant. A narrow renderer that refuses beats a
/// wide one that guesses: this writes **source code into a host**.
/// **它只渲染类型化表达式那一形，别的形状一律拒绝。** `cut(<表达式>)` 是"知道某个面的 Rust 路径"的工具
/// 无需猜测就能写出的东西；逻辑路径那些形状（`cut [..]`、`cut "串"`、字符串字面量的区间）的拼写带有读者
/// 才知道的含义，本函数无法核实，因此**拒绝**而不是发射一条"解析成另一种声明"的文本。窄而会拒绝的渲染器
/// 胜过宽而会猜的：它写进的是**宿主的源码**。
///
/// The one trap the grammar itself has: the parser treats a top-level `to` ident inside `cut(...)` as
/// a range separator, so a path whose **segment** is `to` would come back as a range. That is
/// refused, not escaped.
/// 语法自带的一个陷阱：解析器把 `cut(...)` 里顶层的 `to` 识别符当作区间分隔符，因此**段名**为 `to` 的路径
/// 会被读回成区间。这里**拒绝**它，而不是转义。
pub fn render_graft_expression(
    cut: &str,
    cut_end: Option<&str>,
    full: bool,
    graft: &str,
) -> Result<String, FaceSyntaxError> {
    for (what, value) in [("cut", cut), ("graft", graft)] {
        if value.trim().is_empty() {
            return Err(spelling_error(format!("{what} must not be empty")));
        }
        // The logical-path and literal forms are a different spelling of the same grammar; a value
        // carrying their punctuation would parse as that spelling instead of an expression.
        // 逻辑路径与字面量是同一语法的另一种拼写；带它们标点的值会被解析成那种拼写而不是表达式。
        if value.contains(['"', '[', ']', '\n', '\r']) {
            return Err(spelling_error(format!(
                "{what} `{value}` carries punctuation that belongs to another spelling of this \
                 grammar; this renderer writes the typed-expression form only"
            )));
        }
        if value.split("::").any(|segment| segment.trim() == "to") {
            return Err(spelling_error(format!(
                "{what} `{value}` has a `to` segment, which the parser reads as a range separator"
            )));
        }
    }
    if let Some(end) = cut_end
        && (end.trim().is_empty() || end.contains(['"', '[', ']', '\n', '\r']))
    {
        return Err(spelling_error(format!(
            "the range end `{end}` is not an expression this renderer can write"
        )));
    }
    // The reader refuses this pair, so the writer must not produce it: a generator that emits what
    // its own parser rejects is how a tool writes a declaration into a host that cannot be read back.
    // 读取方拒绝这一对，因此写入方不得产出它：一个发射自己解析器所拒之物的生成器，正是"把读不回来的声明
    // 写进宿主"的成因。
    if cut_end.is_some() && full {
        return Err(spelling_error(range_full_refusal()));
    }
    let range = cut_end.map_or(String::new(), |end| format!(" to {end}"));
    let whole = if full { " full" } else { "" };
    Ok(format!("cut({cut}{range}){whole} graft({graft})"))
}

/// Why `full` on a range cut is refused — the one wording both directions of this grammar use.
/// 为何拒绝"区间 + `full`"——本语法两个方向共用的那一份文案。
fn range_full_refusal() -> String {
    "`full` on a range cut has no defined meaning: a range already names every slot it covers, so \
     there is no one subtree for `full` to replace. Write one cut per face, or drop `full`."
        .to_owned()
}

/// A spelling this renderer refuses to write, with the reason and no location (the caller has not
/// written it yet, so there is nothing to point at).
/// 本渲染器拒绝写出的拼写，带原因、不带位置（调用方还没写下它，因此没有东西可指）。
fn spelling_error(message: String) -> FaceSyntaxError {
    FaceSyntaxError {
        message,
        location: None,
    }
}

/// Parse static and dynamic graft declarations without executing them.
/// 解析静态和动态 graft 声明，不执行宏。
pub fn graft_entries(source: &str) -> Result<Vec<GraftSyntax>, FaceSyntaxError> {
    let file = super::super::nesting::parse_file(source)?;
    let mut entries = Vec::new();
    let mut visitor = GraftVisitor {
        entries: &mut entries,
        error: None,
        module_cfgs: Vec::new(),
        imports: Default::default(),
    };
    visitor.visit_file(&file);
    visitor.error.map_or(Ok(entries), Err)
}

#[cfg(test)]
#[path = "graft_tests.rs"]
mod graft_tests;
