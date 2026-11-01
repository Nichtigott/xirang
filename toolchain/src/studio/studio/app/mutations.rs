//! App mutations: the write paths, and what each of them does to the session.
//! App 文件变更：各条写入路径，以及每条对会话做了什么。

use super::project_context::{package_namespace, package_root, select_project};
use super::write_guard::{selected_package_root, with_selected_project};
use super::*;
use crate::run_method::pascal_case;

impl App {
    pub(super) fn submit_new_project(&mut self, project: &NewProjectState) {
        let directory = project.values[new_project_field::DIRECTORY].trim();
        let package = project.values[new_project_field::PACKAGE].trim();
        let kind = project.values[new_project_field::KIND].trim();
        if directory.is_empty() || package.is_empty() {
            self.alert("New project failed: directory and package are required".to_owned());
            return;
        }
        if !matches!(kind, "binary" | "library") {
            self.alert("New project failed: kind must be binary or library".to_owned());
            return;
        }
        if !package
            .chars()
            .all(|ch| ch == '_' || ch == '-' || ch.is_ascii_alphanumeric())
        {
            self.event =
                "New project failed: package must use letters, digits, '_' or '-'".to_owned();
            return;
        }
        // A relative directory is the reader's, not the process's: the wizard is
        // open over a project, so `my-app` means a child of the project the reader
        // is looking at rather than of whatever directory Studio happened to be
        // started from.
        // 相对目录属于读者而不是进程：向导开在一个项目之上，因此 `my-app` 指的是读者正在看的
        // 项目的兄弟目录，而不是 Studio 恰好在其中启动的那个目录的子目录。
        let root = std::path::PathBuf::from(directory);
        let root = if root.is_absolute() {
            root
        } else {
            package_root().join(root)
        };
        let source = crate::build_method::scaffold::detected_source(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")),
            &std::env::current_exe().unwrap_or_default(),
        );
        let kind = if kind == "library" {
            crate::build_method::scaffold::ProjectKind::Library
        } else {
            crate::build_method::scaffold::ProjectKind::Binary
        };
        if let Err(error) =
            crate::build_method::scaffold::create_project(&root, package, kind, &source)
        {
            self.alert(format!("New project failed: {error}"));
            return;
        }
        // Keep the new project visible immediately. The next reload reads its
        // folder-backed faces; no manual environment setup or restart needed.
        // 立即切换到新项目；下一次 reload 会读取它的文件注册面，无需手动设置环境变量。
        select_project(root.clone(), root.join("Cargo.toml"), package);
        // The scaffold wrote a whole tree: the same entry point as every other write
        // applies it and records that this session already saw it (audit `STU-S-27`).
        // 脚手架写了一整棵树：与其余写入同一个入口来应用它，并记下本会话已经看到过它
        // （审计 `STU-S-27`）。
        let _ = self.after_project_write(None);
        self.note(format!(
            "Created {kind} project at {}; Studio switched to it",
            root.display()
        ));
        self.overlay = None;
    }

    /// Take the pending editor handoff, leaving none queued behind it.
    /// 取走待处理的编辑器交接请求，取走后不再有排队项。
    ///
    /// Returns the file and the 1-based line to open; `None` when nothing is pending.
    /// 返回要打开的文件与从 1 开始的行号；没有待处理请求时为 `None`。
    pub(super) fn submit_add(&mut self, add: &AddState) {
        let parent = match self.resolve_parent(&add.values[face_field::PARENT]) {
            Ok(parent) => parent,
            Err(error) => {
                self.alert(format!("Add failed: {error}"));
                return;
            }
        };
        let kind_fallback = pascal_case(&add.values[face_field::MODULE]);
        let kind = if add.values[face_field::KIND].trim().is_empty() {
            kind_fallback.as_str()
        } else {
            add.values[face_field::KIND].trim()
        };
        if add.values[face_field::MODULE].is_empty() {
            self.alert("Add failed: module name is required".to_owned());
            return;
        }
        let needs_registry = match add.values[face_field::NEEDS_REGISTRY].parse::<bool>() {
            Ok(value) => value,
            Err(_) => {
                self.alert("Add failed: needs registry must be true or false".to_owned());
                return;
            }
        };
        // **Every** language goes through the shared authoring service, Rust included: the Studio
        // form used to call the older executor directly for Rust, which is one of the entrance forks
        // this round closes. The form only offers the rows a language carries, so the request names
        // exactly those, and the service decides where the file goes.
        // **每一种**语言都走共享创作服务，包括 Rust：Studio 表单过去对 Rust 直接调用更老的执行器，而那是本轮
        // 收掉的入口分叉之一。表单只提供某语言携带的行，因此请求只点名那些字段，落点由服务决定。
        self.submit_add_in_language(add, parent, kind, needs_registry);
    }

    pub(super) fn submit_edit(&mut self, id: NodeId, edit: &AddState) {
        let needs_registry = match edit.values[face_field::NEEDS_REGISTRY].parse::<bool>() {
            Ok(value) => value,
            Err(_) => {
                self.alert("Edit failed: needs registry must be true or false".to_owned());
                return;
            }
        };
        let patch = crate::run_method::ModuleFacePatch {
            module: &edit.values[face_field::MODULE],
            kind: &edit.values[face_field::KIND],
            preset: &edit.values[face_field::PRESET],
            parts: &edit.values[face_field::PARTS],
            name_zh: &edit.values[face_field::NAME_ZH],
            name_en: &edit.values[face_field::NAME_EN],
            summary_zh: &edit.values[face_field::SUMMARY_ZH],
            summary_en: &edit.values[face_field::SUMMARY_EN],
            exports: &edit.values[face_field::EXPORTS],
            stable_name: &edit.values[face_field::STABLE_NAME],
            needs_registry,
            getting_from_other_registry: &edit.values[face_field::GETTING_FROM_OTHER_REGISTRY],
            registration_rule: &edit.values[face_field::REGISTRY_RULE],
            admission: &edit.values[face_field::ADMISSION],
            handle_traits: &edit.values[face_field::HANDLE_TRAITS],
            handle_contracts: &edit.values[face_field::HANDLE_CONTRACTS],
            part_traits: &edit.values[face_field::PART_TRAITS],
            part_contracts: &edit.values[face_field::PART_CONTRACTS],
            requires: &edit.values[face_field::REQUIRES],
            provides: &edit.values[face_field::PROVIDES],
            runtime_checks: &edit.values[face_field::RUNTIME_CHECKS],
            flow: &edit.values[face_field::FLOW],
            flow_provider: &edit.values[face_field::FLOW_PROVIDER],
        };
        match with_selected_project(|| {
            crate::run_method::edit_module_face(&self.registry, id, &patch)
        }) {
            Ok(change) => {
                let message = change.message;
                let changed_source = change.source;
                // Keep the visible inspector in sync with the file we just committed: the
                // reload this runs is deliberately after the atomic write, so a failed
                // parse keeps the previous healthy state. Same entry point as the add
                // path (audit `STU-S-27`).
                // 让可见的检视器与我们刚提交的文件同步：它跑的重载刻意在原子写入之后，因此解析失败
                // 会保留上一份健康状态。与新增路径同一个入口（审计 `STU-S-27`）。
                let healthy = self.after_project_write(None).is_ok();
                if healthy {
                    if let Some(info) = self
                        .registry
                        .depth_first()
                        .into_iter()
                        .find(|info| source_path_for(&info.source.file) == changed_source)
                    {
                        self.selected = info.id;
                    }
                    self.note(format!("{message}; registration reloaded"));
                }
                self.overlay = None;
            }
            Err(error) => self.alert(format!("Edit failed: {error}")),
        }
    }

    pub(super) fn submit_plugin(&mut self, plugin: &PluginState) {
        let source = plugin.values[plugin_field::SOURCE].trim();
        let framework = plugin.values[plugin_field::FRAMEWORK].trim();
        let package = plugin.values[plugin_field::PACKAGE].trim();
        let version = plugin.values[plugin_field::VERSION].trim();
        let crate_name = plugin.values[plugin_field::CRATE].trim();
        let checksum = plugin.values[plugin_field::CHECKSUM].trim();
        let mode = plugin.values[plugin_field::MODE].trim();
        // The three provenance rows are optional, and they are the whole reason this form and the
        // MCP bridge can write the same lock: empty rows mean the seven-column form, and naming
        // any one of them declares the extension (the other two columns then spell empty). The
        // record is built once here, and the kernel renders the line from it, so neither writer
        // spells a column layout of its own.
        // 三个来源行是可选的，而它们正是这份表单与 MCP 桥能写同一把锁的全部原因：三行都空就是七列
        // 形式，点名其中任意一个就声明了该扩展（另外两列随即拼成空）。记录在这里建一次，锁行由内核
        // 从它渲染，因此两个写入方都不再自己拼一套列布局。
        let signature = plugin.values[plugin_field::SIGNATURE].trim();
        let fingerprint = plugin.values[plugin_field::FINGERPRINT].trim();
        let revocations = plugin.values[plugin_field::REVOCATIONS].trim();
        let Some(source_kind) = PluginSource::parse_plugin_source(source) else {
            self.alert("Plugin failed: source must be official or user".to_owned());
            return;
        };
        let Some(mode_kind) = PluginMode::parse_plugin_mode(mode) else {
            self.alert("Plugin failed: mode must be extension or replacement".to_owned());
            return;
        };
        if [framework, package, version, crate_name, checksum]
            .iter()
            .any(|value| value.is_empty())
        {
            self.event =
                "Plugin failed: framework, package, version, crate and checksum are required"
                    .to_owned();
            return;
        }
        // One of Studio's writers, so it needs the project the reader opened, not
        // one resolved from the environment or the working directory: it creates
        // `.xirang/plugins/` and appends to a lock file in it.
        // Studio 的写入方之一，因此它需要读者打开的那个项目，而不是从环境或工作目录解析出来的
        // 一个：它会创建 `.xirang/plugins/` 并往其中的锁文件里追加。
        let package_root = match selected_package_root() {
            Ok(root) => root,
            Err(error) => {
                self.alert(format!("Plugin failed: {error}"));
                return;
            }
        };
        let plugin_root = package_root.join(".xirang/plugins");
        let lock_name = if source == "official" {
            "official.lock"
        } else {
            "user.lock"
        };
        let lock = plugin_root.join(lock_name);
        let declares_provenance = [signature, fingerprint, revocations]
            .iter()
            .any(|value| !value.is_empty());
        let column =
            |value: &str| -> Option<String> { declares_provenance.then(|| value.to_owned()) };
        let record = PluginRecord {
            source: source_kind,
            framework: framework.to_owned(),
            package: package.to_owned(),
            version: version.to_owned(),
            crate_name: crate_name.to_owned(),
            checksum: checksum.to_owned(),
            mode: mode_kind,
            signature: column(signature),
            public_key_fingerprint: column(fingerprint),
            revocation_list: column(revocations),
        };
        let existing = std::fs::read_to_string(&lock).unwrap_or_default();
        if existing.lines().any(|line| line.trim() == record.line()) {
            self.note("Plugin already selected".to_owned());
            self.overlay = None;
            return;
        }
        let catalog = match PluginCatalog::parse_plugin_catalog(&existing) {
            Ok(catalog) => catalog,
            Err(error) => {
                self.alert(format!("Plugin failed: invalid lock: {error}"));
                return;
            }
        };
        // The runtime's rule, not an identical-record rule: a ten-field official record must be
        // writable when the lock carries the seven-field form, because that is the artifact the
        // runtime accepts. `contains` asked for all ten fields to match and refused this write
        // (audit `PH-7`).
        // 用的是运行期那条规则，而不是"记录完全相同"：当锁里是七字段形式时，一条十字段的官方记录
        // 必须写得进去，因为那正是运行期接受的工件。`contains` 要求十个字段全等，因此拒绝了这次写入
        // （审计 `PH-7`）。
        if source_kind == PluginSource::Official && !catalog.contains_record(&record) {
            self.event =
                "Plugin failed: official package is not present in the trusted lock".to_owned();
            return;
        }
        if !crate_name
            .chars()
            .all(|character| character == '_' || character.is_ascii_alphanumeric())
        {
            self.alert("Plugin failed: crate must be a Rust identifier".to_owned());
            return;
        }
        // The lock is the artifact the host reads, so the parser decides whether
        // this append is legal, and it decides before anything is written: a
        // record the kernel refuses must not land on disk, and the refusal has to
        // reach the reader instead of a success banner. The text that comes back
        // is the text written below, so what was validated is what is stored.
        // 锁是宿主读取的工件，因此这次追加是否合法由解析器决定，而且是在写任何东西之前决定：
        // 内核拒绝的记录不得落盘，拒绝必须到达读者而不是被成功横幅盖掉。交回的文本就是下面写下
        // 的文本，因此被校验的就是被存下的。
        let line = format!("{}\n", record.line());
        let candidate = match PluginCatalog::with_record(&existing, &line) {
            Ok(text) => text,
            Err(error) => {
                self.event =
                    format!("Plugin failed: {lock_name} would not parse with this record: {error}");
                return;
            }
        };
        let entry_name = if source == "official" {
            "official.rs"
        } else {
            "user.rs"
        };
        let entry = plugin_root.join(entry_name);
        let anchor = format!("\n#[allow(unused_imports)]\nuse {crate_name} as _;\n");
        let entry_existed = entry.is_file();
        let entry_text = std::fs::read_to_string(&entry).unwrap_or_default();
        // The gate asks whether the *entry line* is there, not whether its text
        // appears anywhere in the file: a commented-out or quoted copy is not an
        // import, and reading it as one skipped the append and then reported
        // "Plugin selected" over an entry that never imported the crate — a write
        // that succeeded while the semantic effect it exists for did not happen
        // (audit `STU-S-08`).
        // 这道闸门问的是**入口那一行**在不在，而不是那串文本是否出现在文件的任何地方：被注释掉
        // 或被引号包住的副本不是一次导入；把它读成导入会跳过追加，然后在一条从未导入该 crate 的
        // 入口上报 "Plugin selected"——一次成功了、而它存在的语义目的却没有发生的写入（审计
        // `STU-S-08`）。
        let entry_line = format!("use {crate_name} as _;");
        let imported = entry_text.lines().any(|line| line.trim() == entry_line);
        if !imported && let Err(error) = append_line(&entry, &anchor) {
            self.alert(format!(
                "Plugin failed: cannot update {entry_name}: {error}"
            ));
            return;
        }
        // Two files carry one decision, and the lock is the one the runtime reads.
        // A half-written pair would leave the entry importing a crate the lock
        // does not record, so the entry file goes back to its previous bytes
        // before the error is reported.
        // 一个决定由两个文件承载，而锁是运行期读取的那一个。写了一半会让入口导入一个锁里没有
        // 记录的 crate，因此在报告错误之前把入口文件恢复成先前的字节。
        if let Err(error) = std::fs::write(&lock, candidate.as_bytes()) {
            let restored = if entry_existed {
                std::fs::write(&entry, &entry_text)
            } else {
                std::fs::remove_file(&entry)
            };
            let note = if restored.is_ok() {
                "the entry file was restored"
            } else {
                "the entry file could not be restored"
            };
            self.alert(format!(
                "Plugin failed: cannot update {lock_name}: {error}; {note}"
            ));
            return;
        }
        // A plugin lock changes what the host admits, but not this snapshot: an empty batch
        // says so, while the write still counts as seen so the watcher cannot report our own
        // lock back as someone else's change (audit `STU-S-27`).
        // 插件锁改变的是宿主准入什么，而不是这份快照：空的批量说明了这一点，而这次写入仍被记为
        // 已见过，因此监听器不能把我们自己的锁报成别人的变更（审计 `STU-S-27`）。
        let _ = self.after_project_write(Some(Vec::new()));
        self.note(format!("Plugin selected: {package} ({source}, {mode})"));
        self.overlay = None;
    }

    /// Add one face in a non-Rust language through the shared authoring service.
    /// 经共享创作服务用一种非 Rust 语言新增注册面。
    fn submit_add_in_language(
        &mut self,
        add: &AddState,
        parent: NodeId,
        kind: &str,
        needs_registry: bool,
    ) {
        use xirang_kernel::authoring::{AuthoringOperation, FieldUpdate, RegistrationField};
        let Some(language) = xirang_kernel::interchange::LanguageId::parse_language(&add.language)
        else {
            self.alert(format!(
                "Add failed: `{}` is not a language id",
                add.language
            ));
            return;
        };
        let mut request = crate::authoring_service::AuthoringRequest::create();
        request.language = Some(language);
        request.module = Some(add.values[face_field::MODULE].clone());
        request.parent = Some(parent);
        request.operation = AuthoringOperation::Create;
        // Every row the form carries goes into the request, for every language. The service hands
        // the named ones to the adapter and leaves the rest at the generator's defaults, so a row
        // this list forgets would silently stop being written — which is why it is the whole table
        // and not "the ones a foreign language happens to use".
        // 表单携带的每一行都进请求，对所有语言都是。服务把点名的行交给适配器、其余留给生成器默认值，因此这张表
        // 漏掉一行，那一行就会静默地不再被写下——这正是它是整张表、而不是"外语恰好用到的那几行"的原因。
        let name_zh = if add.values[face_field::NAME_ZH].trim().is_empty() {
            kind
        } else {
            add.values[face_field::NAME_ZH].as_str()
        };
        let name_en = if add.values[face_field::NAME_EN].trim().is_empty() {
            kind
        } else {
            add.values[face_field::NAME_EN].as_str()
        };
        // The rows the form **carries for this language**, and only those: a foreign adapter's
        // capability table says which fields it can hold, the form prints the rest muted, and sending
        // a row the language does not carry would either be refused by the plan or silently bake a
        // value the author never typed. `Kind` and `NeedsRegistry` are always sent — every language
        // names them — and a blank row means "nothing named", which is the generator's default.
        // **这门语言携带的**表单行，且只有那些：外语适配器的能力表说明它能持有哪些字段，表单把其余画成暗色，
        // 而发送一门语言不携带的行，要么被计划拒绝、要么静默烘进作者从未键入的取值。`Kind` 与 `NeedsRegistry`
        // 总是发送——每种语言都点名它们——而空行意味着"什么都没点名"，也就是生成器的默认值。
        for (row, field, value) in [
            (face_field::KIND, RegistrationField::Kind, kind),
            (
                face_field::PRESET,
                RegistrationField::Preset,
                add.values[face_field::PRESET].as_str(),
            ),
            (
                face_field::PARTS,
                RegistrationField::Parts,
                add.values[face_field::PARTS].as_str(),
            ),
            (face_field::NAME_ZH, RegistrationField::NameZh, name_zh),
            (face_field::NAME_EN, RegistrationField::NameEn, name_en),
            (
                face_field::SUMMARY_ZH,
                RegistrationField::SummaryZh,
                add.values[face_field::SUMMARY_ZH].as_str(),
            ),
            (
                face_field::SUMMARY_EN,
                RegistrationField::SummaryEn,
                add.values[face_field::SUMMARY_EN].as_str(),
            ),
            (
                face_field::EXPORTS,
                RegistrationField::Exports,
                add.values[face_field::EXPORTS].as_str(),
            ),
            (
                face_field::STABLE_NAME,
                RegistrationField::StableName,
                add.values[face_field::STABLE_NAME].as_str(),
            ),
            (
                face_field::GETTING_FROM_OTHER_REGISTRY,
                RegistrationField::GettingFromOtherRegistry,
                add.values[face_field::GETTING_FROM_OTHER_REGISTRY].as_str(),
            ),
            (
                face_field::REGISTRY_RULE,
                RegistrationField::RegistrationRule,
                add.values[face_field::REGISTRY_RULE].as_str(),
            ),
            (
                face_field::ADMISSION,
                RegistrationField::Admission,
                add.values[face_field::ADMISSION].as_str(),
            ),
            (
                face_field::HANDLE_CONTRACTS,
                RegistrationField::HandleContracts,
                add.values[face_field::HANDLE_CONTRACTS].as_str(),
            ),
            (
                face_field::PART_CONTRACTS,
                RegistrationField::PartContracts,
                add.values[face_field::PART_CONTRACTS].as_str(),
            ),
            (
                face_field::REQUIRES,
                RegistrationField::Requires,
                add.values[face_field::REQUIRES].as_str(),
            ),
            (
                face_field::PROVIDES,
                RegistrationField::Provides,
                add.values[face_field::PROVIDES].as_str(),
            ),
            (
                face_field::RUNTIME_CHECKS,
                RegistrationField::RuntimeChecks,
                add.values[face_field::RUNTIME_CHECKS].as_str(),
            ),
            (
                face_field::FLOW,
                RegistrationField::Flow,
                add.values[face_field::FLOW].as_str(),
            ),
            (
                face_field::FLOW_PROVIDER,
                RegistrationField::FlowProvider,
                add.values[face_field::FLOW_PROVIDER].as_str(),
            ),
            (
                face_field::NEEDS_REGISTRY,
                RegistrationField::NeedsRegistry,
                if needs_registry { "true" } else { "false" },
            ),
        ] {
            let always = matches!(
                field,
                RegistrationField::Kind | RegistrationField::NeedsRegistry
            );
            if !always && !add.carried.contains(&row) {
                continue;
            }
            if value.trim().is_empty() && !always {
                continue;
            }
            if let Err(error) = request
                .registration
                .set_text(field, FieldUpdate::Set(value.trim()))
            {
                self.alert(format!("Add failed: {error}"));
                return;
            }
        }
        let service =
            crate::authoring_service::AuthoringService::new(package_root(), package_namespace());
        let plan = match service.plan(&request) {
            Ok(plan) => plan,
            Err(error) => {
                self.alert(format!("Add failed: {error}"));
                return;
            }
        };
        match service.apply(&plan) {
            Ok(applied) => {
                let _ = self.after_project_write(None);
                self.note(format!("{}; press r to reload", applied.message));
                self.overlay = None;
            }
            Err(error) => self.alert(format!("Add failed: {error}")),
        }
    }

    fn resolve_parent(&self, value: &str) -> Result<NodeId, String> {
        let value = value.trim();
        if value.is_empty() || value.eq_ignore_ascii_case("root") {
            return Ok(self.registry.id());
        }
        if let Ok(id) = value.parse::<NodeId>() {
            return self
                .registry
                .registry(id)
                .map(|_| id)
                .ok_or_else(|| format!("`{id}` is not a Registry"));
        }
        let matches = self
            .registry
            .depth_first()
            .into_iter()
            .filter(|info| {
                info.needs_registry
                    && (info.registry_name.eq_ignore_ascii_case(value)
                        || info.kind.eq_ignore_ascii_case(value)
                        || self
                            .registry
                            .path_for(info.id)
                            .is_some_and(|path| path.eq_ignore_ascii_case(value)))
            })
            .collect::<Vec<_>>();
        match matches.as_slice() {
            [info] => Ok(info.id),
            [] => Err(format!("parent `{value}` was not found")),
            _ => Err(format!(
                "parent `{value}` is ambiguous; use its node identity"
            )),
        }
    }
}

/// Append one line to a file, creating it when it is absent.
/// 向文件追加一行；文件不存在时创建它。
fn append_line(path: &std::path::Path, line: &str) -> std::io::Result<()> {
    std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .and_then(|mut file| std::io::Write::write_all(&mut file, line.as_bytes()))
}

// The lock-write regression tests live in their own file under `tests/`, mounted
// as a test-only module: the same shape every sibling there has, which is what
// exempts it from the size ceiling (`conventions/src/size.rs::is_mounted_as_test`).
// 锁写入的回归测试住在 `tests/` 下的独立文件里，以仅测试模块挂载：与那里的每个同级文件同
// 一形态，这正是它免于尺寸上限的原因（`conventions/src/size.rs::is_mounted_as_test`）。
#[cfg(test)]
#[path = "tests/lock_writes.rs"]
mod lock_writes;
