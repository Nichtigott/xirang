//! App construction, reload, and registry lifecycle.
//! App 构造、重载与注册表生命周期。

use super::cargo_probe::cargo_rustc_mir;
use super::project_context::package_root;
use super::*;

impl App {
    /// Switch between the four Studio workspaces and initialize their state.
    /// 切换四个 Studio 工作区并初始化对应状态。
    pub fn open_page(&mut self, page: StudioPage) {
        self.page = page;
        match page {
            StudioPage::Inspect => self.overlay = None,
            StudioPage::Multilang => {
                self.overlay = None;
                self.multilang = crate::multilang::inventory(&package_root())
                    .unwrap_or_else(|error| format!("No `xirang.toml` here:\n{error}"));
            }
            StudioPage::Search => {
                self.overlay = Some(Overlay::Search(SearchState::default()));
            }
            StudioPage::Data => {
                let (center, center_function) = self
                    .selected_info()
                    .map(|info| (Some(info.id), Some(info.source.function.to_owned())))
                    .unwrap_or((None, None));
                self.overlay = Some(Overlay::Search(SearchState {
                    graph_mode: true,
                    center,
                    center_function,
                    ..SearchState::default()
                }));
            }
        }
    }

    /// Put an ordinary status line in the event log.
    /// 往事件日志写入一条普通状态行。
    pub(super) fn note(&mut self, message: impl Into<String>) {
        self.event = message.into();
        self.event_is_alert = false;
    }

    /// Put an alert in the event log: a failure *or* a warning, drawn in red. The
    /// severity is decided here, by the caller's choice of method, instead of being
    /// guessed from the wording (audit `STU-S-18`).
    /// 往事件日志写入一条 alert：失败**或**警告，以红色绘制。严重程度在这里由调用方选择的方法
    /// 决定，而不是从措辞里猜（审计 `STU-S-18`）。
    pub(super) fn alert(&mut self, message: impl Into<String>) {
        self.event = message.into();
        self.event_is_alert = true;
    }

    fn new(registry: Registry, event: String) -> Self {
        let selected = registry
            .depth_first()
            .first()
            .map_or(registry.id(), |info| info.id);
        Self {
            registry,
            // A session starts with no evidence. `load` looks for this project's
            // trace artifact and replaces this with the rebuilt `CallTrace` only
            // when the artifact passes every identity check; otherwise the trace
            // stays disabled and the legend says so instead of claiming a
            // recording that was never loaded.
            // 会话以“没有证据”开始。`load` 查找本项目的 trace artifact，只有在它通过全部身份
            // 检查时才把这里替换为重建出的 `CallTrace`；否则追踪保持关闭，图例如实说明，而不是
            // 宣称一份从未装入的记录。
            runtime_trace: CallTrace::disabled(),
            trace_status: TraceStatus::Absent,
            mir_graph: None,
            selected,
            page: StudioPage::Inspect,
            multilang: String::new(),
            details_selected: 0,
            collapsed: BTreeSet::new(),
            focus: Focus::Tree,
            overlay: None,
            event,
            // A session's first line is ordinary; `note`/`alert` move it from here.
            // 会话的第一行是普通的；`note`/`alert` 从这里改变它。
            event_is_alert: false,
            reload_error: None,
            should_quit: false,
            editor_request: None,
            hot: HotZones::default(),
            #[cfg(feature = "node-graph")]
            graph_flow: None,
            tree_offset: 0,
            split_percent: 45,
            graph_split_percent: 60,
            graph_dragging_divider: false,
            dragging_divider: false,
            last_source_stamp: source_stamp(),
            tree_cache: RefCell::new(Vec::new()),
            search_memo: RefCell::new(None),
            last_source_check: Instant::now(),
        }
    }

    /// Build an app from the on-disk registration snapshot.
    /// 从磁盘上的注册快照构建 App。
    ///
    /// A failed load keeps an empty root registry and reports the error in `event`.
    /// 加载失败时保留一个空根注册表，并把错误写入 `event`。
    pub fn load_app() -> Self {
        let mut app = match load_registry() {
            Ok(registry) => Self::new(
                registry,
                "Ready. Press / to search or a to add a registration face.".to_owned(),
            ),
            Err(error) => Self::new(
                // One load, one answer: retrying here repeated the whole I/O, and a
                // retry that happened to succeed would hand a **loaded** tree to a
                // session whose event line says the load failed (audit `STU-S-16`).
                // The failed load left an empty root; that is what this keeps.
                // 一次加载、一个答案：在这里重试会重复整趟 I/O，而恰好成功的重试会把一棵**加载
                // 成功**的树交给一条写着加载失败的事件行（审计 `STU-S-16`）。失败的那次留下的是
                // 空根，这里保留的正是它。
                Registry::root(),
                format!("Registration startup failed:\n{error}"),
            ),
        };
        app.install_trace();
        if let Ok(query) = std::env::var("XIRANG_INITIAL_QUERY") {
            app.overlay = Some(Overlay::Search(SearchState {
                query,
                ..SearchState::default()
            }));
        }
        app
    }

    /// Refresh a file-backed registration snapshot after an external save.
    /// 外部编辑器保存后刷新文件型注册快照。
    ///
    /// This checks a compact mtime/size stamp at most twice per second. It
    /// does not rebuild Rust code or touch the live trace, so idle Studio stays
    /// cheap and a failed edit keeps the last valid snapshot visible.
    /// 每秒最多检查两次紧凑的修改时间/大小戳；不会重编译 Rust，也不会清空
    /// 实时追踪。编辑失败时继续显示上一份有效快照。
    pub fn poll_hot_reload(&mut self) {
        if self.last_source_check.elapsed() < Duration::from_millis(500) {
            return;
        }
        self.last_source_check = Instant::now();
        let stamp = source_stamp();
        if stamp == self.last_source_stamp {
            return;
        }
        self.last_source_stamp = stamp;
        let before = self.selected;
        match load_registry() {
            Ok(registry) => {
                self.registry = registry;
                self.reload_error = None;
                if before != self.registry.id() && self.registry.find_registry(before).is_some() {
                    self.selected = before;
                } else if self.registry.find_registry(self.selected).is_none() {
                    self.selected = self.registry.id();
                }
                self.details_selected = self
                    .details_selected
                    .min(self.detail_field_count().saturating_sub(1));
                // The snapshot just changed, so the evidence is judged again against
                // it: a trace that was valid for the previous tree may not be, and a
                // MIR snapshot built from that tree now describes faces this session
                // no longer has (audit `STU-S-03`).
                // 快照刚变，因此证据要针对它重新裁决：对上一棵树有效的追踪未必仍有效，而按那棵树
                // 构建的 MIR 快照现在讲的是本会话已不再有的注册面（审计 `STU-S-03`）。
                // The reload succeeded; the trace it invalidated is the alert. A refused
                // trace means this session shows no evidence, which the reader has to
                // see (audit `STU-S-18`).
                // 重载成功了；它作废掉的那份追踪才是 alert。追踪被拒意味着本会话没有证据可显示，
                // 读者必须看见（审计 `STU-S-18`）。
                match self.rejudge_evidence() {
                    Some(reason) => self.alert(format!(
                        "Hot reload: registration snapshot refreshed. Trace artifact refused: {reason}"
                    )),
                    None => self.note("Hot reload: registration snapshot refreshed.".to_owned()),
                };
                self.refresh_open_graft();
            }
            Err(error) => {
                self.reload_error = Some(ReloadError {
                    phase: "hot_reload",
                    message: error.clone(),
                });
                self.alert("Hot reload failed; keeping the last healthy snapshot.".to_owned());
            }
        }
    }

    /// Registration snapshot of the currently selected node, if it still exists.
    /// 当前选中节点的注册快照（若仍存在）。
    pub fn selected_info(&self) -> Option<&RegistrationSnapshot> {
        self.registry.find_registry(self.selected)
    }

    /// The inspector's rows for the current selection, as `(name, value)` pairs.
    /// 当前选择下检视器的行，形如 `(名字, 取值)` 对。
    ///
    /// The renderer and the key handling read this one list, so the number of rows a
    /// reader sees and the number `↑`/`↓` may walk cannot drift apart; they used to
    /// be two truths — 14 here and 12 in the renderer — and the 13th/14th presses were
    /// silent no-ops on a screen with no such row (audit `STU-S-02`).
    /// 渲染器与按键处理读的是同一份清单，因此读者看到的行数与 `↑`/`↓` 能走的行数不会分叉；
    /// 它们过去是两个真值——这里 14、渲染器 12——第 13/14 次按键在一块没有那种行的界面上是静默
    /// 空操作（审计 `STU-S-02`）。
    pub(crate) fn detail_rows(&self) -> Vec<(&'static str, String)> {
        if let Some(info) = self.selected_info() {
            vec![
                ("name", info.registry_name.to_owned()),
                ("kind", info.kind.to_owned()),
                ("node", info.id.to_string()),
                (
                    "path",
                    self.registry
                        .path_for(info.id)
                        .unwrap_or_else(|| "<unknown>".to_owned()),
                ),
                ("parent", info.parent.to_string()),
                (
                    "preset / parts",
                    format!("{} / {}", info.preset, info.parts),
                ),
                // `params` and `handle` are `kind` by rule (both macros expand them
                // from `stringify!($kind)`), so only the interfaces they must carry
                // are worth a row of their own.
                // `params` 与 `handle` 按规则就是 `kind`（两个宏都用 `stringify!($kind)`
                // 展开它们），因此只有它们必须携带的接口值得单独占一行。
                ("handle interfaces", info.handle_traits.join(", ")),
                ("parts interfaces", info.part_traits.join(", ")),
                ("declared", info.source.describe()),
                (
                    "registration rule",
                    format!(
                        "{} ({})",
                        super::registration_rule_text(&info.registry_rule),
                        info.registry_rule_path
                    ),
                ),
                (
                    "dependency admission",
                    super::admission_text(&info.admission),
                ),
                ("exports", info.exports.join(", ")),
            ]
        } else {
            vec![
                ("name", "root".to_owned()),
                ("node", self.registry.id().to_string()),
            ]
        }
    }

    /// Number of rows shown by the main face inspector.
    /// 主注册面检视器显示的字段行数。
    ///
    /// It is the length of [`App::detail_rows`] and nothing else, so the count and
    /// the drawing are the same list (audit `STU-S-02`).
    /// 它就是 [`App::detail_rows`] 的长度、别无其他，因此行数与绘制用的是同一份清单（审计
    /// `STU-S-02`）。
    pub fn detail_field_count(&self) -> usize {
        self.detail_rows().len()
    }

    /// The source stamp this session last checked, for caches that must not outlive
    /// the tree they were built from (audit `STU-S-10`).
    /// 本会话最后一次检查的源码戳，供“不得活过其构建来源那棵树”的缓存使用（审计 `STU-S-10`）。
    // Consumed by the node-graph renderer and by the frame-invariance test; a build without
    // either does not compile it at all, rather than compiling a method nothing can call.
    // 由 node-graph 渲染器与"一帧不改变会话"的测试消费；两者都不在的构建根本不编译它，而不是编译一个
    // 没人能调用的方法。
    #[cfg(any(feature = "node-graph", test))]
    pub(crate) fn source_stamp(&self) -> u128 {
        self.last_source_stamp
    }

    /// Reload the registration snapshot from disk.
    /// 从磁盘重新加载注册快照。
    pub(super) fn reload(&mut self) {
        match load_registry() {
            Ok(registry) => {
                self.registry = registry;
                self.reload_error = None;
                if self.selected != self.registry.id()
                    && self.registry.find_registry(self.selected).is_none()
                {
                    self.selected = self.registry.id();
                }
                // Same reason as `poll_hot_reload`: replacing the snapshot invalidates
                // the evidence that was judged against the old one (audit `STU-S-03`).
                // 与 `poll_hot_reload` 同理：替换快照会让按旧快照裁决过的证据失效（审计
                // `STU-S-03`）。
                // Same split as `poll_hot_reload`: the refresh is ordinary, a refused
                // trace is the alert (audit `STU-S-18`).
                // 与 `poll_hot_reload` 同一分法：刷新是普通的，被拒的追踪才是 alert（审计
                // `STU-S-18`）。
                match self.rejudge_evidence() {
                    Some(reason) => self.alert(format!(
                        "Registration snapshot reloaded from disk. Trace artifact refused: {reason}"
                    )),
                    None => self.note("Registration snapshot reloaded from disk.".to_owned()),
                };
                self.refresh_open_graft();
            }
            Err(error) => {
                self.reload_error = Some(ReloadError {
                    phase: "reload",
                    message: error.clone(),
                });
                self.alert("Reload failed; keeping the last healthy snapshot.".to_owned());
            }
        }
    }

    /// Everything a project write needs once its file is on disk, in one place.
    /// 一次项目写入落盘之后需要的一切，集中在一处。
    ///
    /// The two write paths used to diverge: the add path registered the batch it was handed
    /// back and stopped there, while the edit path reloaded and re-selected. Both now run
    /// this — the snapshot (that batch, or a reload when the write only reports a message),
    /// the evidence re-judged against the new snapshot, a fresh source stamp so the
    /// hot-reload path does not immediately read our own write as a change, and no graph
    /// left describing the tree that just changed (audit `STU-S-27`, `STU-S-10`,
    /// `STU-S-03`).
    /// 两条写入路径过去各走各的：新增路径注册交回的那一格就停在那里，编辑路径重载并重选。现在两者
    /// 都走这里——快照（那一格，或写入只报消息时的一次重载）、按新快照重判的证据、新的源码戳（使
    /// 热重载路径不会立刻把我们自己的写入读成一次变更），以及不残留描述刚变那棵树的调用图
    /// （审计 `STU-S-27`、`STU-S-10`、`STU-S-03`）。
    /// The `batch` says what the write did to the snapshot: `Some(snapshots)` is the batch
    /// a write handed back — empty when it changed no registration input, as a plugin lock
    /// does — and `None` asks for a reload, because the write changed the files the snapshot
    /// is built from.
    /// `batch` 说明写入对快照做了什么：`Some(snapshots)` 是写入交回的那一批——写入没有改变任何
    /// 注册输入时为空，例如插件锁——而 `None` 要求重载，因为写入改了快照赖以构建的文件。
    pub(super) fn after_project_write(
        &mut self,
        batch: Option<Vec<RegistrationSnapshot>>,
    ) -> Result<(), String> {
        // The stamp comes first: whatever the snapshot decides, this session has already
        // seen the file this write produced, so the watcher must not report it back.
        // 先记戳：无论快照如何决定，本会话已经看到了这次写入产生的文件，因此监听器不得把它报回来。
        self.last_source_stamp = source_stamp();
        // A cached graph describes the tree from before the write.
        // 缓存的调用图描述的是写入之前的那棵树。
        #[cfg(feature = "node-graph")]
        {
            self.graph_flow = None;
        }
        match batch {
            Some(snapshots) => {
                self.registry
                    .register_snapshot_batch(snapshots)
                    .map_err(|error| error.to_string())?;
                // The batch swapped the snapshot just as a reload does, so the evidence has
                // to be re-judged here too (audit `STU-S-03`). The reload path below does
                // that for itself.
                // 这一格与重载一样替换了快照，因此证据在这里也要重判（审计 `STU-S-03`）。下面的
                // 重载路径自己会做这件事。
                self.rejudge_evidence();
                Ok(())
            }
            None => {
                self.reload();
                match &self.reload_error {
                    Some(error) => Err(format!("[phase={}] {}", error.phase, error.message)),
                    None => Ok(()),
                }
            }
        }
    }

    /// Re-ask for a MIR snapshot and say what came of it, in one place so the key
    /// binding stays one line.
    /// 重新请求 MIR 快照并说明结果，集中在一处，使按键绑定保持一行。
    pub(super) fn load_mir_snapshot_report(&mut self) {
        // A snapshot is ordinary; no snapshot is the alert. `MIR unavailable` never said
        // "fail", so it used to be drawn green — this is the case that made the old rule
        // worth removing (audit `STU-S-18`).
        // 快照是普通的；没有快照才是 alert。`MIR unavailable` 从不含 “fail”，因此过去被画成绿
        // 色——正是这个例子让旧规则值得被删掉（审计 `STU-S-18`）。
        match self.load_mir_snapshot() {
            Ok(calls) => self.note(format!(
                "MIR snapshot loaded in memory: {calls} candidate calls."
            )),
            Err(error) => self.alert(format!("MIR unavailable: {error}")),
        }
    }

    /// Ask rustc for a one-shot in-memory MIR snapshot.
    /// 直接请求 rustc 一次，将 MIR 快照留在内存中。
    ///
    /// Failures come back as the compiler text to show —
    /// because the event line belongs to the caller (audit `STU-C-05`).
    /// 失败会作为要展示的编译器文本返回，因为事件行归调用方所有
    /// （审计 `STU-C-05`）。
    pub fn load_mir_snapshot(&mut self) -> Result<usize, String> {
        let manifest = host_manifest();
        let output = cargo_rustc_mir(&manifest, &["--emit=mir"])?;
        if !output.status.success() {
            let detail = String::from_utf8_lossy(&output.stderr);
            return Err(if detail.trim().is_empty() {
                format!("cargo rustc exited with {}", output.status)
            } else {
                detail.trim().to_owned()
            });
        }
        let graph = MirGraph::from_mir_text(&String::from_utf8_lossy(&output.stdout));
        let calls = graph.calls.len();
        self.mir_graph = Some(graph);
        Ok(calls)
    }

    pub(super) fn build_all(&mut self) {
        let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
        let project_root = package_root();
        // Per arm: a passing build is ordinary, a failed build and a Cargo that cannot
        // even be invoked are both alerts — the second never said "fail" and used to be
        // green (audit `STU-S-18`).
        // 逐臂：构建通过是普通的，构建失败与“连 Cargo 都调不起来”都是 alert——后者从不含
        // “fail”，过去是绿的（审计 `STU-S-18`）。
        match Command::new(cargo)
            .args(["build", "--quiet", "--release", "--all-targets"])
            .current_dir(project_root)
            .output()
        {
            Ok(output) if output.status.success() => self.note("Final build passed.".to_owned()),
            Ok(output) => self.alert(format!(
                "Final build failed:\n{}",
                String::from_utf8_lossy(&output.stderr)
            )),
            Err(error) => self.alert(format!("Cannot invoke Cargo: {error}")),
        }
    }
}
