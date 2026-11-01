//! The object ledger: which stable object a declaration is, which source instance it currently has,
//! and the directory the author explicitly gave it.
//! 对象台账：一条声明是哪个稳定对象、它当前是哪个源码实例，以及作者显式指定的目录。
//!
//! The ledger is a **project file**, not a cache: it lives at the package root
//! ([`LEDGER_FILE`]), it is copied with the package, and a fresh clone that carries it resolves the
//! same objects. That is the difference between "the last five moves" (a note for the next reader) and
//! an identity mechanism (requirement 7 of the foundation review).
//! 台账是**项目文件**而不是缓存：它住在包根（[`LEDGER_FILE`]），随包复制，带着它的新克隆解析出同一批
//! 对象。这正是"最近五条搬动"（写给下一个读者的提示）与身份机制（基础复核第 7 条）的区别。
//!
//! It answers exactly three questions, and this list is the whole contract:
//! 它恰好回答三个问题，而这份清单就是全部契约：
//!
//! 1. **Which stable object is this declaration?** — [`ObjectMap::own`]. The stable id survives a
//!    physical move, so a record that names the object keeps matching after the file moved.
//! 2. **Which object owns this directory?** — [`ObjectMap::owner_of_directory`]. This is the island
//!    context an `add` needs when the caller names a place rather than a parent. A directory that two
//!    objects both claim is refused instead of guessed.
//! 3. **Where did the object say it lives?** — [`ObjectEntry::directory`]. It is recorded only when a
//!    placement deviated from the default layout; a declaration found elsewhere is **drift**, reported
//!    rather than silently accepted ([`ObjectMap::drift`]).
//!
//! Default placement itself is not a ledger question: a child's default directory is its parent's
//! directory plus `object/<module>`, and the parent's directory is where its own declaration actually
//! is. That is why "descendants inherit the nearest explicit mapping" holds with no per-file
//! registration: the parent's placement — default or explicit — is the thing being inherited.
//! 默认落点本身不是台账的问题：子面的默认目录是父面目录加 `object/<模块>`，而父面目录就是它自己那条声明
//! 实际所在的地方。这就是"后代继承最近显式映射"无需逐文件登记的原因：被继承的是父面的落点——默认的还是
//! 显式的。

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use xirang_kernel::identity::{NodeId, StableFaceId};

/// The ledger's file name at the package root.
/// 台账在包根的文件名。
pub const LEDGER_FILE: &str = "xirang.objects.tsv";

/// The first line of the file. A file whose header is anything else is refused, not guessed: the
/// columns below are a format, and a reader that guesses a format reads a different project.
/// 文件的第一行。表头不是它的文件会被拒绝而不是被猜：下面那些列是一种格式，猜格式的读取方读到的是另一
/// 个项目。
pub const LEDGER_HEADER: &str = "# xirang objects v1";

/// What a missing column is spelled as.
/// 缺失列写成的样子。
const ABSENT: &str = "-";

/// One source instance this object has had: the identity a declaration had at one path.
/// 这个对象曾经有过的一个源码实例：某个路径下那条声明当时的身份。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Instance {
    /// The source-instance identity (`NodeId`) at that moment.
    /// 那时的源码实例身份（`NodeId`）。
    pub id: NodeId,
    /// The package-relative source it was stamped from.
    /// 它当时所依据的包内相对源码。
    pub source: String,
}

/// One object the ledger knows.
/// 台账知道的一个对象。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObjectEntry {
    /// The persistent object identity. A physical move does not change it.
    /// 持续的对象身份。物理搬动不改变它。
    pub stable: StableFaceId,
    /// The logical registration path, e.g. `root/shop/panel/Area`.
    /// 逻辑注册路径，例如 `root/shop/panel/Area`。
    pub path: String,
    /// Logical paths this object used to occupy, oldest first. They remain nameable through the
    /// generated catalog so a third-party declaration keeps resolving after a logical rename.
    /// 这个对象曾经占据过的逻辑路径，最旧的在前。它们经生成目录继续可被点名，使第三方声明在逻辑改名后
    /// 仍然解析到同一对象。
    pub retired_paths: Vec<String>,
    /// The package-relative source the object currently comes from.
    /// 该对象当前来自的包内相对源码。
    pub source: String,
    /// The directory an explicit placement gave this object, when one did.
    /// 一次显式落点给这个对象的目录（当有过这样一次时）。
    pub directory: Option<String>,
    /// Every source instance observed for this object, oldest first, current last.
    /// 该对象被观察到的每个源码实例，最旧的在前、当前的在后。
    pub instances: Vec<Instance>,
}

impl ObjectEntry {
    /// The instance id this entry currently names.
    /// 该条目当前点名的实例身份。
    pub fn current_id(&self) -> Option<NodeId> {
        self.instances.last().map(|instance| instance.id)
    }

    /// Whether this entry has ever been stamped from `source`.
    /// 该条目是否曾经由 `source` 盖过身份。
    pub fn has_source(&self, source: &str) -> bool {
        self.instances
            .iter()
            .any(|instance| instance.source == source)
    }
}

/// One object as the tree currently sees it: derived identity, logical path, package-relative source.
/// 树此刻看到的一个对象：推导出的身份、逻辑路径、包内相对源码。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Observed {
    /// The identity the build derives for the declaration right now.
    /// 构建此刻为这条声明推导出的身份。
    pub stable: StableFaceId,
    /// Its logical registration path.
    /// 它的逻辑注册路径。
    pub path: String,
    /// Its package-relative source.
    /// 它的包内相对源码。
    pub source: String,
}

impl Observed {
    /// Build an observation from the three facts the tree has.
    /// 用树手里的三件事构造一条观察。
    pub fn new(stable: StableFaceId, path: impl Into<String>, source: impl Into<String>) -> Self {
        Self {
            stable,
            path: path.into(),
            source: source.into(),
        }
    }
}

/// The whole ledger.
/// 整份台账。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ObjectMap {
    entries: Vec<ObjectEntry>,
}

impl ObjectMap {
    /// An empty ledger. A package that never deviated from the default layout has no file.
    /// 一份空台账。从未偏离默认布局的包没有这个文件。
    pub fn empty() -> Self {
        Self::default()
    }

    /// Read the package's ledger. A missing file is an empty ledger; an unreadable or malformed one
    /// is an error naming what is wrong and where.
    /// 读取包的台账。文件不存在即空台账；读不了或格式错的是一个点名"错在哪、哪一行"的错误。
    pub fn read_from(package_root: &Path) -> Result<Self, String> {
        let path = package_root.join(LEDGER_FILE);
        match std::fs::read_to_string(&path) {
            Ok(text) => Self::parse_text(&text, &path.display().to_string()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Self::empty()),
            Err(error) => Err(format!("cannot read {}: {error}", path.display())),
        }
    }

    /// Parse a ledger. `origin` names the file in every error message.
    /// 解析一份台账。`origin` 在每条错误里点名那个文件。
    pub fn parse_text(text: &str, origin: &str) -> Result<Self, String> {
        let mut lines = text.lines();
        let header = loop {
            match lines.next() {
                Some(line) if line.trim().is_empty() => continue,
                Some(line) => break line,
                None => {
                    return Err(format!(
                        "{origin}: an empty file; write `{LEDGER_HEADER}` and one line per object, \
                         or delete it to mean `every object keeps the default layout`"
                    ));
                }
            }
        };
        if header.trim() != LEDGER_HEADER {
            return Err(format!(
                "{origin}: the first line is `{header}`, and this reader understands \
                 `{LEDGER_HEADER}`; a ledger of another version is not guessed at"
            ));
        }
        let mut map = Self::empty();
        for (offset, line) in lines.enumerate() {
            let number = offset + 2;
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            let fields: Vec<&str> = line.split('\t').collect();
            if !(fields.len() == 5 || fields.len() == 6) {
                return Err(format!(
                    "{origin}:{number}: a line has {} tab-separated field(s), and this format has \
                     5 (stable id, logical path, source, directory or `{ABSENT}`, instances)",
                    fields.len()
                ));
            }
            let stable = fields[0].parse::<StableFaceId>().map_err(|_| {
                format!(
                    "{origin}:{number}: `{}` is not a 32-digit stable object id",
                    fields[0]
                )
            })?;
            let path = fields[1].trim();
            if path.is_empty() {
                return Err(format!("{origin}:{number}: the logical path is empty"));
            }
            let source = fields[2].trim();
            let relative_source =
                relative_path(source).map_err(|error| format!("{origin}:{number}: {error}"))?;
            let directory = match fields[3].trim() {
                ABSENT | "" => None,
                directory => Some(
                    relative_path(directory)
                        .map_err(|error| format!("{origin}:{number}: {error}"))?,
                ),
            };
            let retained = if fields.len() == 6 && !fields[5].trim().is_empty() {
                fields[5]
                    .split(',')
                    .map(str::trim)
                    .filter(|path| !path.is_empty())
                    .map(str::to_owned)
                    .collect()
            } else {
                Vec::new()
            };
            let instances = parse_instances(fields[4].trim())
                .map_err(|error| format!("{origin}:{number}: {error}"))?;
            map.insert(ObjectEntry {
                stable,
                path: path.to_owned(),
                retired_paths: retained,
                source: relative_source,
                directory,
                instances,
            })
            .map_err(|error| format!("{origin}:{number}: {error}"))?;
        }
        Ok(map)
    }

    /// The text a write puts on disk. Entries are ordered by logical path, so the file is reviewable
    /// as a diff instead of churning whenever an unrelated object moves.
    /// 写入磁盘的文本。条目按逻辑路径排序，因此文件可以当 diff 审阅，而不是每逢无关对象搬动就整篇翻动。
    pub fn render_text(&self) -> String {
        let mut text = format!("{LEDGER_HEADER}\n");
        let mut ordered: Vec<&ObjectEntry> = self.entries.iter().collect();
        ordered.sort_by(|left, right| (&left.path, left.stable).cmp(&(&right.path, right.stable)));
        for entry in ordered {
            let directory = entry.directory.as_deref().unwrap_or(ABSENT);
            let instances = if entry.instances.is_empty() {
                ABSENT.to_owned()
            } else {
                entry
                    .instances
                    .iter()
                    .map(|instance| format!("{}@{}", instance.id, instance.source))
                    .collect::<Vec<_>>()
                    .join(",")
            };
            let retired = if entry.retired_paths.is_empty() {
                String::new()
            } else {
                format!("\t{}", entry.retired_paths.join(","))
            };
            text.push_str(&format!(
                "{}\t{}\t{}\t{}\t{}{}\n",
                entry.stable, entry.path, entry.source, directory, instances, retired
            ));
        }
        text
    }

    /// Every entry, in file order.
    /// 每个条目，按文件顺序。
    pub fn entries(&self) -> &[ObjectEntry] {
        &self.entries
    }

    /// The entry for one stable object.
    /// 某個稳定对象的条目。
    pub fn entry(&self, stable: StableFaceId) -> Option<&ObjectEntry> {
        self.entries.iter().find(|entry| entry.stable == stable)
    }

    /// The entry that has ever been stamped from one package-relative source.
    /// 曾经由某个包内相对源码盖过身份的条目。
    pub fn entry_for_source(&self, source: &str) -> Option<&ObjectEntry> {
        self.entries
            .iter()
            .find(|entry| entry.source == source || entry.has_source(source))
    }

    /// The entry that currently declares one logical path.
    /// 当前声明某个逻辑路径的条目。
    pub fn entry_for_path(&self, path: &str) -> Option<&ObjectEntry> {
        self.entries.iter().find(|entry| entry.path == path)
    }

    /// The entry that ever held one source-instance identity. This is what keeps an old reference
    /// resolvable after a physical move, without asking the recent-move notes.
    /// 曾经持有某个源码实例身份的条目。物理搬动之后旧引用仍能解析，靠的就是它，而不是最近搬动记录。
    pub fn entry_for_instance(&self, id: NodeId) -> Option<&ObjectEntry> {
        self.entries
            .iter()
            .find(|entry| entry.instances.iter().any(|instance| instance.id == id))
    }

    /// Which entry owns one observation, if any.
    /// 一条观察由哪个条目拥有（如果有的话）。
    ///
    /// Three keys are tried and must not disagree: the derived id, the current source, and the
    /// logical path. Two different entries answering for one declaration is a conflict — the honest
    /// answer is to refuse and name both, because the alternative is silently adopting one of them.
    /// 试三把键，而它们不得互相矛盾：推导出的 id、当前源码、逻辑路径。两条不同条目为同一条声明作答就是
    /// 冲突——诚实的答案是拒绝并点名两者，因为另一种做法是静默采用其中一个。
    pub fn own(&self, observed: &Observed) -> Result<Option<&ObjectEntry>, String> {
        let mut found: Vec<&ObjectEntry> = Vec::new();
        for candidate in [
            self.entry(observed.stable),
            self.entry_for_source(&observed.source),
            self.entry_for_path(&observed.path),
        ]
        .into_iter()
        .flatten()
        {
            if !found.iter().any(|known| known.stable == candidate.stable) {
                found.push(candidate);
            }
        }
        self.unique(observed, found)
    }

    /// Which entry **currently** owns one observation, ignoring sources the object only used to have.
    /// 一条观察**当前**由哪个条目拥有——忽略对象只是"曾经拥有"的源码。
    ///
    /// [`Self::own`] answers "which object does this declaration belong to, historically": it keeps
    /// an old reference resolvable after a physical move by matching the instance history. That is
    /// the right question for a reference, and the wrong one for a **live declaration**: after a
    /// move, the old path and source are free, and a different object created there would be claimed
    /// by the moved object's history — exactly the silent re-binding a rule bound to an object must
    /// never suffer. This reading matches only the coordinates an entry has **now** (identity,
    /// current source, current path), so a reused spelling is a different object here.
    /// [`Self::own`] 回答的是"这条声明在历史上属于哪个对象"：它靠匹配实例历史让物理搬动后的旧引用仍可
    /// 解析。对**引用**而言这是对的问题，对**活着的声明**却是错的问题：搬动之后旧路径与旧源码空了出来，
    /// 在那里新建的另一个对象会被被搬对象的历史认领——正是绑定到对象的规则绝不能遭受的静默换绑。本读法
    /// 只匹配条目**现在**拥有的坐标（身份、当前源码、当前路径），因此复用的拼写在这里是另一个对象。
    pub fn own_current(&self, observed: &Observed) -> Result<Option<&ObjectEntry>, String> {
        let mut found: Vec<&ObjectEntry> = Vec::new();
        for entry in &self.entries {
            let current = entry.stable == observed.stable
                || entry.source == observed.source
                || entry.path == observed.path;
            if current && !found.iter().any(|known| known.stable == entry.stable) {
                found.push(entry);
            }
        }
        self.unique(observed, found)
    }

    /// One answer from the entries a declaration matched, or a refusal naming all of them.
    /// 一条声明匹配到的条目给出的唯一答案；多于一个时拒绝并点名全部。
    fn unique<'a>(
        &'a self,
        observed: &Observed,
        found: Vec<&'a ObjectEntry>,
    ) -> Result<Option<&'a ObjectEntry>, String> {
        match found.len() {
            0 => Ok(None),
            1 => Ok(Some(found[0])),
            _ => Err(format!(
                "{} at `{}` answers to {} different ledger entries ({}); one declaration is one \
                 object, so repair `{LEDGER_FILE}` before this tree is read",
                observed.path,
                observed.source,
                found.len(),
                found
                    .iter()
                    .map(|entry| format!("{} ({})", entry.stable, entry.path))
                    .collect::<Vec<_>>()
                    .join(", ")
            )),
        }
    }

    /// The stable object one observation belongs to: the ledger's id when it knows the declaration,
    /// the derived one otherwise.
    /// 一条观察所属的稳定对象：台账认识这条声明时用台账的 id，否则用推导出的那个。
    pub fn object_of(&self, observed: &Observed) -> Result<StableFaceId, String> {
        Ok(self
            .own(observed)?
            .map_or(observed.stable, |entry| entry.stable))
    }

    /// Record an observation: keep the stable id the ledger already has, follow the source when the
    /// object moved physically, and append the new instance.
    /// 记录一条观察：保留台账已有的稳定 id，在对象发生物理搬动时跟随源码，并追加新的实例。
    ///
    /// Returns the entry's stable id, which is the object's identity before and after the call.
    /// 返回该条目的稳定 id——也就是这次调用前后对象的身份。
    pub fn observe(
        &mut self,
        observed: &Observed,
        instance: Option<NodeId>,
    ) -> Result<StableFaceId, String> {
        let stable = self.object_of(observed)?;
        match self.entries.iter().position(|entry| entry.stable == stable) {
            None => {
                self.insert(ObjectEntry {
                    stable,
                    path: observed.path.clone(),
                    retired_paths: Vec::new(),
                    source: observed.source.clone(),
                    directory: None,
                    instances: instance
                        .map(|id| Instance {
                            id,
                            source: observed.source.clone(),
                        })
                        .into_iter()
                        .collect(),
                })?;
            }
            Some(index) => {
                // A logical move may land the object on a path another object already declares;
                // two objects cannot share one registration position.
                // 逻辑搬动可能把对象落到另一个对象已经声明的路径上；两个对象不能共享同一个注册位置。
                if let Some(other) = self
                    .entries
                    .iter()
                    .find(|entry| entry.stable != stable && entry.path == observed.path)
                {
                    return Err(format!(
                        "the logical path `{}` is already declared by stable object {}; one \
                         registration position has one object",
                        observed.path, other.stable
                    ));
                }
                let entry = &mut self.entries[index];
                if entry.path != observed.path
                    && !entry.path.is_empty()
                    && !entry.retired_paths.contains(&entry.path)
                {
                    entry.retired_paths.push(entry.path.clone());
                }
                entry.path = observed.path.clone();
                entry.source = observed.source.clone();
                if let Some(id) = instance
                    && !entry
                        .instances
                        .iter()
                        .any(|known| known.id == id && known.source == observed.source)
                {
                    entry.instances.push(Instance {
                        id,
                        source: observed.source.clone(),
                    });
                }
            }
        }
        Ok(stable)
    }

    /// Insert an entry, refusing the three shapes that would make the ledger ambiguous.
    /// 插入一个条目，拒绝三种会让台账有歧义的情形。
    ///
    /// The refusals are the "明确语义" the review asked for: a stable id declared twice, one logical
    /// path declared twice, and one source standing for two objects. Each names the two rows.
    /// 这些拒绝就是复核要求的"明确语义"：同一个稳定 id 声明两次、同一个逻辑路径声明两次、同一份源码代表
    /// 两个对象。每条都点名两行。
    pub fn insert(&mut self, entry: ObjectEntry) -> Result<(), String> {
        if let Some(existing) = self
            .entries
            .iter()
            .find(|known| known.stable == entry.stable)
        {
            return Err(format!(
                "stable object {} is declared twice (`{}` and `{}`)",
                entry.stable, existing.path, entry.path
            ));
        }
        if let Some(existing) = self.entries.iter().find(|known| known.path == entry.path) {
            return Err(format!(
                "the logical path `{}` is declared by two objects ({} and {})",
                entry.path, existing.stable, entry.stable
            ));
        }
        if let Some(existing) = self
            .entries
            .iter()
            .find(|known| known.source == entry.source)
        {
            return Err(format!(
                "the source `{}` stands for two objects ({} and {}); one declaration file is one \
                 object",
                entry.source, existing.stable, entry.stable
            ));
        }
        self.entries.push(entry);
        Ok(())
    }

    /// Set the directory an explicit placement gave one object, or clear it.
    /// 设置一次显式落点给某个对象的目录，或清除它。
    pub fn set_directory(
        &mut self,
        stable: StableFaceId,
        directory: Option<String>,
    ) -> Result<(), String> {
        let directory = match directory {
            None => None,
            Some(directory) => Some(relative_path(&directory)?),
        };
        let entry = self
            .entries
            .iter_mut()
            .find(|entry| entry.stable == stable)
            .ok_or_else(|| {
                format!("no ledger entry for stable object {stable}; observe it before mapping it")
            })?;
        entry.directory = directory;
        Ok(())
    }

    /// The object that owns one package-relative directory, by the **nearest** mapping that covers it.
    /// 拥有某个包内相对目录的对象，按覆盖它的**最近**映射判定。
    ///
    /// A directory that two entries claim is refused: "multiple objects share a directory" is a
    /// shape this project allows, and the whole point of refusing is that guessing the parent of a
    /// new child there would be inventing a relation nobody wrote down.
    /// 两条条目都声称拥有的目录会被拒绝："多个对象共享目录"是本项目允许的形状，而拒绝的全部意义正是：
    /// 在那里猜一个新增子面的父级，等于发明一条没人写下的关系。
    pub fn owner_of_directory(&self, directory: &str) -> Result<Option<&ObjectEntry>, String> {
        let directory = directory.trim_end_matches('/');
        let covering: Vec<&ObjectEntry> = self
            .entries
            .iter()
            .filter(|entry| {
                entry.directory.as_deref().is_some_and(|mapped| {
                    let mapped = mapped.trim_end_matches('/');
                    directory == mapped || directory.starts_with(&format!("{mapped}/"))
                })
            })
            .collect();
        let deepest = covering
            .iter()
            .map(|entry| {
                entry
                    .directory
                    .as_deref()
                    .map_or(0, |mapped| mapped.trim_end_matches('/').len())
            })
            .max()
            .unwrap_or(0);
        let nearest: Vec<&ObjectEntry> = covering
            .into_iter()
            .filter(|entry| {
                entry
                    .directory
                    .as_deref()
                    .is_some_and(|mapped| mapped.trim_end_matches('/').len() == deepest)
            })
            .collect();
        match nearest.len() {
            0 => Ok(None),
            1 => Ok(Some(nearest[0])),
            _ => Err(format!(
                "`{directory}` lies under {} objects' mapped directories ({}), so it cannot decide \
                 which object a new child belongs to; name the parent explicitly",
                nearest.len(),
                nearest
                    .iter()
                    .map(|entry| format!("{} ({})", entry.path, entry.stable))
                    .collect::<Vec<_>>()
                    .join(", ")
            )),
        }
    }

    /// Declarations whose recorded directory disagrees with where they actually are.
    /// 记录的目录与它实际所在不一致的那些声明。
    ///
    /// A mapping that is not followed is worse than no mapping: the ledger would describe a project
    /// that no longer exists, and the next `add` in that directory would put a child under a parent
    /// that is no longer there.
    /// 没有被遵守的映射比没有映射更糟：台账会描述一个已经不存在的项目，而下一次在那个目录里 `add` 会把子面
    /// 放到一个已经不在了的父级下。
    pub fn drift(&self, observed: &[Observed]) -> Vec<String> {
        let mut rows = Vec::new();
        for entry in &self.entries {
            let Some(directory) = entry.directory.as_deref() else {
                continue;
            };
            let directory = directory.trim_end_matches('/');
            // The declaration is identified by its object or its logical path first: after a
            // physical move its source differs from the recorded one, and that difference is the
            // drift this function exists to report.
            // 先按对象或逻辑路径认出那条声明：物理搬动之后它的源码与记录的不同，而这个差别正是本函数要
            // 报告的漂移。
            let here = observed
                .iter()
                .find(|face| face.stable == entry.stable || face.path == entry.path)
                .or_else(|| observed.iter().find(|face| face.source == entry.source));
            match here {
                None => rows.push(format!(
                    "{} ({}) is mapped to `{directory}` and no declaration is at its recorded \
                     source `{}`",
                    entry.path, entry.stable, entry.source
                )),
                Some(face) if !face.source.starts_with(&format!("{directory}/")) => {
                    rows.push(format!(
                        "{} ({}) is mapped to `{directory}` and its declaration is at `{}`",
                        entry.path, entry.stable, face.source
                    ))
                }
                Some(_) => {}
            }
        }
        rows
    }

    /// Write the ledger back to the package root.
    /// 把台账写回包根。
    pub fn write_to(&self, package_root: &Path) -> Result<PathBuf, String> {
        let path = package_root.join(LEDGER_FILE);
        std::fs::write(&path, self.render_text())
            .map_err(|error| format!("cannot write {}: {error}", path.display()))?;
        Ok(path)
    }
}

/// A package-relative path, normalized; anything absolute or climbing out is refused.
/// 一个包内相对路径，已规范化；绝对路径或向上越界的都被拒绝。
pub fn relative_path(value: &str) -> Result<String, String> {
    let value = value.trim().trim_start_matches("./");
    if value.is_empty() {
        return Err("an empty path is not a package-relative path".to_owned());
    }
    let path = Path::new(value);
    if path.is_absolute()
        || path
            .components()
            .any(|component| !matches!(component, std::path::Component::Normal(_)))
    {
        return Err(format!(
            "`{value}` is not a package-relative path (absolute paths and `..` belong to this \
             machine, not to the project)"
        ));
    }
    Ok(path.to_string_lossy().replace('\\', "/"))
}

/// Parse the instances column.
/// 解析实例那一列。
fn parse_instances(value: &str) -> Result<Vec<Instance>, String> {
    if value.is_empty() || value == ABSENT {
        return Ok(Vec::new());
    }
    let mut instances = Vec::new();
    let mut seen = BTreeMap::new();
    for item in value.split(',') {
        let (id, source) = item.split_once('@').ok_or_else(|| {
            format!("`{item}` is not an instance; the shape is `<32-digit id>@<source>`")
        })?;
        let id = id
            .trim()
            .parse::<NodeId>()
            .map_err(|_| format!("`{id}` is not a 32-digit instance id"))?;
        let source = relative_path(source)?;
        if let Some(previous) = seen.insert(id, source.clone()) {
            return Err(format!(
                "instance {id} is listed twice (from `{previous}` and `{source}`)"
            ));
        }
        instances.push(Instance { id, source });
    }
    Ok(instances)
}

#[cfg(test)]
#[path = "object_map_tests.rs"]
mod object_map_tests;
