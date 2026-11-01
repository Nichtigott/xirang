//! Durable state for one verification task: its input manifest, phase journal and writer mark.
//! 一次验证任务的持久状态：输入清单、阶段 journal 与写者标记。
//!
//! The ledger says *what* was verified; this module says **against which bytes**, and it
//! survives the process that wrote it. That is what lets a later call name the files that
//! changed since the evidence was produced instead of only reporting that a digest differs
//! (A15/A16), and what lets a killed run leave an interpretable phase record instead of a
//! JSON file that looks finished (A17/A19).
//! 台账说的是**验证了什么**；本模块说的是**对着哪些字节**，而且它比写下它的进程活得更久。正因如此，
//! 之后的调用才能点名"证据产出之后变了哪些文件"，而不只是报一句摘要不同（A15/A16）；也正因如此，
//! 一个被杀死的运行留下的是一条可解释的阶段记录，而不是一个看起来已完成 JSON 文件（A17/A19）。
//!
//! Inputs are captured from the working tree itself — no Git, no index — so a project with no
//! repository, or one whose tree is dirty before the task starts, is described just as exactly.
//! 输入取自工作树本身——不用 Git、不用索引——因此没有仓库的项目、或任务开始前就是脏树的项目，同样能被
//! 精确描述。
//!
//! The journal is append-only and one JSON object per line, because a single short append is
//! the only write this module can make crash-honest without a transaction.
//! journal 是只追加的、每行一个 JSON 对象，因为一行短追加是本模块在没有事务的前提下唯一能做到崩溃诚实的
//! 写入。

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};

/// The bytes one plan's verdict was computed against: relative path → content digest.
/// 一次计划的判定所依据的字节：相对路径 → 内容摘要。
///
/// The digest is the fold the ledger already recorded as `source_configuration`, so a plan
/// written before this module existed still compares equal when nothing changed.
/// 这个摘要就是台账早已记录为 `source_configuration` 的那个折叠，因此在本模块存在之前写下的计划，只要
/// 什么都没变，比较起来仍然相等。
#[derive(Debug)]
pub(super) struct Inputs {
    /// Sorted `(relative path, sha256)` pairs, exactly as the fold consumes them.
    /// 已排序的 `(相对路径, sha256)` 对，与折叠所消费的形状逐字一致。
    rows: Vec<(PathBuf, String)>,
    /// The environment inputs this closure can depend on, as `(key, sha256 of value)`.
    /// 这个闭包可能依赖的环境输入：`(键, 值的 sha256)`。
    ///
    /// A verdict computed under one `RUSTFLAGS`/`XIRANG_*` value must not be reused under another, and a build
    /// script or `cfg` selecting a branch by environment is exactly how a change becomes invisible to a
    /// file-only closure. Only the declared prefixes are captured; the keys are listed in one place so the
    /// contract ("which environment is an input") has a single home.
    /// 在某个 `RUSTFLAGS`/`XIRANG_*` 取值下算出的判定，不得在另一个取值下被复用；而构建脚本或 `cfg` 按环境选
    /// 分支，正是"只绑文件的闭包看不见的变化"。这里只捕获**声明过的前缀**；键列在一处，于是"哪些环境算输入"
    /// 这条约定只有一个家。
    env: Vec<(String, String)>,
    /// Why this closure cannot be bound completely, if it cannot. An incomplete closure must never issue a
    /// reusable credential: unknown inputs are reported, not silently dropped.
    /// 这个闭包若**不能**被完整绑定，原因写在这里。不完整的闭包**绝不**签发可复用凭据：未知输入要被报出来，
    /// 而不是被静默丢掉。
    incomplete: Option<String>,
    /// The **whole** environment this capture saw, folded to one digest — not only the declared prefixes.
    /// 这次捕获看到的**整个**环境折叠成的一个摘要——不只是声明过的那些前缀。
    ///
    /// The prefix list above was wrong the moment anything outside it became an input, and nothing could
    /// tell: a `#[test]` that reads `XR_REVIEW_MODE` is bound by nothing, so a verdict computed under
    /// `good` was reused under `bad` while the same command run directly failed. The measurement is in
    /// `docs/managed-maintenance/review-2026-10-17/freshness-results.json`. The list is a **reading aid
    /// for the manifest**, not the definition of which environment is an input; this field is the
    /// definition, and it is complete because it is exhaustive.
    /// 前缀清单在"清单之外有任何东西成为输入"的那一刻就是错的，而且没有任何东西能说明这一点：一条读取
    /// `XR_REVIEW_MODE` 的 `#[test]` 没被任何东西绑定，于是在 `good` 下算出的判定被在 `bad` 下复用，而同一条命令
    /// 直接执行却失败（实测见 `docs/managed-maintenance/review-2026-10-17/freshness-results.json`）。那份前缀清单
    /// 只是**给人读清单时的便利**，不是"哪些环境算输入"的定义；这个字段才是定义，而它因为是穷举的所以完整。
    whole_environment: String,
}

/// The environment prefixes the **manifest** lists by name. Every variable is bound (see
/// `whole_environment`); this list only decides which ones a reader can see spelled out in the manifest, so
/// adding a name here is a readability change and never an authorization one. It used to be described as
/// the definition of "which environment is an input", and that description was the false green: a test
/// reading a variable outside it was bound by nothing.
/// **清单**按名字列出的那些环境前缀。每个变量都被绑定（见 `whole_environment`）；这份清单只决定读者能在清单里
/// 看到哪几个名字被写出来，因此在这里加一个名字是**可读性**改动，永远不是授权改动。它曾被描述成"哪些环境算输入"
/// 的定义，而那段描述正是那次假绿：读取清单之外某个变量的测试没被任何东西绑定。
const ENV_INPUT_PREFIXES: &[&str] = &["XIRANG_", "CARGO_", "RUST", "RUSTC", "RUSTDOC"];

impl Inputs {
    /// Walk the working tree and record every verification input it holds.
    /// 遍历工作树，记录它持有的每一项验证输入。
    ///
    /// **There is no extension whitelist.** A verdict can depend on a `.txt` fixture, an extensionless
    /// script, an `include!`d or generated input, so a closure that binds only known extensions reports
    /// "inputs unchanged" after the input changed — the false green this path exists to prevent, and the
    /// spec forbids issuing a reusable credential from an extension whitelist
    /// (`managed-maintenance/basic-repair-efficiency.md` §5 BR-A). Every regular file is bound, a symlink is
    /// bound by its target, and an entry this walk cannot bind makes the closure **incomplete** rather than
    /// silently absent. `target`, `.git`, `.codegraph`, `.direnv` and the excluded output directory are never
    /// inputs, and an input resolving outside the root is an error rather than a silently unbound file.
    /// **没有扩展名白名单。** 判定可能依赖 `.txt` 夹具、**无扩展名**脚本、被 `include!` 或生成的输入，因此只绑
    /// 已知扩展名的闭包会在输入改变之后照样报"inputs unchanged"——这正是这条路径存在的目的所要防的假绿，而规格
    /// 禁止用扩展名白名单签发可复用凭据（`managed-maintenance/basic-repair-efficiency.md` §5 BR-A）。每个常规
    /// 文件都被绑定，符号链接按它的目标绑定，而这趟遍历**绑不上**的条目会让闭包**不完整**，而不是被静默漏掉。
    /// `target`、`.git`、`.codegraph`、`.direnv` 与被排除的输出目录永远不是输入，解析到根之外的输入是错误。
    pub(super) fn capture(root: &Path, excluded: Option<&Path>) -> Result<Self, String> {
        Self::capture_with_env(root, excluded, std::env::vars())
    }

    /// The same capture with the environment supplied by the caller, so a test can bind one without mutating
    /// the process environment.
    /// 同一个捕获，但环境由调用方给入，于是测试可以在**不修改进程环境**的前提下绑定一个。
    pub(super) fn capture_with_env<I>(
        root: &Path,
        excluded: Option<&Path>,
        environment: I,
    ) -> Result<Self, String>
    where
        I: IntoIterator<Item = (String, String)>,
    {
        fn walk_tree(
            root: &Path,
            directory: &Path,
            excluded: Option<&Path>,
            facts: &mut xirang_multilang::facts::Facts,
            rows: &mut Vec<(PathBuf, String)>,
            unbound: &mut Vec<String>,
        ) -> Result<(), String> {
            for entry in std::fs::read_dir(directory).map_err(|e| e.to_string())? {
                let entry = entry.map_err(|e| e.to_string())?;
                let path = entry.path();
                if excluded.is_some_and(|excluded| path.starts_with(excluded)) {
                    continue;
                }
                let name = entry.file_name();
                if ["target", ".git", ".codegraph", ".direnv"]
                    .iter()
                    .any(|skip| name == *skip)
                {
                    continue;
                }
                let file_type = entry.file_type().map_err(|e| e.to_string())?;
                let relative = path.strip_prefix(root).unwrap().to_owned();
                if file_type.is_dir() {
                    walk_tree(root, &path, excluded, facts, rows, unbound)?;
                } else if file_type.is_symlink() {
                    // A symlink is bound by where it points **and** by the bytes that resolve there: retargeting
                    // it changes the closure, and so does changing an external file it reaches. Binding only the
                    // target path let a root-external file change under a passing verdict — the closure said
                    // "unchanged" while the input had changed (BR01 external inputs; 2026-10-18 completion
                    // review, P0). A target whose bytes cannot be bound makes the closure incomplete rather than
                    // silently absent.
                    // 符号链接按它**指向哪里**以及**在那里解析出的字节**绑定：改指向改变闭包，根外文件的内容变了
                    // 也改变闭包。只绑目标路径会让根外文件在判定通过之后改变——闭包说"没变"，而输入已经变了
                    // （BR01 根外输入；2026-10-18 完成性复核，P0）。目标的字节绑不上时，闭包标为**不完整**而不是
                    // 被静默漏掉。
                    let target = std::fs::read_link(&path).map_err(|e| e.to_string())?;
                    let mut bound = format!("link:{}", target.display());
                    match std::fs::canonicalize(&path) {
                        Ok(resolved) if resolved.is_file() => {
                            bound.push('|');
                            bound.push_str(&facts.digest(&resolved)?);
                        }
                        Ok(resolved) => {
                            unbound.push(format!(
                                "{} is a symlink to {}, which is not a regular file, so the content it reaches cannot be bound",
                                relative.display(),
                                resolved.display()
                            ));
                        }
                        Err(_) => {
                            unbound.push(format!(
                                "{} is a symlink whose target cannot be read, so the content it reaches cannot be bound",
                                relative.display()
                            ));
                        }
                    }
                    rows.push((relative, xirang_kernel::sha256_hex(bound.as_bytes())));
                } else if file_type.is_file() {
                    let resolved = path.canonicalize().map_err(|e| e.to_string())?;
                    if !resolved.starts_with(root) {
                        return Err(format!(
                            "verification input outside root: {}",
                            path.display()
                        ));
                    }
                    let digest = facts.digest(&resolved)?;
                    rows.push((relative, digest));
                } else {
                    // A fifo, socket or device: it has no bytes to bind, so the closure is not complete and must
                    // not be presented as if it were.
                    // 命名管道、套接字或设备文件：没有字节可绑，因此这个闭包**不完整**，不得被当作完整来呈现。
                    unbound.push(format!(
                        "{} is not a regular file, so its content cannot be bound",
                        relative.display()
                    ));
                }
            }
            Ok(())
        }
        let root = root.canonicalize().map_err(|e| e.to_string())?;
        // The build and this capture share one file-fact store: `<root>/target/xirang/facts.json`.
        // A candidate slot has no `target/`, so the store stays off there and a verification pass
        // writes nothing into the tree it is verifying.
        // 构建与这里的捕获共用一份文件事实存储：`<root>/target/xirang/facts.json`。候选槽没有
        // `target/`，因此那里缓存关闭、验证过程不会往被验证的树里写任何东西。
        let mut facts =
            xirang_multilang::facts::Facts::load_facts(&root.join("target/xirang"), false);
        let mut rows = Vec::new();
        let mut unbound = Vec::new();
        walk_tree(&root, &root, excluded, &mut facts, &mut rows, &mut unbound)?;
        // A Cargo `path` dependency outside the root is an input too. It is not a symlink, so the walk
        // above never reaches it, and changing the dependency's source would leave this closure saying
        // "unchanged" while the build reads different bytes (BR01 external inputs; 2026-10-19 actual
        // review, R2). The supported range is bounded and honest: every `path` dependency this tree's
        // manifests name, that dependency's own files, and its `path` dependencies to a fixed depth.
        // A dependency whose bytes cannot be read, or a tree too deep, makes the closure **incomplete**
        // rather than silently absent — while a path that simply does not exist is left to Cargo, whose
        // build fails for the same reason a verification would have nothing to certify.
        // 根外的 Cargo `path` 依赖也是输入：它不是符号链接，上面的遍历到不了它，因此改依赖源码会让闭包仍说
        // "没变"而构建读到不同字节（BR01 根外输入；2026-10-19 实际复核 R2）。支持范围有界且诚实：本树各清单
        // 点名的每一条 `path` 依赖、它自己的文件，以及它的 `path` 依赖（固定深度）。读不到的依赖或过深的树让
        // 闭包**不完整**而不是被静默漏掉——而根本不存在的路径交给 Cargo：构建会因为同一个原因失败。
        let mut seen_dependencies = std::collections::BTreeSet::new();
        bind_path_dependencies(
            &root,
            &mut facts,
            &mut rows,
            &mut unbound,
            &mut seen_dependencies,
            0,
        );
        facts.save();
        rows.sort();
        // One read of the environment, used twice: the whole set is bound into the verification identity,
        // and the declared prefixes are the subset the manifest spells out.
        // 一次读取环境，用在两处：整个集合被绑进验证身份，而声明过的前缀是清单逐个写出来的那个子集。
        let mut pairs: Vec<(String, String)> = environment.into_iter().collect();
        pairs.sort();
        let mut env: Vec<(String, String)> = pairs
            .iter()
            .filter(|(key, _)| {
                ENV_INPUT_PREFIXES
                    .iter()
                    .any(|prefix| key.starts_with(prefix))
            })
            .map(|(key, value)| (key.clone(), xirang_kernel::sha256_hex(value.as_bytes())))
            .collect();
        env.sort();
        unbound.sort();
        // The whole environment, sorted so the fold does not depend on iteration order. Values are hashed
        // into the fold rather than kept, so this field cannot leak a secret it happens to see.
        // 整个环境，排序后折叠，使摘要不依赖迭代顺序。取值是被折叠进摘要的，而不是被保存的，因此这个字段不会把
        // 它恰好看到的密钥泄漏出去。
        let whole_environment = xirang_kernel::sha256_hex(format!("{pairs:?}").as_bytes());
        Ok(Self {
            rows,
            env,
            incomplete: (!unbound.is_empty()).then(|| unbound.join("; ")),
            whole_environment,
        })
    }

    /// How many inputs the manifest binds.
    /// 清单绑定了多少项输入。
    pub(super) fn count(&self) -> usize {
        self.rows.len()
    }

    /// The identity a verdict is bound to.
    /// 判定所绑定到的身份。
    ///
    /// Versioned (`v2`) and folded from the whole closure — files, the bound environment, and whether the
    /// closure is complete. A record written before the environment was bound therefore compares **unequal**,
    /// which is the safe direction: an old record stays readable but is never promoted to a reusable
    /// credential it cannot support (BR-A: "旧记录兼容而不升级认证").
    /// 带版本（`v2`）并由**整个闭包**折叠出来——文件、被绑定的环境、以及闭包是否完整。因此，在环境被绑定之前
    /// 写下的记录会比较出**不相等**，而这是安全的方向：旧记录仍可读，但绝不被提升为它支撑不了的可复用凭据
    /// （BR-A：「旧记录兼容而不升级认证」）。
    pub(super) fn digest(&self) -> String {
        xirang_kernel::sha256_hex(
            format!("v2|{:?}|{:?}|{:?}", self.rows, self.env, self.incomplete).as_bytes(),
        )
    }

    /// The identity a **verification** is bound to: the whole closure, environment included.
    /// **验证**所绑定到的身份：整个闭包，含环境。
    ///
    /// Two digests exist because there are two questions, and answering one with the other is how the
    /// false green got in. [`Self::digest`] answers "are these the same bytes" — that is what a candidate's
    /// base identity is about (a tree does not move because a shell variable did). This one answers "would
    /// this verdict be computed the same way again", and for that every environment variable is an input,
    /// because a build script or a `#[test]` may read any of them. Versioned `v3`, so a plan written under
    /// the prefix-only fold compares **unequal** — the safe direction, the same one `v2` took.
    /// 两个摘要并存，因为有两个问题，而用其中一个回答另一个正是那次假绿的来路。[`Self::digest`] 回答"这是同一批
    /// 字节吗"——候选的基树身份问的是这个（一棵树不会因为一个 shell 变量而移动）。这一个回答"这个判定再算一遍会不会
    /// 一样"，而对它来说每个环境变量都是输入，因为构建脚本或 `#[test]` 可能读任何一个。带版本 `v3`，因此按"只绑前缀"
    /// 折叠写下的计划会比较出**不相等**——安全的那一侧，与 `v2` 当初走的是同一侧。
    pub(super) fn verification_digest(&self) -> String {
        xirang_kernel::sha256_hex(
            format!(
                "v3|{:?}|{:?}|{}|{:?}",
                self.rows, self.env, self.whole_environment, self.incomplete
            )
            .as_bytes(),
        )
    }

    /// Why this closure cannot be bound completely, if it cannot.
    /// 这个闭包若不能被完整绑定，原因是什么。
    pub(super) fn incomplete_reason(&self) -> Option<&str> {
        self.incomplete.as_deref()
    }

    /// Write the manifest as `id<TAB>path` lines, so a reader can diff it by eye.
    /// 把清单写成 `id<TAB>路径` 行，使读者能用眼睛比对。
    pub(super) fn write(&self, path: &Path) -> Result<(), String> {
        let mut text = String::new();
        for (relative, digest) in &self.rows {
            text.push_str(digest);
            text.push('\t');
            text.push_str(&relative.to_string_lossy().replace('\\', "/"));
            text.push('\n');
        }
        // The environment rows and the completeness note travel in the same manifest, with a marker that cannot
        // collide with a path, so a reader sees every input class this closure bound.
        // 环境行与完整性说明走同一份清单，并用一个不可能与路径撞车的标记，于是读者看得到这个闭包绑定的每一类输入。
        for (key, digest) in &self.env {
            text.push_str(&format!("{digest}\tenv:{key}\n"));
        }
        if let Some(reason) = &self.incomplete {
            text.push_str(&format!("incomplete\tunbound:{reason}\n"));
        }
        std::fs::write(path, text)
            .map_err(|error| format!("cannot write {}: {error}", path.display()))
    }

    /// Read a manifest back. A missing file is an error: an absent manifest is not "no inputs".
    /// 把清单读回。文件缺失是错误：清单缺席不等于"没有输入"。
    pub(super) fn read(path: &Path) -> Result<Self, String> {
        let text = std::fs::read_to_string(path)
            .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
        let mut rows = Vec::new();
        let mut env = Vec::new();
        let mut incomplete = None;
        for line in text.lines() {
            if let Some(reason) = line.strip_prefix("incomplete\tunbound:") {
                incomplete = Some(reason.to_owned());
                continue;
            }
            let Some((digest, relative)) = line.split_once('\t') else {
                return Err(format!("{}: malformed input line", path.display()));
            };
            if let Some(key) = relative.strip_prefix("env:") {
                env.push((key.to_owned(), digest.to_owned()));
                continue;
            }
            rows.push((PathBuf::from(relative), digest.to_owned()));
        }
        rows.sort();
        env.sort();
        Ok(Self {
            rows,
            env,
            incomplete,
            // A manifest cannot carry the whole environment, and it must not be able to: a file that did
            // would be a writable copy of the thing whose value is that it came from this process. An
            // identity read back from a manifest is therefore not a verification identity — which is why
            // [`Self::verification_digest`] is the only way to mint one, and it is never called on a
            // read-back manifest.
            // 清单载不了整个环境，而且它不该载得：一个载得下的文件，就是"它的价值在于来自这个进程"那件东西的
            // 可写副本。因此从清单读回的身份不是验证身份——这正是只有 [`Self::verification_digest`] 能铸造一个
            // 身份的原因，而它从不在读回的清单上被调用。
            whole_environment: String::new(),
        })
    }

    /// Every input that appeared, disappeared or changed between two manifests.
    /// 两份清单之间出现、消失或改变了的每一项输入。
    pub(super) fn changes(&self, current: &Self) -> Vec<String> {
        self.changes_for("", current)
    }

    /// The same diff with a label in front of an environment row, so a reader can tell a file from a variable.
    /// 同一个差异，但给环境行加一个标签，读者于是能分清文件与变量。
    fn changes_for(&self, _label: &str, current: &Self) -> Vec<String> {
        self.changes_where(current, |_| true)
    }

    /// The same diff, keeping only rows `keep` accepts on **both** sides.
    /// 同样的差异，但两侧都只保留 `keep` 接受的行。
    fn changes_where(&self, current: &Self, keep: impl Fn(&Path) -> bool) -> Vec<String> {
        let before: BTreeMap<&Path, &str> = self
            .rows
            .iter()
            .filter(|(path, _)| keep(path))
            .map(|(path, digest)| (path.as_path(), digest.as_str()))
            .collect();
        let after: BTreeMap<&Path, &str> = current
            .rows
            .iter()
            .filter(|(path, _)| keep(path))
            .map(|(path, digest)| (path.as_path(), digest.as_str()))
            .collect();
        let mut changes = Vec::new();
        for (path, digest) in &after {
            match before.get(path) {
                None => changes.push(format!("added    {}", path.display())),
                Some(was) if was != digest => changes.push(format!("modified {}", path.display())),
                Some(_) => {}
            }
        }
        for path in before.keys() {
            if !after.contains_key(path) {
                changes.push(format!("removed  {}", path.display()));
            }
        }
        // Environment inputs are inputs too: a reader who is told "inputs changed" must be able to see which
        // variable did it, otherwise the reason lives nowhere.
        // 环境输入也是输入：被告知"inputs changed"的读者必须能看到是哪个变量造成的，否则这个原因无处安放。
        let before_env: BTreeMap<&str, &str> = self
            .env
            .iter()
            .map(|(key, digest)| (key.as_str(), digest.as_str()))
            .collect();
        let after_env: BTreeMap<&str, &str> = current
            .env
            .iter()
            .map(|(key, digest)| (key.as_str(), digest.as_str()))
            .collect();
        for (key, digest) in &after_env {
            match before_env.get(key) {
                None => changes.push(format!("added    env:{key}")),
                Some(was) if was != digest => changes.push(format!("modified env:{key}")),
                Some(_) => {}
            }
        }
        for key in before_env.keys() {
            if !after_env.contains_key(key) {
                changes.push(format!("removed  env:{key}"));
            }
        }
        changes
    }

    /// The same diff, ignoring rows under the named relative prefixes.
    /// 同样的差异，但忽略点名相对前缀之下的行。
    ///
    /// A maintenance task's closure question is "did the code and configuration this check bound
    /// change". The knowledge ledger under `.xirang/` is **not** that: its changes are read by the
    /// rules and knowledge obligations, which snapshot their own revisions and lease versions.
    /// Leaving it in the closure diff would make every recorded decision — including the task's own
    /// record — invalidate a verification that was never about it.
    /// 维护任务的闭包问题是"这次检查所绑的代码与配置变了吗"。`.xirang/` 下的知识台账**不是**它：它的变化由
    /// 规则与知识义务按各自记录的修订与租约版本读取。把它留在闭包差异里，会让每一条被记录的决定——包括任务
    /// 自己的记录——去作废一次与它无关的验证。
    pub(super) fn changes_excluding(&self, current: &Self, excluded: &[&str]) -> Vec<String> {
        self.changes_where(current, |path| {
            let spelled = path.to_string_lossy().replace('\\', "/");
            !excluded
                .iter()
                .any(|prefix| spelled == *prefix || spelled.starts_with(&format!("{prefix}/")))
        })
    }

    /// Why this closure cannot be bound completely, when it cannot.
    /// 这个闭包（在不能完整绑定时）为什么绑不完整。
    pub(super) fn incomplete(&self) -> Option<&str> {
        self.incomplete.as_deref()
    }
}

/// Whether a TOML table can carry dependency entries.
/// 一个 TOML 表能否携带依赖条目。
///
/// `[dependencies]`, `[dev-dependencies]`, `[build-dependencies]`, one dependency name under any of
/// them (`[dependencies.foo]`), and the `[target.'cfg(…)'.dependencies]` forms Cargo accepts. A
/// target table (`[lib]`, `[[bin]]`, `[package]`) is not one, whatever its keys are called.
/// `[dependencies]`、`[dev-dependencies]`、`[build-dependencies]`，它们之下的一层依赖名
/// （`[dependencies.foo]`），以及 Cargo 接受的 `[target.'cfg(…)'.dependencies]` 形式。target 表
/// （`[lib]`、`[[bin]]`、`[package]`）不是，无论它的键叫什么。
fn dependency_table(table: &str) -> bool {
    let sections = table.split('.').collect::<Vec<_>>();
    for (index, section) in sections.iter().enumerate() {
        if matches!(
            *section,
            "dependencies" | "dev-dependencies" | "build-dependencies"
        ) {
            return sections.len() - index - 1 <= 1;
        }
    }
    false
}

/// The most `path` dependency hops one closure follows.
/// 一个闭包最多跟随多少层 `path` 依赖。
const MAX_DEPENDENCY_DEPTH: usize = 8;

/// Bind every out-of-root `path` dependency this tree's manifests name, recursively.
/// 绑定本树各清单点名的每一条根外 `path` 依赖，并递归下去。
fn bind_path_dependencies(
    root: &Path,
    facts: &mut xirang_multilang::facts::Facts,
    rows: &mut Vec<(PathBuf, String)>,
    unbound: &mut Vec<String>,
    seen: &mut std::collections::BTreeSet<PathBuf>,
    depth: usize,
) {
    let manifests: Vec<PathBuf> = rows
        .iter()
        .filter(|(path, _)| !path.starts_with("dependency:"))
        .filter(|(path, _)| path.file_name().is_some_and(|name| name == "Cargo.toml"))
        .map(|(path, _)| root.join(path))
        .collect();
    bind_from(root, facts, &manifests, rows, unbound, seen, depth);
}

/// Bind the dependencies the given manifests name, then the manifests of those dependencies.
/// 绑定这些清单点名的依赖，再绑定那些依赖自己的清单点名的依赖。
fn bind_from(
    root: &Path,
    facts: &mut xirang_multilang::facts::Facts,
    manifests: &[PathBuf],
    rows: &mut Vec<(PathBuf, String)>,
    unbound: &mut Vec<String>,
    seen: &mut std::collections::BTreeSet<PathBuf>,
    depth: usize,
) {
    if depth > MAX_DEPENDENCY_DEPTH {
        unbound.push(format!(
            "path dependencies nest deeper than {MAX_DEPENDENCY_DEPTH} levels, so the closure cannot be followed further"
        ));
        return;
    }
    for manifest in manifests {
        for (_spelling, dependency) in path_dependencies(manifest) {
            let Ok(canonical) = dependency.canonicalize() else {
                // A path that does not resolve is Cargo's refusal to make, not ours: there is nothing to
                // bind, and the build that would have read it fails for the same reason.
                continue;
            };
            if canonical.starts_with(root) {
                // Inside the tree: the walk above already bound every byte of it.
                continue;
            }
            if !canonical.is_dir() {
                unbound.push(format!(
                    "{} is a path dependency that is not a directory, so the bytes it reaches cannot be bound",
                    canonical.display()
                ));
                continue;
            }
            if !seen.insert(canonical.clone()) {
                continue;
            }
            let mut files = Vec::new();
            match walk_dependency(&canonical, &mut files) {
                Ok(()) => {}
                Err(reason) => {
                    unbound.push(reason);
                    continue;
                }
            }
            let mut nested = Vec::new();
            for file in files {
                if file.file_name().is_some_and(|name| name == "Cargo.toml") {
                    nested.push(file.clone());
                }
                let Ok(digest) = facts.digest(&file) else {
                    unbound.push(format!(
                        "{} is a path dependency file that cannot be read, so its bytes cannot be bound",
                        file.display()
                    ));
                    continue;
                };
                rows.push((
                    PathBuf::from(format!("dependency:{}", file.display())),
                    digest,
                ));
            }
            bind_from(root, facts, &nested, rows, unbound, seen, depth + 1);
        }
    }
}

/// The `path = "..."` values one manifest names, resolved against that manifest's directory.
/// 一份清单点名的 `path = "..."` 取值，按该清单所在目录解析。
pub(super) fn path_dependencies(manifest: &Path) -> Vec<(String, PathBuf)> {
    let Ok(text) = std::fs::read_to_string(manifest) else {
        return Vec::new();
    };
    let directory = manifest.parent().unwrap_or_else(|| Path::new("."));
    let mut found = Vec::new();
    let mut table = String::new();
    for line in text.lines() {
        // Comments are not declarations. A comment that mentions `path = "..."` used to be bound as a
        // dependency, which made unrelated prose look like an input and put `incomplete` on closures
        // that were fine (2026-10-19 review, finding on false binding).
        // 注释不是声明。提到 `path = "..."` 的注释过去会被当成依赖绑定，让无关散文看起来像输入，并给本来
        // 完整的闭包打上 `incomplete`（2026-10-19 复核关于误绑定的发现）。
        let line = strip_comment(line);
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            table = trimmed
                .trim_matches(|character| character == '[' || character == ']')
                .trim()
                .to_ascii_lowercase();
            continue;
        }
        // **Only dependency tables declare dependencies.** `[lib] path` and `[[bin]] path` are target
        // paths; reading them bound every target of every dependency as "a path dependency that is
        // not a directory", so a host depending on a crate that ships binaries got an `incomplete`
        // closure it could never repair (measured by P5 on a generated host depending on this
        // checkout: sixty-odd bogus entries, and the maintenance task's `verified` was `unknown`).
        // **只有依赖表声明依赖。** `[lib] path` 与 `[[bin]] path` 是 target 路径；把它们读进来会把每个
        // 依赖的每个 target 绑成"不是目录的 path 依赖"，于是依赖一个带二进制的 crate 的宿主得到一份
        // 永远修不好的 `incomplete` 闭包（P5 在一个依赖本检出的生成宿主上量到：六十余条假条目，维护任务
        // 的 `verified` 因此是 `unknown`）。
        if !dependency_table(&table) {
            continue;
        }
        let Some(rest) = key_value(line, "path") else {
            continue;
        };
        let rest = rest.trim_start();
        // TOML has two legal string spellings and Cargo accepts both; reading only `"` left a
        // single-quoted dependency outside the closure and outside the candidate's mirror.
        // TOML 有两种合法字符串拼法，Cargo 都接受；只读 `"` 会让单引号依赖落在闭包之外、也落在候选镜像之外。
        let Some(quote) = rest
            .chars()
            .next()
            .filter(|character| *character == '"' || *character == '\'')
        else {
            continue;
        };
        let rest = &rest[1..];
        let Some(end) = rest.find(quote) else {
            continue;
        };
        let spelling = rest[..end].trim();
        if spelling.is_empty() {
            continue;
        }
        let named = Path::new(spelling);
        let resolved = if named.is_absolute() {
            named.to_path_buf()
        } else {
            directory.join(named)
        };
        found.push((spelling.to_owned(), resolved));
    }
    found
}

/// The value after `key =` on a line, where `key` is a whole TOML key (line start, after `{` or a comma).
/// 一行里 `key =` 之后的取值，其中 `key` 是完整的 TOML 键（行首、`{` 之后或逗号之后）。
fn key_value<'a>(line: &'a str, key: &str) -> Option<&'a str> {
    let mut search = 0;
    while let Some(found) = line[search..].find(key) {
        let at = search + found;
        let before = line[..at].chars().next_back();
        let boundary = before.is_none_or(|character| {
            character == '{' || character == ',' || character.is_whitespace()
        });
        let after = line[at + key.len()..].trim_start();
        if boundary && after.starts_with('=') {
            return Some(&after[1..]);
        }
        search = at + key.len();
    }
    None
}

/// The line without its comment: `#` starts a comment only outside a string.
/// 去掉注释之后的行：`#` 只有在字符串之外才开始注释。
fn strip_comment(line: &str) -> &str {
    let mut quote = None;
    for (index, character) in line.char_indices() {
        match quote {
            Some(active) if character == active => quote = None,
            Some(_) => {}
            None if character == '"' || character == '\'' => quote = Some(character),
            None if character == '#' => return &line[..index],
            None => {}
        }
    }
    line
}

/// Every regular file of one dependency tree, without its build products.
/// 一棵依赖树的每个常规文件，不含构建产物。
fn walk_dependency(directory: &Path, files: &mut Vec<PathBuf>) -> Result<(), String> {
    for entry in std::fs::read_dir(directory).map_err(|error| {
        format!(
            "path dependency {} cannot be read: {error}",
            directory.display()
        )
    })? {
        let entry = entry.map_err(|error| error.to_string())?;
        let path = entry.path();
        let name = entry.file_name();
        if ["target", ".git", ".codegraph", ".direnv"]
            .iter()
            .any(|skip| name == *skip)
        {
            continue;
        }
        let file_type = entry.file_type().map_err(|error| error.to_string())?;
        if file_type.is_dir() {
            walk_dependency(&path, files)?;
        } else if file_type.is_file() {
            files.push(path);
        } else {
            return Err(format!(
                "path dependency entry {} is not a regular file, so the bytes it reaches cannot be bound",
                path.display()
            ));
        }
    }
    Ok(())
}

/// The three files one durable plan owns.
/// 一份持久计划拥有的三个文件。
pub(super) struct PlanFiles {
    /// The ledger the plan's own reply names.
    /// 计划自己的回复所点名的台账。
    pub ledger: PathBuf,
    /// The input manifest it was computed against.
    /// 它据以计算的输入清单。
    pub inputs: PathBuf,
    /// The append-only phase journal.
    /// 只追加的阶段 journal。
    pub journal: PathBuf,
}

/// The file names one plan id owns inside the verification directory.
/// 一个计划 id 在验证目录里拥有的文件名。
pub(super) fn files(directory: &Path, id: &str) -> PlanFiles {
    PlanFiles {
        ledger: directory.join(format!("{id}.json")),
        inputs: directory.join(format!("{id}.inputs.tsv")),
        journal: directory.join(format!("{id}.journal.jsonl")),
    }
}

/// One plan id as the reply writes it: three dash-separated numbers.
/// 回复写下的计划 id：三段以连字符分隔的数字。
pub(super) fn looks_like_plan_id(text: &str) -> bool {
    let parts: Vec<&str> = text.split('-').collect();
    parts.len() == 3 && parts.iter().all(|part| part.parse::<u64>().is_ok())
}

/// Append one phase record. A short single-line append is the whole durability story.
/// 追加一条阶段记录。一行短追加就是全部的持久性保证。
pub(super) fn note(journal: &Path, record: Value) -> Result<(), String> {
    use std::io::Write;
    let mut line = serde_json::to_string(&record).map_err(|error| error.to_string())?;
    line.push('\n');
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(journal)
        .map_err(|error| format!("cannot open {}: {error}", journal.display()))?;
    file.write_all(line.as_bytes())
        .map_err(|error| format!("cannot append to {}: {error}", journal.display()))
}

/// The last phase the journal recorded, when it recorded any.
/// journal 记录到的最后一条阶段（若记录过）。
pub(super) fn tail(journal: &Path) -> Option<Value> {
    let text = std::fs::read_to_string(journal).ok()?;
    text.lines()
        .rev()
        .find(|line| !line.trim().is_empty())
        .and_then(|line| serde_json::from_str(line).ok())
}

/// A phase record's own writer mark.
/// 一条阶段记录自己的写者标记。
pub(super) fn writer_of(record: &Value) -> Option<(String, u64)> {
    let writer = record.get("writer").and_then(Value::as_str)?.to_owned();
    let pid = record.get("pid").and_then(Value::as_u64)?;
    Some((writer, pid))
}

/// Whether another process still owns the run that wrote this record.
/// 写下这条记录的运行是否仍被另一个进程拥有。
///
/// On Linux the process directory is the authority. Where the platform cannot answer, the
/// caller is told `None` — unknown — and must not treat that as "the writer is gone", because
/// guessing wrong would start a second writer on a live plan.
/// 在 Linux 上进程目录就是权威。平台答不上来时调用方得到 `None`——未知——而不能把它当成"写者已经走了"，
/// 因为猜错就会在一个活着的计划上启动第二个写者。
pub(super) fn writer_alive(pid: u64) -> Option<bool> {
    if pid == std::process::id() as u64 {
        return Some(true);
    }
    #[cfg(target_os = "linux")]
    {
        Some(Path::new(&format!("/proc/{pid}")).exists())
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = pid;
        None
    }
}

/// The stable key a task is recognised by, so a later call can find its own plan.
/// 一个任务据以被认出的稳定键，使之后的调用能找到自己的那份计划。
///
/// The key covers the arguments that decide **what work is done** and not the ones that only
/// shape the reply or the output location: a second call with a different `verbose` or
/// `target_dir` is the same task, while a different `face` or package is not.
/// 这个键覆盖**决定做什么工作**的参数，不覆盖只影响回复形状或输出位置的参数：换 `verbose` 或
/// `target_dir` 的第二次调用是同一个任务，而换 `face` 或包名则不是。
pub(super) fn task_key(arguments: &Value) -> String {
    let mut deciding: BTreeMap<&str, Value> = BTreeMap::new();
    for key in [
        "files",
        "cases",
        "face",
        "workspace",
        "package",
        "test",
        "filter",
        "mode",
        "profile",
        "target",
        "all_targets",
        "no_default_features",
        "deny_warnings",
    ] {
        if let Some(value) = arguments.get(key) {
            deciding.insert(key, value.clone());
        }
    }
    let canonical = serde_json::to_string(&deciding).unwrap_or_default();
    xirang_kernel::sha256_hex(canonical.as_bytes())[..16].to_owned()
}

/// The newest ledger in this directory whose recorded task key matches.
/// 这个目录里记录的任务键与之匹配的最新台账。
pub(super) fn newest_for_task(directory: &Path, key: &str) -> Option<PathBuf> {
    let entries = std::fs::read_dir(directory).ok()?;
    let mut candidates: Vec<(std::time::SystemTime, PathBuf)> = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|extension| extension.to_str()) != Some("json") {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        let Ok(ledger) = serde_json::from_str::<Value>(&text) else {
            continue;
        };
        if ledger.get("task").and_then(Value::as_str) != Some(key) {
            continue;
        }
        let modified = entry
            .metadata()
            .and_then(|metadata| metadata.modified())
            .unwrap_or(std::time::UNIX_EPOCH);
        candidates.push((modified, path));
    }
    candidates.sort();
    candidates.pop().map(|(_, path)| path)
}

/// The record one phase writes, with the writer and process that produced it.
/// 一个阶段写下的记录，带上产出它的写者与进程。
pub(super) fn phase(writer: &str, name: &str) -> Value {
    json!({
        "at": std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64,
        "writer": writer,
        "pid": std::process::id(),
        "phase": name,
    })
}

#[cfg(test)]
#[path = "check_task_tests.rs"]
mod check_task_tests;
