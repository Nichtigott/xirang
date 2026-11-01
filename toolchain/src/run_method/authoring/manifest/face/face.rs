//! Face field editing and registry-rule derivation.
//! 注册面字段编辑与注册规范推导。
//!
//! Rendering lives in `render.rs`; this file keeps the edit and rule half.
//! 渲染位于 `render.rs`；本文件保留编辑与规范部分。

use std::path::{Path, PathBuf};

use super::super::FaceManifest;
use crate::run_method::RuntimeCheckSpec;
use crate::run_method::authoring::context::{
    identity_base, legacy_rule_path_for_source, rule_path_for_source, validate_kind_name,
};
use crate::run_method::authoring::parse::*;

#[path = "render.rs"]
mod render;

impl FaceManifest {
    /// Hand this face its identity, for a writer to **bake** into the file it writes.
    /// 把这个面的身份交给写者，供它**烘焙**进自己写下的文件。
    ///
    /// A hand-written face cannot compute its own `NodeId` in a const context (no hasher, no view of its
    /// own relative path), so the tool that writes the face writes the id it already has — and the build's
    /// per-face identity assertion is what keeps a moved or renamed file from silently re-keying every
    /// graft record that names this face.
    /// 手写的面在常量上下文里算不出自己的 `NodeId`（没有哈希器、也看不到自己的相对路径），因此写下这个面的
    /// 工具写下它已经握着的那个 id——而构建期那句逐面身份断言，正是让"被移动或改名"不会静默地换掉每一条点名
    /// 这个面的嫁接记录的那一步。
    pub(crate) fn set_identity(&mut self, id: xirang_kernel::identity::NodeId) {
        let bytes = id
            .into_bytes()
            .iter()
            .map(u8::to_string)
            .collect::<Vec<_>>()
            .join(", ");
        self.values.insert("id".to_owned(), bytes);
    }

    pub(crate) fn rule_source_path(&self) -> Result<PathBuf, String> {
        let source = Path::new(self.values.get("source").map(String::as_str).unwrap_or(""));
        let directory = source
            .parent()
            .ok_or_else(|| "generated face has no module directory".to_owned())?;
        if let Some(declared) = self.values.get("registry_rule_path")
            && !declared.trim().is_empty()
        {
            // The recorded path is **package-relative** (the writer stores it that way), so it joins
            // the package root. The historical `strip_prefix("src/")` + identity-base join was a
            // spelling of the same file for `src/` faces and the wrong file for everything else.
            // 记录的路径是**包内相对**（写入方就是这么存的），因此拼到包根上。历史上的
            // `strip_prefix("src/")` + 身份基准拼接对 `src/` 下的面是同一个文件，对其余一切是错的文件。
            return Ok(super::super::context::package_root().join(declared));
        }
        let canonical = identity_base()?
            .join(directory)
            .join("registry_rule/registry_rule.rs");
        if canonical.is_file() {
            return Ok(canonical);
        }
        // Existing faces authored before the dedicated folder remain valid.
        // 兼容早期使用 registry/rules/rules.rs 的注册面。
        Ok(identity_base()?.join(
            Path::new(&legacy_rule_path_for_source(
                self.values.get("source").map(String::as_str).unwrap_or(""),
            ))
            .strip_prefix("src/")
            .unwrap_or_else(|_| Path::new("")),
        ))
    }

    pub(crate) fn rule_source_path_string(&self) -> Result<String, String> {
        if let Some(declared) = self.values.get("registry_rule_path")
            && !declared.trim().is_empty()
        {
            return Ok(declared.clone());
        }
        Ok(rule_path_for_source(
            self.values.get("source").map(String::as_str).unwrap_or(""),
        ))
    }

    pub(crate) fn rule_module_path(&self) -> Result<String, String> {
        let rule_path = self.rule_source_path_string()?;
        let module = rule_path
            .strip_prefix("src/")
            .unwrap_or(&rule_path)
            .rsplit_once('/')
            .map_or_else(|| rule_path.as_str(), |(directory, _)| directory)
            .replace('/', "::");
        Ok(format!("crate::{module}::REGISTRATION_RULE"))
    }

    pub(crate) fn render_rule_source(&self) -> Result<Option<String>, String> {
        if self.values.get("needs_registry").map(String::as_str) != Some("true") {
            return Ok(None);
        }
        let rule = render_registration_rule(
            self.values
                .get("registration_rule")
                .map(String::as_str)
                .unwrap_or("ANY"),
        )?;
        Ok(Some(format!(
            "//! Registry rule.\n//! 注册规范。\n\nuse crate::RegistrationRule;\n\npub const REGISTRATION_RULE: RegistrationRule = {rule};\n"
        )))
    }

    /// Apply one field a caller can edit.
    /// 应用调用方可以编辑的一个字段。
    ///
    /// The accepted set is exactly the set the renderer writes back: a key the
    /// template never emits is refused here instead of being stored and silently
    /// dropped on the next save. `registry_name`, `handle`, and `params` were in
    /// the accepted set while nothing rendered them — `handle` could not even be
    /// parsed (the kernel's field word list does not carry it), so an edit to any
    /// of the three returned `Ok` and changed nothing (audit `LG-38`).
    /// 接受集恰好就是渲染器会写回的集合：模板从不发射的键在这里被拒，而不是被存下来、在下次保存时
    /// 被静默丢掉。`registry_name`、`handle` 与 `params` 过去在接受集里，却没有任何东西渲染它们——
    /// `handle` 甚至无法被解析（内核的字段词表里没有它），因此对这三者中任何一个的编辑都会返回
    /// `Ok` 而什么都不改变（审计 `LG-38`）。
    pub(crate) fn edit(&mut self, field: &str, value: &str) -> Result<(), String> {
        match field {
            "kind"
            | "preset"
            | "parts"
            | "name_zh"
            | "name_en"
            | "summary_zh"
            | "summary_en"
            | "stable_name"
            | "exports"
            | "getting_from_other_registry"
            | "registration_rule"
            | "admission"
            | "handle_traits"
            | "handle_contracts"
            | "part_traits"
            | "part_contracts"
            | "requires"
            | "provides"
            | "runtime_checks"
            | "flow"
            | "flow_provider" => {
                if value.contains(['\n', '\r']) {
                    return Err("field value cannot contain a newline".to_owned());
                }
                if field == "kind" {
                    validate_kind_name(value)?;
                }
                if field == "registration_rule" {
                    parse_registration_rule_owned(value)?;
                }
                if field == "admission" {
                    parse_admission_owned(value)?;
                }
                if field == "getting_from_other_registry" {
                    parse_optional_source(value)?;
                }
                if field == "requires" {
                    parse_requirements(value)?;
                }
                if field == "runtime_checks" {
                    RuntimeCheckSpec::parse_list(value)?;
                }
                if field == "flow" {
                    parse_flow_value(value)?;
                }
                if field == "flow_provider" && !value.trim().is_empty() {
                    validate_flow_provider(value)?;
                }
                // The key is the field name: `admission` is stored under its own
                // name like every other field in this arm, and the difference that
                // looks like it belongs here (the compact and expression spellings)
                // is handled in the validation above.
                // 键就是字段名：`admission` 与这一支里的其它字段一样存自己的名字，而看起来属于这里
                // 的那点差异（紧凑与表达式两种拼法）由上面的校验处理。
                self.values.insert(field.to_owned(), value.to_owned());
                Ok(())
            }
            "needs_registry" if matches!(value, "true" | "false") => {
                self.values.insert(field.to_owned(), value.to_owned());
                Ok(())
            }
            "needs_registry" => Err("needs_registry must be `true` or `false`".to_owned()),
            _ => Err(format!(
                "field `{field}` is not editable; use the registration face fields"
            )),
        }
    }

    /// Whether this face owns a generated registry-rule source file.
    /// 该注册面是否拥有生成的注册规则源文件。
    ///
    /// A face that declares a rule of its own needs one even when it owns no
    /// child registry. The two callers used to disagree — the renderer emitted
    /// `registry_rule: <module>::REGISTRATION_RULE` for a custom rule while the
    /// editor wrote the module only for a registry owner — so saving a leaf face
    /// with a custom rule produced source that referenced a file nobody wrote.
    /// 声明了自己规则的注册面即使不拥有子注册机也需要它。两个调用方过去口径不一致——
    /// 渲染器会为自定义规则发射 `registry_rule: <模块>::REGISTRATION_RULE`，而编辑器
    /// 只为拥有注册机的面写该模块——于是保存一个带自定义规则的叶子面会生成引用不存在
    /// 文件的源码。
    pub(crate) fn owns_rule_source(&self) -> bool {
        let rule = self
            .values
            .get("registration_rule")
            .map_or("", String::as_str)
            .trim();
        self.values.get("needs_registry").map(String::as_str) == Some("true")
            || (!rule.is_empty() && rule != "ANY")
    }

    /// Whether `registry_rule:` is redundant because the canonical sibling rule
    /// derives it.
    /// `registry_rule:` 是否因为同目录规范规则已经推导出它而多余。
    ///
    /// Only a face that owns a registry derives the field: the resolver defaults
    /// every other face to the permissive rule, which is what a child face wants,
    /// since the rule that governs it belongs to its parent. A face whose rule was
    /// moved elsewhere keeps naming it.
    /// 只有拥有注册机的面才推导该字段：解析器把其余面默认成宽松规则——这正是子面的需要，
    /// 因为管它的规则属于它的父级。规则被挪到别处的面仍然写出它。
    pub(crate) fn derives_rule_from_the_sibling(&self) -> bool {
        if self.values.get("needs_registry").map(String::as_str) != Some("true") {
            return false;
        }
        let source = self.values.get("source").map(String::as_str).unwrap_or("");
        let declared = self
            .values
            .get("registry_rule_path")
            .map_or("", String::as_str)
            .trim();
        declared.is_empty() || declared == rule_path_for_source(source)
    }
}

#[cfg(test)]
mod rule_source_ownership_tests {
    use super::super::FaceManifest;
    use std::collections::BTreeMap;

    /// The renderer and the editor must agree on when a face owns a generated
    /// rule source: a custom rule needs one even for a leaf face, and a face that
    /// owns a registry needs one even with `ANY`. They used to disagree, so
    /// saving a leaf face with a custom rule wrote source referencing a module
    /// nobody created.
    /// 渲染器与编辑器必须对"何时拥有生成的规则源"口径一致：自定义规则即使在叶子面上也
    /// 需要它，而拥有注册机的面即使规则是 `ANY` 也需要它。两者过去不一致，于是保存一个
    /// 带自定义规则的叶子面会写出引用不存在模块的源码。
    #[test]
    fn a_custom_rule_makes_a_leaf_face_own_its_rule_source() {
        let manifest = |needs_registry: &str, rule: &str| {
            let mut manifest = FaceManifest {
                values: BTreeMap::new(),
            };
            manifest
                .values
                .insert("needs_registry".to_owned(), needs_registry.to_owned());
            manifest
                .values
                .insert("registration_rule".to_owned(), rule.to_owned());
            manifest
        };

        assert!(!manifest("false", "ANY").owns_rule_source());
        assert!(manifest("false", "parts:paint").owns_rule_source());
        assert!(manifest("true", "ANY").owns_rule_source());
    }

    /// A face that owns a registry and keeps its rule at the canonical path does
    /// not repeat that path to reference the rule: the declaration resolves it
    /// from the sibling module instead. A leaf face, or one whose rule lives
    /// elsewhere, still names it.
    /// 拥有注册机、且规则就在规范路径上的面不必为了引用规则而重复该路径：声明改为从同目录
    /// 模块推导它。叶子面、或规则放在别处的面仍然写出它。
    #[test]
    fn a_registry_face_derives_the_rule_it_keeps_beside_it() {
        let manifest = |needs_registry: &str, declared_path: &str| {
            let mut manifest = FaceManifest {
                values: BTreeMap::new(),
            };
            // `source` is relative to the package `src` directory, which is the
            // form the manifest keeps and `rule_path_for_source` consumes.
            // `source` 相对包的 `src` 目录，这正是清单保存、`rule_path_for_source`
            // 消费的形式。
            manifest
                .values
                .insert("source".to_owned(), "widget/widget.rs".to_owned());
            manifest
                .values
                .insert("needs_registry".to_owned(), needs_registry.to_owned());
            manifest
                .values
                .insert("registration_rule".to_owned(), "parts:paint".to_owned());
            if !declared_path.is_empty() {
                manifest
                    .values
                    .insert("registry_rule_path".to_owned(), declared_path.to_owned());
            }
            manifest
        };

        // Canonical path, whether it is written out or left to the default.
        assert!(manifest("true", "").derives_rule_from_the_sibling());
        assert!(
            manifest("true", "src/widget/registry_rule/registry_rule.rs")
                .derives_rule_from_the_sibling()
        );
        // A rule that was moved elsewhere keeps its explicit reference.
        assert!(!manifest("true", "src/widget/other/rules.rs").derives_rule_from_the_sibling());
        // A face that owns no registry defaults to the permissive rule, so it
        // cannot derive the sibling without changing what it enforces.
        assert!(!manifest("false", "").derives_rule_from_the_sibling());
        assert!(
            !manifest("false", "src/widget/registry_rule/registry_rule.rs")
                .derives_rule_from_the_sibling()
        );
    }
}

/// The stack the `flow_provider` validator parses on.
/// `flow_provider` 验证器解析时所用的栈。
///
/// The kernel's nesting guard admits up to 128 levels, and 128 levels of `syn`
/// recursion is what an eight-megabyte stack is for — the amount the main thread
/// gets. Naming the size here is what makes the guard's budget mean the same
/// thing for every caller.
/// 内核的嵌套守卫最多放行 128 层，而 `syn` 递归 128 层正是八兆栈（主线程拿到的量）能承受的。
/// 把大小写在这里，守卫的预算才对每个调用方都意味着同一件事。
const FLOW_PROVIDER_STACK_BYTES: usize = 8 * 1024 * 1024;

/// Validate one `flow_provider` type path.
/// 校验一条 `flow_provider` 类型路径。
///
/// Two things are needed, and the edit path had neither. The parse goes through
/// the kernel's guarded implementation (`render_flow_provider`, which measures
/// nesting before `syn` recurses), and it runs on a *fixed* stack, because a
/// guard budget is only meaningful against a known amount of stack. Measured by
/// the audit round that filed this: depth 300 of `A<A<…>>` aborts an
/// eight-megabyte stack, and depth 100 aborts a two-hundred-fifty-six-kilobyte
/// one — the same depth that parses here. The edit entry runs before the
/// renderer, so without this the editor and the MCP bridge died before the
/// guarded renderer could speak.
/// 需要两件事，而编辑路径两件都没有。解析走内核那个带守卫的实现（`render_flow_provider`，它
/// 先量嵌套、再让 `syn` 递归），且跑在**固定**栈上，因为守卫的预算只对着已知的栈量才有意义。
/// 审计轮实测：`A<A<…>>` 的 300 层会让八兆栈 abort，100 层会让 256 KiB 栈 abort——而同一个
/// 深度在这里解析得好好的。编辑入口先于渲染器运行，因此没有这两件事时，编辑器与 MCP 桥会在带
/// 守卫的渲染器开口之前就死掉。
fn validate_flow_provider(value: &str) -> Result<(), String> {
    let source = value.to_owned();
    std::thread::Builder::new()
        .name("xirang-flow-provider".to_owned())
        .stack_size(FLOW_PROVIDER_STACK_BYTES)
        .spawn(move || {
            render_flow_provider(&source)
                .map(|_| ())
                .map_err(|error| error.to_string())
        })
        .map_err(|error| format!("cannot validate flow_provider: {error}"))?
        .join()
        .unwrap_or_else(|_| Err("flow_provider validation panicked".to_owned()))
}

#[cfg(test)]
mod flow_provider_nesting_tests {
    use super::super::FaceManifest;
    use std::collections::BTreeMap;

    /// A nested generic type path of `depth` levels: `A<A<…<u8>…>>`.
    /// `depth` 层的嵌套泛型类型路径：`A<A<…<u8>…>>`。
    fn nested(depth: usize) -> String {
        let mut value = "A<".repeat(depth);
        value.push_str("u8");
        value.push_str(&">".repeat(depth));
        value
    }

    fn manifest_editing(field: &str, value: &str) -> Result<(), String> {
        let mut manifest = FaceManifest {
            values: BTreeMap::new(),
        };
        manifest.edit(field, value)
    }

    /// A `flow_provider` deep enough to overflow the parser is *refused*, not
    /// fatal. The edit path called `syn::parse_str` directly while the kernel's
    /// one guarded implementation only ran at render time, so the editor and the
    /// MCP bridge aborted the process before the guard could speak: a stack
    /// overflow is not a catchable panic, so there was no `Err` to report.
    /// 深到能让解析器溢出的 `flow_provider` 会被**拒绝**，而不是致命。编辑路径直接调
    /// `syn::parse_str`，而内核唯一的带守卫实现只在渲染时运行，因此编辑器与 MCP 桥会在守卫
    /// 说话之前就 abort 进程：栈溢出不是可捕获的 panic，没有任何 `Err` 可以报。
    ///
    /// Red before the fix: this test's process died with
    /// `fatal runtime error: stack overflow` at depth 300.
    /// 修前为红：本测试进程在 300 层时以 `fatal runtime error: stack overflow` 死亡。
    #[test]
    fn a_pathologically_nested_flow_provider_is_refused_not_fatal() {
        let error = manifest_editing("flow_provider", &nested(300))
            .expect_err("nesting above the kernel's limit is refused");
        assert!(
            error.contains("128"),
            "the refusal must name the nesting limit: {error}"
        );
    }

    /// The answer does not depend on how much stack the caller happens to have:
    /// an admitted depth parses on the validator's own fixed stack even when the
    /// caller is a 256 KiB thread. Red before the fix — the *same* value that
    /// parses on a test thread aborted here, because the parse ran on the
    /// caller's stack.
    /// 答案不取决于调用方恰好有多少栈：被放行的深度在验证器自己的固定栈上能解析，即使调用方
    /// 是一条 256 KiB 的线程。修前为红——同一个在测试线程上能解析的值在这里 abort，因为解析跑在
    /// 调用方的栈上。
    #[test]
    fn the_answer_does_not_depend_on_the_callers_stack() {
        let value = nested(100);
        assert_eq!(value.len(), 302, "200 opener bytes + `u8` + 100 closers");
        let verdict = std::thread::Builder::new()
            .name("xirang-tiny-stack".to_owned())
            .stack_size(256 * 1024)
            .spawn(move || manifest_editing("flow_provider", &value))
            .expect("spawn the tiny-stack caller")
            .join()
            .expect("the validation must not abort the process");
        assert!(
            verdict.is_ok(),
            "an admitted path is valid however small the caller's stack is: {verdict:?}"
        );
    }
}
