//! Module creation transaction: stage, validate, then write.
//! 模块创建事务：先暂存校验，再写入源码树。

use super::*;

/// Everything a create would write, before anything is written.
/// 一次新建会写下的全部内容，在写下任何东西之前。
pub(crate) struct NewModulePlan {
    /// The face file's absolute path.
    /// 注册面文件的绝对路径。
    pub(crate) source: PathBuf,
    /// The face file's path relative to the identity base, for messages.
    /// 注册面相对身份基准的路径，用于消息。
    pub(crate) source_relative: PathBuf,
    /// The exact file text the plan writes.
    /// 计划写入的确切文件文本。
    pub(crate) source_text: String,
    /// The generated registry-rule file, when the face owns one.
    /// 该面拥有时生成的注册规则文件。
    pub(crate) rule: Option<(PathBuf, String)>,
    /// The registration snapshot the staged registry accepted.
    /// 暂存注册树接受的注册快照。
    pub(crate) info: RegistrationSnapshot,
    /// The resolved parent identity, for the message and for callers that plan a move.
    /// 解析后的父身份，供消息与计划移动的调用方使用。
    pub(crate) parent: NodeId,
}

pub(crate) fn create_module(
    registry: &Registry,
    name: &str,
    parent: NodeId,
    configured: Option<&ModuleFaceValues<'_>>,
) -> Result<(AuthoringChange, RegistrationSnapshot), String> {
    let plan = plan_create_module(
        registry,
        name,
        parent,
        None,
        xirang_multilang::source_ref::SourceBase::Identity,
        configured,
    )?;
    apply_create_module(&plan)
}

/// Plan a create without writing it: the caller reviews or applies the plan.
/// 计划一次新建而不写下它：调用方审阅或落盘该计划。
pub(crate) fn plan_create_module(
    registry: &Registry,
    name: &str,
    parent: NodeId,
    parent_module: Option<&str>,
    parent_base: xirang_multilang::source_ref::SourceBase,
    configured: Option<&ModuleFaceValues<'_>>,
) -> Result<NewModulePlan, String> {
    // The legacy root token means "this Registry's root". Studio uses a
    // package namespace, so its concrete root identity is intentionally not
    // the process-wide legacy constant.
    // 旧根标识表示“当前 Registry 的根”。Studio 使用包命名空间，因此具体根
    // 标识不会等于进程级旧常量。
    let parent = if parent == ROOT_NODE_ID {
        registry.id()
    } else {
        parent
    };
    let parent_face = if parent == registry.id() {
        None
    } else {
        let face = registry
            .find_registry(parent)
            .ok_or_else(|| format!("parent `{parent}` is not registered"))?;
        if !face.needs_registry {
            return Err(format!("parent `{parent}` does not own a Registry"));
        }
        Some(face)
    };

    // The identity path is relative to `identity_base`, the file lives under the
    // scan root. A root-level face keeps the scan root's own offset from the
    // identity base (`src/`, or the target directory) so a face written under a
    // `[lib] path = "host/lib.rs"` layout lands where the build reads it.
    // 身份路径相对 `identity_base`，文件住在扫描根之下。根级面保留扫描根相对身份基准的偏移
    // （`src/`，或目标目录），因此在 `[lib] path = "host/lib.rs"` 布局下写出的面会落在构建读取的地方。
    let scan_root = scan_root()?;
    let identity_base = identity_base()?;
    let identity_prefix = scan_root
        .strip_prefix(&identity_base)
        .unwrap_or_else(|_| Path::new(""))
        .to_path_buf();
    // A child inherits its parent's **place**, so the spelling base travels with the parent: an
    // island parent's `source` is package-relative, and its children stay in the island instead of
    // being pulled under the scan root.
    // 子面继承父级的**位置**，因此拼写基准随父一起传递：孤岛父级的 `source` 是包内相对，它的子面留在
    // 孤岛里，而不是被拉进扫描根之下。
    let (relative_dir, source_base) = match parent_face {
        None => (
            identity_prefix.join(name),
            xirang_multilang::source_ref::SourceBase::Identity,
        ),
        Some(face) => {
            let source = Path::new(&face.source.file);
            if source.is_absolute() || source.components().any(is_parent_component) {
                return Err("external parent source cannot be authored by this package".to_owned());
            }
            (
                source
                    .parent()
                    .unwrap_or_else(|| Path::new(""))
                    .join("object")
                    .join(name),
                parent_base,
            )
        }
    };
    plan_new_face(
        registry,
        name,
        parent,
        parent_face
            .map(|face| face.source.file.as_str())
            .unwrap_or("<root>"),
        parent_face.map(|face| face.kind.as_str()).unwrap_or("root"),
        None,
        parent_module,
        relative_dir,
        source_base,
        configured,
    )
}

/// Plan a face at one concrete directory, with the parent spelling handed in.
/// 在一个具体目录里计划注册面，父的拼法由参数给出。
///
/// This is the half both create paths share: the ordinary path resolves the parent from the
/// registry, while a face under a declared (foreign-language) parent resolves it from the
/// declaration text and hands in the reference the Rust renderer must write. Keeping the rendering
/// here means the foreign-parent case cannot drift from the ordinary one.
/// 这是两条新建路径共用的那一半：普通路径从注册树解析父，而挂在声明式（外语）父下的面从声明文本解析
/// 父，并传入 Rust 渲染器必须写下的引用。把渲染留在这里，外语父的情形就不会与普通情形漂移。
#[allow(clippy::too_many_arguments)]
fn plan_new_face(
    registry: &Registry,
    name: &str,
    parent: NodeId,
    parent_source: &str,
    parent_kind: &str,
    parent_reference: Option<&str>,
    parent_module: Option<&str>,
    relative_dir: PathBuf,
    source_base: xirang_multilang::source_ref::SourceBase,
    configured: Option<&ModuleFaceValues<'_>>,
) -> Result<NewModulePlan, String> {
    let scan_root = scan_root()?;
    let identity_base = identity_base()?;
    let source_relative = relative_dir.join(format!("{name}.rs"));
    // The physical path follows the base the placement was written against: the identity base for
    // the historical layout, the package root for a face placed outside it. The record spelling
    // (`source_relative`) is the same in both cases; only where it is joined differs.
    // 物理路径跟随落点所写的基准：历史布局是身份基准，放在它之外的面是包根。记录拼写
    // （`source_relative`）两种情况相同，只有拼接的起点不同。
    let source = match source_base {
        xirang_multilang::source_ref::SourceBase::Identity => identity_base.join(&source_relative),
        xirang_multilang::source_ref::SourceBase::Package => {
            super::super::context::package_root().join(&source_relative)
        }
    };
    if source.exists() {
        return Err(format!(
            "module `{}` already exists",
            source_relative.display()
        ));
    }
    // The kernel owns the one PascalCase derivation; this used to be a local
    // copy that split only on `_`, so a name with another separator produced a
    // kind that is not a valid Rust type name.
    // PascalCase 推导只有内核那一份；这里曾是一份只按 `_` 切分的本地副本，因此名字里
    // 出现别的分隔符时会产出并非合法 Rust 类型名的 kind。
    let kind = xirang_kernel::authoring::pascal_case(name);
    let mut face = FaceManifest::new(
        name,
        &kind,
        parent,
        parent_source,
        parent_kind,
        &normalized_path(&source_relative),
    );
    if let Some(reference) = parent_reference {
        face.values
            .insert("parent_reference".to_owned(), reference.to_owned());
    }
    if let Some(module) = parent_module {
        // The Rust module address is a fact from the generated module chain, not a re-derivation
        // from `parent_source`: a custom `[lib] path` and an island both make the two differ.
        // Rust 模块地址是生成模块链的事实，不是从 `parent_source` 重新推导：自定义 `[lib] path` 与
        // 孤岛都会让两者不同。
        face.values
            .insert("parent_module".to_owned(), module.to_owned());
    }
    if let Some(configured) = configured {
        apply_module_face_values(&mut face, configured, FaceWrite::Create)?;
    }
    // A face that owns a registry gets its rule beside **this file**, and the recorded path is
    // package-relative: the kernel's historical default hardcodes `src/`, which put an island's rule
    // under `src/components/…` — a file no build reads, while the generated module looked for it
    // beside the face (measured: `no sibling registry_rule module`).
    // 拥有注册机的面把规则放在**本文件旁边**，而记录的路径是包内相对：内核的历史默认值写死 `src/`，于是
    // 孤岛的规则被写进 `src/components/…`——一个没有构建会读的文件，而生成的模块却去面旁边找它（实测：
    // `no sibling registry_rule module`）。
    if face.values.get("needs_registry").map(String::as_str) == Some("true") {
        let rule = source
            .parent()
            .ok_or_else(|| "generated face has no module directory".to_owned())?
            .join("registry_rule/registry_rule.rs");
        let package = xirang_multilang::source_ref::package_source(
            &super::super::context::package_root(),
            &rule,
        );
        face.values.insert("registry_rule_path".to_owned(), package);
        // The rule's **module chain** is a second fact, and it comes from the same interpreter the
        // read-back uses: the discovered tree (walk root + ledger mappings), not from the parent's
        // module and not from the placement base. An explicit placement mounts at its stem, an
        // inherited child under its parent's chain, and a walk-root child at its directories.
        // 规则还有**模块链**这个事实，它来自读回所用的同一个解释器：发现树（遍历根 + 台账映射），既不
        // 来自父模块、也不来自落点基准。显式落点挂在 stem 上，继承子面挂在父链下，遍历根下的子面挂在
        // 它的目录链上。
        let package_root = super::super::context::package_root();
        let layout = super::super::context::layout()?;
        let package_source = xirang_multilang::source_ref::package_source(&package_root, &source);
        let chain = super::parents::module_chain_of(&layout, &package_source)?
            .unwrap_or_else(|| name.to_owned());
        face.values.insert(
            "registry_rule_module".to_owned(),
            format!("{chain}::registry_rule"),
        );
    }
    // The default author interface is `Soil` + `#[soil]` (maintainer ruling 2026-10-09, restated
    // 2026-10-16): the attribute preserves the author's declaration and derives identity, contract checks
    // and `REGISTRATION`, and it emits the module-level `NODE_ID` itself — so this path bakes no identity and
    // writes no `Registration` literal. The literal spelling stays readable and writable **for
    // compatibility** (`is_struct_literal` keeps a file's own spelling on edit); it is not what this
    // generator emits.
    // 默认作者接口是 `Soil` ＋ `#[soil]`（维护者 2026-10-09 裁定、2026-10-16 重申）：属性保留作者声明并派生
    // 身份、合同检查与 `REGISTRATION`，模块级 `NODE_ID` 由它自己发射——因此这条路径不烘焙身份、也不写
    // `Registration` 字面量。字面量拼法保留**可读可写**（兼容：编辑时按文件自己的拼法写回），但它不是本
    // 生成器的输出。
    let info = face.to_snapshot()?;

    // Validate against a staged registry before touching the source tree. This
    // keeps an invalid add from leaving half-written files behind.
    // 写入源码树前先在暂存注册机中校验，避免失败的 add 留下半成品文件。
    registry
        .clone()
        .register_snapshot_batch([info.clone()])
        .map_err(|error| format!("registration rejected:\n{error}"))?;

    let rule = face.render_rule_source()?.map(|text| {
        (
            face.rule_source_path()
                .expect("a rendered rule source has a path"),
            text,
        )
    });
    let rendered = face.render_source()?;
    let source_text = if rendered.lines().any(|line| line == GENERATED_MARKER) {
        rendered
    } else {
        format!("{GENERATED_MARKER}\n{rendered}")
    };
    let _ = scan_root;
    Ok(NewModulePlan {
        source,
        source_relative,
        source_text,
        rule,
        info,
        parent,
    })
}

/// Plan a create at one concrete directory from the public create request.
/// 从公开的新建请求在具体目录里计划一次新建。
#[allow(clippy::too_many_arguments)]
pub(crate) fn plan_new_face_from_spec(
    registry: &Registry,
    name: &str,
    parent: NodeId,
    parent_source: &str,
    parent_kind: &str,
    parent_reference: Option<&str>,
    parent_module: Option<&str>,
    relative_directory: PathBuf,
    source_base: xirang_multilang::source_ref::SourceBase,
    spec: &NewModuleFace<'_>,
) -> Result<NewModulePlan, String> {
    let values = ModuleFaceValues::from_new(spec);
    plan_new_face(
        registry,
        name,
        parent,
        parent_source,
        parent_kind,
        parent_reference,
        parent_module,
        relative_directory,
        source_base,
        Some(&values),
    )
}

/// Write a planned create through the shared transaction, taking back everything on failure.
/// 经共享事务落盘一次已计划的新建；失败时把本次调用创建的一切收回。
///
/// There is one rollback implementation in this crate (`authoring::write_transaction`), and this used to be
/// a second one: a list appended to *after* a `create_new` succeeded can never remove the file that
/// failed half-way through, which is exactly the defect the F2 probe reproduced. `create_new` now
/// captures the path before it creates it, so a partial write is removable, and a target that already
/// exists is left to its owner.
/// 本 crate 只有一份回滚实现（`authoring::write_transaction`），而这里过去是第二份：在 `create_new` **成功之后**
/// 才追加的表永远删不掉"写了一半失败"的那个文件，而这正是 F2 探针复现的缺陷。现在 `create_new` 在创建
/// 之前就保存路径，因此部分写入可被删除，而已经存在的目标留给它的主人。
pub(crate) fn apply_create_module(
    plan: &NewModulePlan,
) -> Result<(AuthoringChange, RegistrationSnapshot), String> {
    let mut transaction = super::super::write_transaction::WriteTransaction::new();
    let outcome = (|| -> Result<(), String> {
        if let Some((rule, rule_source)) = &plan.rule {
            transaction.create_new(rule, rule_source.as_bytes())?;
        }
        // The default author interface (`Soil` + `#[soil]`) emits its own `NODE_ID`: the attribute
        // preserves the declaration and derives identity, so this path neither bakes an identity nor
        // writes the literal form.
        // 默认作者接口（`Soil` ＋ `#[soil]`）自己发射 `NODE_ID`：属性保留声明并派生身份，因此这条路径既不
        // 烘焙身份、也不写字面量形。
        // The generated marker is still written for a face this tool creates: the ownership rule
        // (`is_xirang_owned_source`) reads either the marker or a macro call, and keeping it makes
        // `edit` / `rename` / `delete` independent of which spelling the file ended up using.
        // 本工具新建的面仍然写生成标记：归属规则读标记或宏调用任一，保留它可以让"编辑/改名/删除"不依赖
        // 文件最终用的是哪一种拼法。
        transaction.create_new(&plan.source, plan.source_text.as_bytes())?;
        Ok(())
    })();
    if let Err(error) = outcome {
        let recovery = transaction.rollback();
        return Err(format!("{error}; {}", recovery.describe()));
    }
    Ok((
        AuthoringChange {
            message: format!(
                "created `{}` under parent {}",
                plan.source_relative.display(),
                plan.parent
            ),
            source: plan.source.clone(),
        },
        plan.info.clone(),
    ))
}

#[cfg(test)]
#[path = "create_tests.rs"]
mod create_tests;
