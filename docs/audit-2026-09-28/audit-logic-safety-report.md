# 逻辑与安全总账（audit-logic-safety-report）

- 范围：全树「大逻辑 / 函数级谬误与假实现 / 安全 / 可靠性」的合并总账（用户第 ⑦ 条）。
- 输入：本路 `audit-logic-hunt.md`（LH-01…LH-11）+ 五路片区报告（`audit-lane-{kernel,surfaces,studio,bridges,gates}.md`）+ 四份复核报告（`audit-verify-{kernel,surfaces,studio,gates}.md`）+ 本路 `audit-verify-bridges.md`，以及队长转达的 X-1/X-2/V-01…V-03 与既有条目的修正口径。
- 本轮只出报告，源码一行未改。文件名按队长 2026-09-28 规则用 `audit-` 前缀（任务书写的是 `logic-safety-report.md`）。
- 合并去重：重复出现的同一缺陷只留一条，用 `= 别名` 列出所有来源 id；跨片共有的机制在同一节里指认一次。
- **复核状态图例**：`✅实测`＝有可跑的探针命令与输出（写明谁跑的）；`🔍复核`＝复核报告已用与作者不同的手段确认；`📖读码`＝只有代码阅读佐证；`⏳unverified`＝本轮无人独立复核，**视为未证实**。
- 严重度：CRITICAL＝静默错答 / 身份或数据损坏 / 安全不可逆；MAJOR＝明确行为错误、可达 abort、假实现、门禁静默失效；MINOR＝局部一致性 / 错误上下文 / 自述欠说明。

## 0. 总表

| id | 严重度 | 一句话 | 来源 | 复核状态 |
| --- | --- | --- | --- | --- |
| LG-01 | CRITICAL | 进程级身份缓存的键不含命名空间，跨包污染 `NodeId` | LH-01 | ✅实测（t8，冷/热 + 三包六序） |
| LG-02 | CRITICAL | 创作面「读一次再写回」静默丢弃 admission 的 deny 列表，门禁被放宽且不可逆 | K-01 | ✅实测（t9 探针；本轮同码复核） |
| LG-03 | CRITICAL | Studio 插件写盘路径能产出解析器拒绝的锁，界面报「已选中」 | S-01 | 🔍复核（t11）＋📖读码（本轮） |
| LG-04 | MAJOR | 插件锁 schema 门禁：按行序生效、对空锁不生效、表头拼错即静默关闭 | K-07 = X-2 | ✅实测（t9；本轮独立探针四形态） |
| LG-05 | MAJOR | `flow_provider` 编辑入口缺内核已有的嵌套守卫 → 极短输入即可 abort | S19 = K-22 | ✅实测（t3 abort 134；本轮独立复现 abort+s 读码可达链） |
| LG-06 | MAJOR | 同一「trait 契约」规则两处实现：overlay 非 full 分支不查 `needs_registry` | K-04 | 🔍复核（t9 实测叶子仍可注册） |
| LG-07 | MAJOR | 兄弟同名 `registry_name` 无人拒绝，路径这一事实键三处静默碰撞 | K-05 | 🔍复核（t9） |
| LG-08 | MAJOR | `FaceView.path` 顺序依赖，与运行期 `path_for` 可不同判 | S1 | 🔍复核（t13） |
| LG-09 | MAJOR | 构建期「货币凭据」先于载荷发布，载荷非事务写 → 混代产物被读成 current | S2 | 🔍复核（t13） |
| LG-10 | MAJOR | 模块迁移对整棵子树做无边界文本替换，兄弟模块被静默改写 | S5 | 🔍复核（t13） |
| LG-11 | MAJOR | `face_views` 静默丢弃解析失败的注册面文件，并把子面改挂根（打印运行期从未有过的路径） | LH-02 | ✅实测（t8） |
| LG-12 | MAJOR | `diff records:true` 只在「身份在树里」那一支查「是否被声明」→ 与 `xirang.grafts` 结论互相矛盾 | LH-03 = BR-8 | ✅实测（t8/t12） |
| LG-13 | MAJOR | `XIRANG_NAMESPACE` 下 `verify` 用 Cargo 名发布、`diff`/`search` 用覆盖名读 → 每个面都被自信地报成 re-identified | BR-6 | ✅实测（t12） |
| LG-14 | MAJOR | `linked` collector 分支什么都不提交，模块文档却说它决定进哪个链接段 | S9 | 📖读码（本轮） |
| LG-15 | MAJOR | 剪枝清单的符号列是逐行匹配魔法标识符得到的，其中一支无条件返回 `Button::…` | S3 | ✅实测/读码（t13 计数 + 本轮 grep：全仓仅两处） |
| LG-16 | MAJOR | `compile_error_demo` 这个魔法目录/文件名改写任意宿主的构建结果 | S7 | 📖读码（本轮，四处调用点） |
| LG-17 | MAJOR | authoring 的 source 回落链硬编码本仓库夹具目录名，宿主同名目录会被截断 | S8 | 📖读码（本轮） |
| LG-18 | MAJOR | MIR `jsonl:true` 可给任意可读文件盖上本包快照表头（来源可证 → 来源可造） | LH-04 | ✅实测（t8） |
| LG-19 | MAJOR | 预览副本跟随目录符号链接走出包根且无环/深度守卫 | LH-06 | ✅实测（t8） |
| LG-20 | MAJOR | 预览副本的临时路径可预测：同名既有目录会被删除重建（并留泄漏） | LH-07 = BR-19 | ✅实测（t12：目录被删 + `-416016-0` 残留） |
| LG-21 | MAJOR | `apply edit` 静默忽略 `handle_contracts`/`part_contracts`，而自述与白名单都把它们列为可写 | NEW-B1 | ✅实测（t12） |
| LG-22 | MAJOR | 非 UTF-8 请求帧结束整个桥，而三类同类畸形帧都被作答（与 README「不写 stderr」相反） | BR-5 | ✅实测（t12，exit 1 / 0 行 stdout） |
| LG-23 | MAJOR | 预览 diff 把每个非 UTF-8 文件报成「新增」，无上限，且每次预览复制 `.git` | BR-4 | ✅实测（t12） |
| LG-24 | MAJOR | `lint` 把 `#![deny(warnings)]` 当作「带着 missing_docs」，保护可被静默移除 | G-01 | ✅实测（t14 + 本轮 rustc 复现） |
| LG-25 | MAJOR | 发布工作流的 tag 守卫用子串黑名单判否，`== false` 等价否定写法整类放行 | G-02 | 🔍复核（t14） |
| LG-26 | MAJOR | 持有 `CARGO_REGISTRY_TOKEN` 的 job 的 action pin 只由注释承诺，无门禁 | G-13 | 🔍复核（t14） |
| LG-27 | MAJOR | `merge_authored` 静默忽略作者改过的 `runtime_checks`，热重载把编辑吃掉 | K-02 | 🔍复核（t9 探针） |
| LG-28 | MAJOR | 畸形 `requires` 条目在校验器与快照解析器之间口径不同：一处拒绝、一处静默丢；桥的两个工具再给出两种结论 | K-06 = BR-10 | ✅实测（t9/t12 读码对照）+📖读码（本轮 rules.rs） |
| LG-29 | MAJOR | 模块级 `cfg` 一律跳过，特性已开启也照跳 → 发布的注册面缺失 | K-08 | 🔍复核（t9，三种形态） |
| LG-30 | MAJOR | trait 标签派生遇带逗号的泛型实参失败 → 构建期误报「缺 trait」 | K-09 | 🔍复核（t9） |
| LG-31 | MAJOR | trace artifact 的字符串驻留是进程级无回收 `Box::leak`，长寿命桥可被逐次喂大 | LH-05 | ✅实测（t8，+6.5 MB/10 万串） |
| LG-32 | MAJOR | 子进程已给完整帧却不退出时，答案被丢成 `Timeout`（与注释承诺相反） | LH-08 = BR-20 = BR-C1 | ✅实测（t8/t12，`Err(Timeout) after 303ms`） |
| LG-33 | MINOR（原 MAJOR） | `validate_*` 通过而 `apply_*` 失败时，错误文案指错原因（机制已被证伪） | K-03 | 🔍复核（t9 证伪机制） |
| LG-34 | MINOR（原「数据丢失」） | Studio Edit 表单在文件读不到时显示空契约（写盘不会抹掉；真缺陷是 LG-21） | V-03 | ✅实测（t12 证伪数据丢失） |
| LG-35…LG-58 | MINOR | 其余一致性/错误上下文/自述欠说明条目 | 见 §5/§6 表 | 见各表 |

---

## 1. 大逻辑（跨 crate 的流程与不变量）

### LG-01 CRITICAL — 进程级身份缓存的键不含命名空间，跨包污染 `NodeId`
- **=** LH-01（本路 t8）
- **影响面**：`build_method` 的一次构建/校验发布的**每一个** `NodeId`；下游 `explain`/`registry`/`search`/`diff`/`graft` 记录解析、发布剪枝选择集（`cache.rs` 的 `collect_active_ids`/`source_is_active`）与 `contracts.rs`。
- **触发条件**：长寿命进程（MCP 桥的 `verify`、Studio、跑多包的测试二进制）+ 两个包有相同相对源码路径 + 该进程的**第一次** `prime_node_id_cache` 能读到 unit 文件（此前跑过一次 `cargo build`/`xirang check` 即满足；若设了 `CARGO_TARGET_DIR`，unit 目录跨包共享，两步即可）。
- **最小复现**：`/tmp/nichprobe/idprobe`——同一 fixture 复制成 A/B/C 三包，只改包名：
  ```text
  冷 unit 缓存：A→B 两跑均 ok
  热 unit 缓存：A→B → probe-b FAILED（phase=static plan, field=parent, actual=69fb465d… B 的身份, parent node is missing）
              B→A / C→A → ok（其 unit 属别的命名空间，写入侧校验拒成空 map）
  ```
  另一个可观察痕迹：失败那次运行的 `pruning_manifest.tsv` 是**正确**的 B 身份，而拓扑校验用的是 A 的——同一次运行里两处 id 来源不同（`build_method/static_plan.rs:117` 用缓存、`build_method/manifests.rs:181-184` 重算）。污染还会被写进下一份 unit 缓存（实测 B 的 unit 里存着 `probe-c` 的 id）。
- **修复方向**：缓存键加命名空间（`(namespace, relative) → (NodeId, kind)`）；`node_id()` 命中后再用 `package_node_id(&relative, &kind)` 复核；或让 `run_as_package` 结束即清。
- **不修会怎样**：同进程第二个包得到「看起来正确」的混合命名空间树；实测方向是假失败（安全），但同一缓存值也流进作用域/剪枝选择集，那里没有第二道交叉校验 → 假通过、剪错符号集。
- **复核状态**：✅ 我本人的 t8 探针（冷/热 + 六序矩阵 + unit 文件内容对照）；t2/t3 两路报告与本条无冲突。

### LG-02 CRITICAL — 创作面「读一次再写回」静默丢弃 admission 的 deny 列表
- **=** K-01（kernel-auditor 的 1 号 CRITICAL）
- **影响面**：`Admission` 是外部依赖门禁（`Admission::accepts` 里 deny 优先）。经 Studio/MCP 任一字段编辑后写回，deny 从源文件消失 → 门禁放宽，且写回是源码级、不可逆。
- **触发条件**：某面同时写 `allow` 与 `deny`（被内核与测试支持的写法，`core/src/registry_core/declaration/declaration.rs:344` 就是这种夹具），对它做一次任何字段的编辑并保存。
- **最小复现**：本轮的 `/tmp/nichverify/synthprobe`（独立于 t9 的 `/tmp/nk-probe`）：
  ```text
  K01.parse = Ok("allow:ui")
  K01.original.accepts(ui/experimental) = false
  K01.render_admission(allow:ui) = Ok("crate::Admission::new(&[\"ui\"], &[])")
  ```
  即 `Admission::new(&["ui"], &["ui/experimental"])` → `allow:ui` → `crate::Admission::new(&["ui"], &[])`。
- **修复方向**：紧凑语法承载两张列表（如 `allow:a,b;deny:c`）；或两列表同时非空时 `Err`（不允许静默丢字段），并在 `render_admission` 同步拒绝。
- **不修会怎样**：一次无关编辑即可放宽门禁；内核自己的往返测试只覆盖 allow-only/deny-only/ANY 三形态，**恰好没有两列表同时非空**那一种，所以没有任何东西会发现。
- **复核状态**：✅ t9 探针（读→写→门禁三跳）；本轮同代码独立复现（同一函数、独立探针工程）。

### LG-03 CRITICAL — Studio 插件写盘路径能产出解析器拒绝的锁，界面报「已选中」
- **=** S-01（studio）
- **影响面**：插件锁是**准入**的唯一输入（`plugin-host/src/admission.rs:57-82` 读它）。写出一份解析器拒绝的锁，等于把宿主准入打瘸；恢复需要手改文件。
- **触发条件**：plugins 表单里改 checksum，或按一次 Enter 切 mode 再按两次 s（作者给出的两条路径）；user 来源完全绕过身份检查，写入前只有「整行字符串相等」与「official 才走 `contains_record`」两道闸。
- **最小复现**：作者未给可跑探针；**t11 复核**用与作者不同的装置确认了「写盘闸门不足以产生内核接受的记录」这一半。内核侧的事实可独立验：`PluginCatalog::parse` 把身份五元组重复判为硬错误（`core/src/registry_core/plugin/catalog/catalog.rs:192-207`，内核用例 `:325-332`）。
- **修复方向**：追加记录交给内核唯一的解析/校验入口（先 render，再 parse 回读，成功才落盘）。
- **不修会怎样**：界面说成功、宿主读不了锁；官方/用户两条来源都受影响（user 更宽）。
- **复核状态**：🔍 t11 复核（结论：证实）；本轮只做代码阅读佐证（读写两端的判定位置），未构造 TUI 探针。

### LG-04 MAJOR — 插件锁 schema 门禁可按外观与行序被静默绕过
- **=** K-07（MAJOR）+ X-2（t9 新发现，原评 MINOR）
- **影响面**：`# xirang-schema=` 是锁与内核身份 schema 的对账闸门。它一旦不生效，用另一版身份语义写下的锁会被当作当前语义读入，进而影响准入与身份解释。
- **触发条件**（四种形态，本轮的 `/tmp/nichverify/synthprobe` 一次跑出）：
  ```text
  X2.typo_header（`# xirang-schema v9`，少一个 `=`）= Ok(1)      ← 被当普通注释吞掉，门禁关闭
  X2.correct_header_wrong_version（`# xirang-schema=v9`）      = Err("uses identity schema v9, expected v3")
  K07.header_only_no_records                                    = Ok(0)   ← 无记录 ⇒ 检查不跑
  K07.header_after_record                                       = Ok(1)   ← 表头在记录之后 ⇒ 之前的记录从不检查
  K07.comments_only                                             = Ok(0)
  ```
  代码位置：`core/src/registry_core/plugin/catalog/catalog.rs:131-137`（`strip_prefix("# xirang-schema=")` 失败即落入 `starts_with('#') => continue`）与 `:138-145`（schema 检查在记录循环内）。
- **修复方向**：表头识别改为「以 `# xirang-schema` 开头就进 schema 处理」（拼错即报错，而不是当注释）；schema 校验提到循环之前，对整份文件（含无记录）生效；表头出现在记录之后即拒绝。
- **不修会怎样**：门禁的开启条件落在「某一行恰好长成那个样子」而不是「它在该在的位置」——见 §5.2；今天 schema 值只有 `3`，所以影响是潜在放行，一旦规则改版即成为实际错读。
- **复核状态**：✅ t9（K-07 三形态 + X-2）；本轮用独立探针工程重跑四种形态，结论一致。**严重度**：把 K-07 与 X-2 合成一条 MAJOR；X-2 单独时仍是 MINOR（潜在），合并后按「门禁可静默失效」计。

### LG-05 MAJOR — `flow_provider` 编辑入口缺内核已有的嵌套守卫 → 极短输入即可 abort
- **=** S19（surfaces，由 kernel-auditor 转达）+ K-22（kernel，t9 建议上调 MAJOR，同一机制的另一入口）
- **影响面**：库内 **abort**（不是 `Result`）——栈溢出会带走整个宿主进程/桥进程。可达入口包括 authoring 的 `edit`（Studio 表单、MCP `apply`）与公开 API 的调用方预算递归。
- **触发条件**：`flow_provider` 或同类类型路径文本深度超出调用线程可用栈。内核在 `core/src/registry_core/authoring/parse/flow.rs:107` 有 `guard_nesting`（注释自己写明「没有守卫时 542 字节的类型路径会以栈溢出 abort」），而 `run_method/authoring/manifest/face/face.rs:140-143` 直接 `syn::parse_str::<syn::Path>`，且 `flow_provider` 确实在 `EDIT_FIELD_ORDER`（`run_method/authoring/operations/face_write.rs:79`）里 → 编辑路径先于任何守卫执行。
- **最小复现**（本轮的 `/tmp/nichverify/synprobe`，独立于 t3 的 `/tmp/xirang-probe-surfaces/probe_syn`）：
  ```text
  $ ./target/release/synprobe 100          # 256 KiB 栈，输入 301 字节
  input bytes = 301
  thread '<unknown>' has overflowed its stack
  fatal runtime error: stack overflow, aborting
  exit 134（SIGABRT，core dumped）
  ```
  t3 的实测：8 MiB 栈下 200 层正常、300 层（901 B）abort；256 KiB 栈下 100 层即 abort。
- **修复方向**：把内核的 `guard_nesting`（已公开）接到 run_method 的这一处；K-22 那条把「按调用方预算递归」的公开 API 改成受本模块上界约束。
- **不修会怎样**：一个几百字节的字段值就能 abort 宿主进程；错误不是可捕获的 `Err`，宿主没有恢复余地。
- **复核状态**：✅ abort 本轮实测（exit 134）+ 📖 可达链读码（`run_method/authoring/operations/operations.rs:318-332` → `run_method/authoring/operations/face_write.rs:79` → `run_method/authoring/manifest/face/face.rs:140`）；t9 对 K-22 亦有独立实测。

### LG-06 MAJOR — 同一「trait 契约」规则两处实现：overlay 非 full 分支不查 `needs_registry`
- **=** K-04（kernel）
- **影响面**：嫁接（overlay）后的注册树不变量：一个不再声明 `needs_registry` 的面不该留着非空子注册机。
- **触发条件**：对一个有子注册机的面做**非 full** 嫁接，候选声明 `needs_registry: false`。
- **最小复现**：t9 用探针实测「该『叶子』仍可被注册」（t9 §2.4）；本轮 📖 复核了两处实现的分叉位置（`core/src/registry_core/tree/graft_ops/overlay.rs:258-309` vs `core/src/registry_core/tree/graft_ops/graft_ops.rs:261-264`）。
- **修复方向**：overlay 分支复用 `replace_info` 的那条判定（或把规则提到一处）。
- **不修会怎样**：同一棵树经「注册」与「嫁接」两条路得到不同的自洽性结论；`validate` 与 `apply` 的语义分歧会在嫁接后留下不自洽的树。
- **复核状态**：🔍 复核（t9 实测）；本轮读码佐证。

### LG-07 MAJOR — 兄弟同名 `registry_name` 无人拒绝，路径这一事实键三处静默碰撞
- **=** K-05（kernel）
- **影响面**：`path`（`root/a/b`）是 index/`by_path`、`path_for`、`resolve_path` 的事实键；同名兄弟会让「用路径点名一个面」不再唯一。
- **触发条件**：同一父下两个面写同一个 `registry_name`（两处宏调用即可）。
- **最小复现**：t9 复核证实（t9 §2.5）；本轮未复跑探针。判据是 `core/src/registry_core/tree/transaction/transaction.rs:98-179` 的三处静默行为。
- **修复方向**：注册/静态校验阶段按父级检查 `registry_name` 唯一（错误带两个来源），而不是让路径键先到先得。
- **不修会怎样**：`explain <path>`/`apply node=<path>` 命中不确定的那个；graft 记录与 manifest 都按路径写，后续对账会指错面。
- **复核状态**：🔍 复核（t9）；⏳ 本轮无新证据。

### LG-08 MAJOR — `FaceView.path` 顺序依赖，与运行期 `path_for` 可不同判
- **=** S1（surfaces）
- **影响面**：`build_method` 的只读视图 → CLI `explain`、MCP `registry`/`search`/`overlay`（`build_method/graft_view/overlay_rows.rs:137` 用它判「哪个槽位被替换」）。
- **触发条件**：子面在发现顺序里排在父面之前（`logical_path` 只走一层、父路径缓存未命中即回落 `root`）。
- **最小复现**：t13 复核（t13 §S1）；现有测试只断言父面 path，**刻意没断言子面**，所以没有任何东西会红。
- **修复方向**：父链递归解析（或先按依赖序排序再算路径）；补一条子面 path 的钉子。
- **不修会怎样**：同一个面在只读视图与运行期注册树里路径不同 → overlay/graft 判到另一个槽位（静默）。
- **复核状态**：🔍 复核（t13）；本轮读码确认两处实现（`build_method/face_view.rs:280-298` vs `core/src/registry_core/tree/query/query.rs:86-96`）。

### LG-09 MAJOR — 构建期「货币凭据」先于载荷发布，载荷非事务写
- **=** S2（surfaces）
- **影响面**：`target/xirang/out` 是 `explain`/`registry`/`diff`/`search` 的「构建侧真值」；混代产物会被 `build_output_is_current` 读成 `current`。
- **触发条件**：五份载荷逐个写的中途失败（磁盘满、权限、被中断）；或载荷写入被截断（`build_method/cache.rs:36-39` 的 `fs::write` 就地截断）。
- **最小复现**：t13 复核（t13 §S2）；本轮 📖 复核了顺序（`build_method/pipeline.rs:155-164` 指纹在前、`:174-184` 五份载荷在后）。
- **修复方向**：载荷先写、指纹最后且仅在全绿时发布；`write_if_changed` 改临时文件 + rename。
- **不修会怎样**：读者把上一代的行当本代现状作答（静默错答），且 `current` 为真让这错答案看起来是新鲜的。
- **复核状态**：🔍 复核（t13）；本轮读码佐证。

### LG-10 MAJOR — 模块迁移对整棵子树做无边界文本替换
- **=** S5（surfaces）
- **影响面**：authoring 的 rename 路径（Studio / MCP `apply rename`）。
- **触发条件**：被迁移模块名是子树内另一个模块名的**前缀**（如 `control` 与 `control_extra`）。
- **最小复现**：t13 复核（t13 §S5）；同仓库 `build_method/scope_view.rs:109-116` 对同一件事守了 `::` 边界，是现成的对照口径。
- **修复方向**：用 `::`/`/` 边界匹配（复用 `build_method` 那套规则）。
- **不修会怎样**：兄弟模块被静默改写（数据损坏），文本编辑后编译器才可能报出别的错。
- **复核状态**：🔍 复核（t13）。

### LG-11 MAJOR — `face_views` 静默丢弃解析失败的注册面文件，并把子面改挂根
- **=** LH-02（本路 t8）
- **影响面**：MCP `registry`/`search`/`diff`/`impact`/`usages`/`converge` 的「树那一半」；`trace` 的身份校验也吃它（`known` 集合来自 `face_views`）。
- **触发条件**：包内任意一个注册面文件解析失败（两处宏调用、字段写坏、缺 `kind`）。
- **最小复现**（t8，实测）：fixture 的 `control.rs` 追加第二个宏调用 →
  ```text
  cargo-xirang xirang check → phase=face-syntax source=control/control.rs:43
  xirang.registry            → faces 2；root/button、root/slider（parent-unresolved）；Control 面消失
  ```
  即「构建说有文件坏了，视图只少报，并且打印出运行期从未有过的逻辑路径 `root/button`」。
- **修复方向**：`face_views` 返回不可读面的计数/清单；消费端在回复里说「有 N 个面文件解析不了」；子面保持 `parent-unresolved` 不回退路径。
- **不修会怎样**：agent 会拿一个更小且路径被改写的树去做编辑（`apply` 的 `node` 用「`xirang.registry` 报告的逻辑路径」点名）。
- **复核状态**：✅ 实测（t8）；t3 的「不可读面文件会被 entry 阶段报错」结论与之不冲突（那条讲构建，这条讲视图）。

### LG-12 MAJOR — `diff records:true` 的「是否被声明」只查一支，与 `xirang.grafts` 互相矛盾
- **=** LH-03（本路）+ BR-8（bridges，同一处的工具自述半边）
- **影响面**：agent 对 graft 记录健康度的判读；`undeclared`（会被发布剪枝、永不生效）是行动含义最强的一桶。
- **触发条件**：一条记录的身份不在树里（典型：槽位没动、身份换了）且宿主入口用**类型化**切口点名该槽位（脚手架宿主默认写法）。
- **最小复现**（t8/t12，实测）：只放一条 `moved_identity`：
  ```text
  xirang.diff {"records":true} → ok 0  undeclared 0  stale 0  re-identified 1  unreadable 0
  xirang.grafts                → [NOT declared by the host entry] / unkept plans 1: the release prunes these slots
  ```
- **修复方向**：把 `row.declared` 的应用提到桶分流之外（两维结论）；`plan_rows.rs` 在身份缺席时用「同路径的面」解析 module，使类型化切口也能被判为已声明。
- **不修会怎样**：同一条记录两个工具给出相反的健康结论，`undeclared 0` 是错的计数。
- **复核状态**：✅ 实测（t8 首次、t12 复核命令同形）。

### LG-13 MAJOR — `XIRANG_NAMESPACE` 下 `verify` 与 `diff`/`search` 各用一套命名空间
- **=** BR-6（bridges）
- **影响面**：桥自述「verify 驱动与 CLI check 同一个入口」；一旦两侧命名空间不同，「刚校验过」的树被报成每个身份都变了。
- **触发条件**：宿主显式设 `XIRANG_NAMESPACE`（桥在无分包根时**自己建议**设它），然后先 `verify` 再 `diff`/`search`。
- **最小复现**（t12 实测）：
  ```text
  XIRANG_NAMESPACE=alternate-ns
  verify → verdict ok；diff → build current, faces 3 vs 3, added 0 gone 0 reidentified 3
                            ~ root/control 12df60c3…（Cargo 名的身份） -> 8ea79f01…
  search Button → [re-identified (371236fe… -> df5b6a7e…)]
  ```
  其中 `12df60c3…`/`371236fe…` 是 Cargo 名 `probe-host` 下的身份（独立算出）。
- **修复方向**：`verify` 用 `registry::namespace(root)` 作 `check_for` 的 package；或两侧不一致时明确拒绝。
- **不修会怎样**：一份「刚校验通过」的树被自信地报成「每个面都换了身份」。
- **复核状态**：✅ 实测（t12）；作者原本只有代码判据。

### LG-14 MAJOR — `linked` collector 分支什么都不提交，而文档说它决定「进哪个链接段」
- **=** S9（surfaces）
- **影响面**：宏契约的自我描述；宿主按文档选 `linked` 会以为注册进了静态段。
- **触发条件**：宿主/工具按模块文档选择 `linked` collector。
- **最小复现**：📖 本轮读码：
  ```text
  run_method/macros/face_registration.rs:9-10  每个字段假定给出，收集器标识决定注册信息进入哪个链接器段
  run_method/macros/face_registration.rs:168-171  (linked; …) => { /* 只有注释，什么都不提交 */ }
  ```
- **修复方向**：要么实现链接段提交，要么把文档改成「`linked` 目前不提交任何东西（保留位）」。
- **不修会怎样**：宿主以为自己在用链接收集器，实际静态计划才是唯一来源；文档与行为不一致属本仓明文标准下的缺陷。
- **复核状态**：📖 读码（本轮）；t13 已复核同一处（S9）。

### LG-15 MAJOR — 剪枝清单的符号列由魔法标识符匹配产生，一支无条件返回 `Button::…`
- **=** S3（surfaces）
- **影响面**：`pruning_manifest.tsv` 是**发布剪枝**前读的证据文件。
- **触发条件**：某面（不叫 `Button`）定义了 `fn optional_pruning_probe`；或某行含 `PRUNING_TABLE`（含注释/字符串）。
- **最小复现**：📖 本轮读码 + 全仓 grep（`--exclude-dir=target`）：
  ```text
  build_method/manifests.rs:208  } else if line.contains("optional_pruning_probe") && line.contains("fn optional_pruning_probe")
  build_method/manifests.rs:210      "Button::optional_pruning_probe"
  ```
  三个魔法标识符全仓只有这几行（无声明者、无测试）；被命名的那一面并不存在。
- **修复方向**：符号列从已解析的注册面/静态计划读出，而不是逐行文本匹配；删除其余魔法名分支。
- **不修会怎样**：产物里出现与本面无关（前缀还是别的面）的符号，且没有任何东西能发现它错——剪枝决策基于错误证据。
- **复核状态**：✅ 计数与调用点由 t13 独立复核；本轮独立 grep 确认「全仓仅两处」。

### LG-16 MAJOR — `compile_error_demo` 魔法目录/文件名改写任意宿主的构建结果
- **=** S7（surfaces）
- **影响面**：任何宿主的构建（契约聚合、静态计划、生成树）。
- **触发条件**：宿主 `src/` 下存在名为 `compile_error_demo` 的模块（或同名 `.rs`）。
- **最小复现**：📖 本轮读码列出四处调用点：
  ```text
  build_method/contracts.rs:78            if node.name == "compile_error_demo" && !include_demo
  build_method/registration_check.rs:87   path.file_name() == Some("compile_error_demo.rs")
  build_method/static_plan.rs:74          if node.name == "compile_error_demo" || …
  build_method/renderer/tree.rs:68-70     writeln!(… "#[cfg(feature = \"compile_error_demo\")]")
  build_method/renderer/pass.rs:169       同上
  ```
- **修复方向**：把这个演示开关变成显式的宿主意愿（环境变量/清单字段/特性名常量），不要用目录名当开关。
- **不修会怎样**：一个合法命名会让该面的契约校验与静态计划**静默跳过**（发布树少一个面）。
- **复核状态**：📖 读码（本轮四处）；t13 复核过同一处。

### LG-17 MAJOR — authoring 的 source 回落链硬编码本仓库夹具目录名
- **=** S8（surfaces）
- **影响面**：发布 crate `run_method` 的 authoring 路径（Studio/MCP 的编辑）对**任意宿主**计算相对源码路径。
- **触发条件**：宿主的源码路径里出现 `control`、`engine`、`trimmed_core`、`compile_error_demo` 之一（例如 `/home/me/engine/src/main.rs`）。
- **最小复现**：📖 本轮读码：
  ```text
  run_method/authoring/parse/parse.rs:31
  let roots = ["compile_error_demo", "control", "engine", "trimmed_core"];
  ```
  随后按第一个命中的组件把路径**截断**成相对路径。
- **修复方向**：回落链改为显式传入的包根/身份基准（`source_layout`），或直接报错而不是猜。
- **不修会怎样**：编辑记录里的 `source` 指向错误的相对路径（写进文件/记忆的声明也随之错）；宿主目录名恰好像本仓夹具时触发。
- **复核状态**：📖 读码（本轮）；t13 复核过同一处。

### LG-18 MAJOR — MIR `jsonl:true` 可给任意可读文件盖上本包快照表头
- **=** LH-04（本路）
- **影响面**：MIR 快照的**来源凭据**（`xirang.mir` 的 delta、`xirang.unified` 的拒绝逻辑都依赖它）。
- **触发条件**：`path` 指向包内任意一个不以 `.jsonl` 结尾的可读文件（文本解析器对非 MIR 文本从不失败）。
- **最小复现**（t8 实测）：
  ```text
  xirang.mir {"path":"notes.txt"}              → functions 0 calls 0 locals 0
  xirang.mir {"path":"notes.txt","jsonl":true} → {"kind":"snapshot","namespace":"probe-host","root":"cc05a41d…"}
  ```
  该行读回后被当作合法快照。
- **修复方向**：零记录时拒绝盖章（或要求文本转储里至少出现一个 MIR 函数头）；把「未标识」与「已标识」的语义分开。
- **不修会怎样**：「这份 artifact 描述哪棵树」由文件内容可证退化为可凭空造；跨树 delta 的判定建立在标签而非事实上。
- **复核状态**：✅ 实测（t8）。

### LG-19 MAJOR — 预览副本跟随目录符号链接走出包根且无环/深度守卫
- **=** LH-06（本路）
- **影响面**：MCP `apply` 预览把包复制到 `temp_dir`；链接可把包外整棵树拖进副本（磁盘/时间），自指链接会复制数十份仍报成功。
- **触发条件**：包内存在目录符号链接（monorepo/共享目录常见）；预览是 `apply` 的默认模式。
- **最小复现**（t8 实测）：
  ```text
  ln -sfn /tmp/nichprobe/outside <fix>/src/outside_link
    → cannot copy /tmp/nichprobe/fix/src/outside_link/secret.txt: Permission denied   ← 包外文件
  ln -sfn . <fix>/src/self  → 预览成功，回复 faces 124，路径出现 self/self/self/…/control/control.rs
  ```
  同桥的读侧对同一条规则是拒绝的（`mcp/index.rs:42-50`、`:66-68`、`:167-174` 的 `is_safe_child`）。
- **修复方向**：`copy_directory` 用 `symlink_metadata` 判断类型，不跟随链接（或 canonicalize 后要求仍在根内）+ `visited`/深度上限。
- **不修会怎样**：一次预览把包外数据读进临时目录；自指链接让预览变成无界复制（实测仍报成功）。
- **复核状态**：✅ 实测（t8）。

### LG-20 MAJOR — 预览副本的临时路径可预测：同名既有目录被删除重建（且失败会留残留）
- **=** LH-07 + BR-19（本路 t8 + bridges t5/t12）
- **影响面**：`/tmp/xirang-mcp-preview-<pid>-<seq>` 上的既有数据；失败时把半份工程源码留在共享临时目录。
- **触发条件**：同机上有人（或另一个实例）在该可预测路径上已有目录；或复制中途失败（不可读文件/磁盘满）。
- **最小复现**（t12 实测）：
  ```text
  预置真实目录 + important.txt 于预测路径（exec 保 pid）
    → pre-created: /tmp/xirang-mcp-preview-415521-0
    → GONE after the run（既有数据被删）
  chmod 000 fixture 内一个文件后预览
    → cannot copy …/unreadable.txt: Permission denied
    → 残留 /tmp/xirang-mcp-preview-416016-0（find 到 .git/config、.git/index、binfile.dat…）
  ```
  **机制更正**：作者设想的「符号链接被顺着写入」被证伪——`mcp/preview.rs:27` 先 `remove_dir_all(destination)`，符号链接本身被删、目标未被写入（t12 实测）。真后果是「删除并重建」与「失败泄漏」。
- **修复方向**：`tempfile`/`O_EXCL` 排他创建 + 0700；`Drop` 守卫保证任何返回路径都清理（含 `copy_package` 自身失败）。
- **不修会怎样**：预测路径上的他人数据被静默删除；失败一次即留下一份工程源码副本（多次即填满 `/tmp`）。
- **复核状态**：✅ 实测（t8 泄漏、t12 删除/泄漏与符号链接证伪）。

### LG-21 MAJOR — `apply edit` 静默忽略 `handle_contracts` / `part_contracts`
- **=** NEW-B1（本路 t12）
- **影响面**：agent 用 `xirang.apply` 的 edit 设置「参与编译检查的契约」；实际得到的是未经检查的标签。
- **触发条件**：`action: edit`（或 rename 之外任何非 add）时在 `fields` 里给这两个键。
- **最小复现**（t12 实测）：
  ```text
  add  + handle_contracts: "crate::Foo::Bar" → 文件得到 handle_traits: ["Bar"] 与 handle_contracts: [crate::Foo::Bar]
  edit + handle_contracts: "crate::Foo::Bar" → 回 action apply / applied …（成功），文件只有 handle_traits: ["Bar"]，无 contract 行
  ```
  根因：`run_method/authoring/operations/face_write.rs:59-79` 的 `EDIT_FIELD_ORDER` 不含这两个字段（它们只在 `CREATE_FIELD_ORDER`），而 `mcp/apply.rs:172-196` 的白名单与工具自述都把它们列为可编辑。
- **修复方向**：把两个字段加进 edit 字段序（并决定空值语义），或在 edit 路径上显式拒绝这两个键并在自述里写明「contract 只在 add 上可设」。
- **不修会怎样**：一次「成功」的编辑把作者要的编译器检查契约降级成未检查标签，回复里没有一个字提到。
- **复核状态**：✅ 实测（t12）。

### LG-22 MAJOR — 非 UTF-8 请求帧结束整个桥
- **=** BR-5（bridges）
- **影响面**：MCP 会话；两份 README 明文承诺「不写 stderr，失败是 stdout 上的错误响应」。
- **触发条件**：客户端发来一行非 UTF-8 字节（JSON-RPC 下本属非法请求）。
- **最小复现**（t12 实测，原始字节）：
  ```text
  exit code: 1  stdout lines: 0
  stderr: xirang-mcp: cannot read stdin: request line is not valid UTF-8
  对照：超长行 exit=0 replies=2；坏 JSON exit=0 replies=2；非对象成员 exit=0 replies=4
  ```
- **修复方向**：把非 UTF-8 归入「坏帧」——写 `-32700` 后 `continue`；真正的传输故障才 `Err`。
- **不修会怎样**：一帧坏字节终止整个会话，且打破 README 的两处承诺；四类畸形帧里只有这一类是致命的。
- **复核状态**：✅ 实测（t12）。

### LG-23 MAJOR — 预览 diff 把每个非 UTF-8 文件报成「新增」，无上限，且每次预览复制 `.git`
- **=** BR-4 + BR-C2（bridges）
- **影响面**：预览回复的正确性与规模；`apply` 的默认路径。
- **触发条件**：包内存在非 UTF-8 文件（本检出实测 36 个，其中 29 个在 `.git/`），或 `.git` 较大（本检出 9.9 MB / 树 47 MB）。
- **最小复现**（t12 实测）：fixture 放两个**两边完全相同**的非 UTF-8 文件后预览：
  ```text
  + .git/index      ← 未被这次编辑碰过
  + binfile.dat     ← 两边完全相同
  + src/probe_add/probe_add.rs   ← 唯一真实变化
  （UTF-8 且未变的 .git/config、plain.txt 不出现 —— 触发条件正是「读不出 ⇒ 当新增」）
  ```
  `.git/config` 被逐字节比较而未变，反证副本里含 `.git`。
- **修复方向**：两侧原始字节相等即 `continue`（或 `fs::read` 比字节）；跳过长列表里的 `.git` 等 VCS/工具目录；给 diff 加行数上限并声明。
- **不修会怎样**：agent 以为一批文件会变（假阳性）；回复无上限；每次预览复制一份 `.git`。
- **复核状态**：✅ 实测（t12）；作者原评 MAJOR，维持。

### LG-24 MAJOR — `lint` 把 `#![deny(warnings)]` 当作「带着 missing_docs」
- **=** G-01（gates）
- **影响面**：`missing_docs` 保护（九个 crate 的公开面文档）可被静默移除而三道防线全绿。
- **触发条件**：把某个 crate 根的 `#![warn(missing_docs)]` 换成 `#![deny(warnings)]`。
- **最小复现**（本轮 rustc 复现，独立于 t14）：
  ```text
  #![deny(warnings)] + pub fn undocumented() {}   → rustc exit 0，零输出
  （无属性）                                        → exit 0
  #![warn(missing_docs)] + 同内容                   → warning: missing documentation for a function
  ```
  门禁那一条判定在 `conventions/src/lint.rs:225-227`（`trimmed == "#![deny(warnings)]"` → true）。
- **修复方向**：删掉这条特例，只接受含 `missing_docs` 的属性文本。
- **不修会怎样**：`lint` 门禁绿、rustc 无输出、`clippy -D warnings` 无可升级的 lint——保护消失而没有任何信号。
- **复核状态**：✅ t14（夹具 + rustc）+ 本轮独立 rustc 复现。**自我更正**：我第一版探针文件里同时写了 `#![warn(missing_docs)]`，于是 `deny(warnings)` 看起来「有效」——那是我的探针 bug；去掉那一行后结论与作者一致（这条恰好也是 §5.2 的实例：判定必须落在正确位置）。

### LG-25 MAJOR — 发布工作流的 tag 守卫用子串黑名单判否，等价否定写法整类放行
- **=** G-02（gates）
- **影响面**：持有发布 token 的路径；`startsWith(github.ref, 'refs/tags/') == false` 语义上是「只在非 tag 上跑」，即每个分支 push 都可能发布。
- **触发条件**：`release.yml` 的发布步骤 `if:` 写成 `… && startsWith(github.ref, 'refs/tags/') == false`。
- **最小复现**：t14 复核（夹具 + text 级门禁，比作者更宽）；判据是 `conventions/src/release_workflow.rs:96-111` 的四条子串判定全部为「看起来没问题」。
- **修复方向**：条件判定从子串黑名单改成「只允许肯定式形状」的白名单；至少拒绝 TAG_GUARD 后紧跟 `== false`/`= false`/`== 0`。
- **不修会怎样**：门禁说发布是 tag 守卫的，实际条件可被一个后缀绕过——而这是唯一按 token 发布的入口。
- **复核状态**：🔍 复核（t14）。

### LG-26 MAJOR — 持有 `CARGO_REGISTRY_TOKEN` 的 job 的 action pin 只由注释承诺
- **=** G-13（gates）
- **影响面**：供应链（可发布的 token 环境里运行第三方 action）。
- **触发条件**：有人改动该 job 的 `uses:`（无门禁会红）。
- **最小复现**：t14 复核（夹具：把 release.yml 副本里的 pin 改成浮动标签，门禁仍绿）。
- **修复方向**：把 pin 规则加进 `conventions`（每个持有 secret 的 job 的 `uses:` 必须含 40 位 SHA）。
- **不修会怎样**：一次看似无害的 action 升级就能把 token 暴露给移动目标。
- **复核状态**：🔍 复核（t14）。

### LG-27 MAJOR — `merge_authored` 静默忽略作者改过的 `runtime_checks`
- **=** K-02（kernel）
- **影响面**：热重载语义（Studio/MCP 编辑后重载）：文件里写着新值，运行期校验仍是旧值。
- **触发条件**：作者侧快照含 `runtime_checks`（与编译期不同）时走 `merge_authored`。
- **最小复现**：t9 探针（`K02.merged.runtime_checks = [FiniteNumber]`，而作者侧给的是 `[NonEmptyText]`）；唯一调用点 `run_method/authoring/operations/operations.rs:352`。
- **修复方向**：与 `flow` 同款「作者侧非空即应用」；或在函数文档里写明「运行期校验以编译期为准，文件编辑需重编译生效」并据此拒绝编辑入口。
- **不修会怎样**：编辑被静默吃掉，文件与行为不一致（校验强度可能被改弱或改强而无人知）。
- **复核状态**：🔍 复核（t9）。

### LG-28 MAJOR — 畸形 `requires` 条目在校验器/快照解析器/两个桥工具之间三种口径
- **=** K-06（kernel）+ BR-10（bridges）+ S12 同族（静默降级）
- **影响面**：能力依赖边（`requirements`）——注册校验、快照、`impact` 的爆炸半径、`converge` 的裁决。
- **触发条件**：`requires` 里出现不带 `=>` 的裸能力名（内核在创作期拒绝，但手写/旧文件可含）。
- **最小复现**：📖 本轮读码三处对照：
  ```text
  core/src/registry_core/authoring/parse/rules.rs:110-118  parse_requirements_owned：split_once("=>")? → 静默丢
  core/src/registry_core/authoring/parse/rules.rs:120 起   parse_requirements（校验器）：拒绝该形状
  mcp/impact.rs:248                                    if let Some(…) = split_once("=>") → 不 push（静默丢边）
  mcp/converge.rs:197-205                              同一输入 push provider:"?" 的 UNANSWERED 裁决
  ```
- **修复方向**：一处解析、一处判定；或「两个工具都报 UNANSWERED」，或「两边都拒绝该输入」。
- **不修会怎样**：同一份 `requires` 文本被三处解释；`impact` 少报一条依赖边 ⇒ 爆炸半径假阴性。
- **复核状态**：✅ t9（K-06）+ t12（BR-10 两侧解析器读码对照）；本轮补 rules.rs 原文。

### LG-29 MAJOR — 模块级 `cfg` 一律跳过，特性已开启也照跳
- **=** K-08（kernel）
- **影响面**：发布态静态计划的注册面集合（该特性下本该存在的面缺失）。
- **触发条件**：face 模块被 `#[cfg(feature = "x")]` 门控且本次构建开启了 `x`。
- **最小复现**：t9 复核（三种形态逐一实测）。
- **修复方向**：`graft_entries` 读同一份 feature 求值结果（内核已有 `face_cfg_enabled`），而不是见到 `cfg` 就跳。
- **不修会怎样**：开启的特性对应的槽位在计划里消失（发布树少面），而 `check` 是绿的。
- **复核状态**：🔍 复核（t9）。

### LG-30 MAJOR — trait 标签派生遇带逗号的泛型实参失败 → 构建期误报「缺 trait」
- **=** K-09（kernel）
- **影响面**：构建期诊断正确性（假失败）。
- **触发条件**：契约路径的泛型实参里含逗号（如 `Foo<A, B>`）。
- **最小复现**：t9 复核（t9 §2.9）。
- **修复方向**：用语法感知的路径解析（front end 已有），而不是按逗号切分。
- **不修会怎样**：合法声明被判成「缺 trait」，作者被迫改动正确的代码来让门禁闭嘴。
- **复核状态**：🔍 复核（t9）。

---

## 2. 函数级谬误与假实现

（§1 里的 LG-15/16/17/21/28 同时也是假实现，不重复；本节列其余。）

| id | 严重度 | 影响面 | 触发条件 | 最小复现 | 修复方向 | 不修会怎样 | 复核状态 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| LG-31 **=** LH-05 | MAJOR | 长寿命 MCP/Studio 进程的内存（外部输入路径） | 每次喂进「词表不同」的 trace artifact | t8 实测：A 组每轮 +6.5 MB / 10 万串且不回落，B 组平 | 驻留表按 artifact 生命周期回收（arena/Arc），或加上限与淘汰 | 会话内存单调增长，永不释放 | ✅ t8 |
| LG-32 **=** LH-08 = BR-20 = BR-C1 | MAJOR | 进程插件适配器的答复丢弃与错误种类 | 插件写完整帧后不退出 | t8/t12 实测：`Err(Timeout) after 303ms`（帧已收到） | 把「帧完整」变成完成条件，或改注释并把丢弃语义写进 `ProcessLimits::timeout` | 已送达的答案被丢，报错误的失败种类；注释还声称已修 | ✅ t8/t12 |
| LG-33 **=** K-03 | MINOR（原 MAJOR） | 错误文案 | 把一个有非空子注册机的面改成叶子 | t9 证伪机制：两条路径同判 `Err`；`UnknownTarget` 兜底不可达 | 只改文案（说明是「不能降级非空子注册机」） | 作者去改身份/父级而不是子注册机 | 🔍 t9 |
| LG-34 **=** V-03 | MINOR（原「数据丢失」） | Studio 表单显示 | 文件在索引后被删/改名/改权限 | t12 实测：把两个 contract 键指名成空后，文件里两条契约行仍在 | 表单区分「读失败」与「没写」，或显示为未知 | 用户看到空契约行（显示歧义），**不会**丢数据 | ✅ t12 证伪 |
| LG-35 **=** S13 | MINOR | `render_call_report` 的名字与文档 | 任何调用 | 📖 读码：实现恒为空树 | 改名或实现 | 读者按名字以为拿到调用树 | 🔍 t13 |
| LG-36 **=** S15 | MINOR | `edit()` 里的恒等分支 | 任何编辑 | 📖 读码 `run_method/authoring/manifest/face/face.rs:144-148` | 删除 | 死逻辑，误导读者 | 🔍 t13 |
| LG-37 **=** K-10/K-17/S11 | MINOR | 文本启发式解析（注册规则 / `registration_kinds` / 规则方法） | 源码注释或字符串里出现 `kind:`、`rule_method(` 等字样 | 📖 读码三处（内核 K-10/K-17、S11） | 走语法解析（内核已有 front end） | 注释里的伪子句被当真；或漏收 | 🔍 t9/t13 |
| LG-38 **=** S6 | MINOR | 字段集合的真值 | 编辑一个「能改但不能写回」的字段 | 🔍 t13 复核 | 让声明集合 = 写出集合（一处导出） | 编辑后文件里看不到变化 | 🔍 t13 |
| LG-39 **=** K-18 | MINOR | 静态 vs 运行时对重复身份的判定 | 两个面同身份 | 📖 读码（静态拓扑不查、运行时查） | 静态侧补同一条 | 同一棵树两条路不同判 | 🔍 t9 |
| LG-40 **=** K-13 + X-1 | MINOR | 插件锁读取的字段语义 | 10 字段记录的 8/9/10 列为空串 | 本轮实测：`X1.ten_fields_empty = Ok signature=None fingerprint=None revocation=None`（对照 `ten_fields_full = Some/Some/Some`） | 空的身份字段按「明确缺失」报错，或把 7/10 字段两种格式分开 | 记录看起来带身份三元组，实际三个都不在 | ✅ 本轮 |
| LG-41 **=** K-11/K-12 | MINOR | 官方通道的信任判定 | 官方工件自述指纹；或 `require_digest` 与「必须已签名」耦合 | t9 复核（K-11 部分证实：缺陷成立、作者引用的测试说反了；K-12 成立） | 指纹必须来自信任根；把两条拒绝解耦 | 官方通道的密钥判定可被工件自述影响 | 🔍 t9 |
| LG-42 **=** S16 | MINOR | 库内 panic 面 | 见 S16 清单（16 处） | 🔍 t13 逐条 | 逐条记账，需要时改 `Result` | 已知的 panic 面没有台账 | 🔍 t13 |
| LG-43 **=** BR-3 | MINOR | `xirang.callgraph` 的参数契约 | 客户端按 schema 校验参数 | t12 实测：schema 无 `limit`，回复却叫调用方 raise 它；`limit` 确实改变答案长度（1/5/10/50 → 1/5/10/39） | 在 catalog 补声明，或删掉 `limit` 读取 | 提示不可执行；未声明的键静默改变答案 | ✅ t12 |
| LG-44 **=** NEW-B2 + BR-13 | MINOR | CLI `--help` 语义 | 七个子命令各带 `--help`；裸调 | t12 实测退出码矩阵：裸调/`--help` → 0；`check|explain|grafts|snippets --help` → 1；`studio --help` → 0 + usage；`new --help` → 1；`build --help` → 0（先跑校验再交给 cargo）；**`mcp --help` → 0 且两流 0 字节** | 共享 `help()`；裸调走 stderr + 非零 | 同一旗标五种语义；`mcp --help` 静默变成服务器 | ✅ t12 |
| LG-45 **=** BR-14 | MINOR | `new` 的依赖来源 | `--path` 与 `--git` 同时给 | t12 实测：exit 0、项目已建、Cargo.toml 只有 `path=`、无 `git=` | 两来源同时给即拒绝（与 `build` 同款） | 静默二选一，与 USAGE 的 `|` 矛盾 | ✅ t12 |
| LG-46 **=** BR-16 | MINOR | 进程适配器的 deadline | `ProcessLimits { timeout: Duration::MAX }` | t12 实测：panic（std time.rs:429），exit 101 | `checked_add` → `HostError::Limit` | 宿主配置错误在库内 panic | ✅ t12 |
| LG-47 **=** BR-18 | MINOR | wasm 懒激活的重试与吞吐 | 失败的激活之后再次 `activate_pending` | t12 实测：`#1 → Err(ABI)`、`#2 → Ok(false)`（待定已被吃掉） | 失败时把候选放回，或文档写明「失败即作废」 | 瞬时失败只能靠重装恢复；持锁编译拖住同槽并发 | ✅ t12 |
| LG-48 **=** S14 | MINOR | 失败回滚的可靠性 | create/operations/migration 的回滚链 | 🔍 t13 读码（全 `let _ =`） | 至少聚合成一条诊断 | 回滚失败无人知 | 🔍 t13 |
| LG-49 **=** S12 | MINOR | trace artifact 读取 | 记录顺序与契约顺序不同 | 🔍 t13 复核（只降级不报错） | 顺序违例报错或明确声明 | 不一致的 artifact 被当作完整 | 🔍 t13 |
| LG-50 **=** S-07/S-24 | MINOR | Studio 错误吞掉 | 见 studio 报告 | 🔍 t11 复核 | 把失败并入可见状态 | 用户看不到失败原因 | 🔍 t11 |
| LG-51 **=** LH-10b | MINOR | `xirang.mir` 的存在性 oracle | 传根外路径 | t8 实测：存在 → 「在配置根内」；不存在 → 「不可读文件」 | 根外统一回「不在根内」 | 泄露宿主路径存在性（低危） | ✅ t8 |
| LG-52 **=** LH-10a | MINOR | 坏帧的错误上下文 | 插件写半帧后 exit 0 | t8 实测：`Io("failed to fill whole buffer")` | 包装成带操作名/「帧」字样的错误 | 调用方拿不到任何上下文 | ✅ t8 |
| LG-53 **=** G-06/G-23 | MINOR | 门禁豁免范围 | 文件名以 `audit`/`design` 开头的任何文件（含审计报告） | 队长 09-28 广播（我本人也撞到：报告改名即红/绿） | 豁免按目录或显式清单，而不是文件名前缀 | 报告被当活文档扫描；门禁红/绿取决于文件名 | 🔍 t14 |
| LG-54 **=** G-03/G-04/G-07/G-08/G-05/G-17/G-18/G-19/G-20/G-22/G-24/G-25/G-26/G-27 | MINOR | 门禁的假阴性/假阳性/文档漂移 | 见 gates 报告逐条 | 🔍 t14 | 见逐条 | 门禁的覆盖面与其自述不符 | 🔍 t14 |

---

## 3. 安全

| id | 严重度 | 影响面 | 触发条件 | 最小复现 | 修复方向 | 不修会怎样 | 复核状态 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| LG-01 | CRITICAL | 身份（本仓安全基元） | 见 §1 | ✅ t8 | 键加命名空间 + 读侧复核 | 跨包身份污染 | ✅ |
| LG-02 | CRITICAL | 外部依赖门禁 | 见 §1 | ✅ 本轮/t9 | 承载两张列表或拒绝 | 门禁被静默放宽、不可逆 | ✅ |
| LG-03 | CRITICAL | 插件准入 | 见 §1 | 🔍 t11 | 写盘走内核解析/校验 | 写出宿主读不了的锁 | 🔍 |
| LG-05 | MAJOR（可达 abort） | 宿主进程 | 见 §1（301 B / 256 KiB 栈即 abort） | ✅ 本轮 exit 134 | 接 `guard_nesting` | 输入即可打死进程 | ✅ |
| LG-19 | MAJOR | 包外数据 / 磁盘 | 见 §1 | ✅ t8 | 不跟随链接 + 环/深度守卫 | 包外树被读进副本 | ✅ |
| LG-20 | MAJOR | `/tmp` 上的既有数据 | 见 §1 | ✅ t12 | 排他创建 + `Drop` 守卫 | 同名目录被删；失败留源码副本 | ✅ |
| LG-23 | MAJOR | 预览回复正确性/规模 | 见 §1 | ✅ t12 | 字节比较 + 跳过 `.git` + 上限 | 假阳性 + 无界回复 | ✅ |
| LG-24 | MAJOR | 文档一致性保护 | 见 §1 | ✅ t14/本轮 | 删特例 | 保护可被静默移除 | ✅ |
| LG-25 | MAJOR | 发布路径 | 见 §1 | 🔍 t14 | 肯定式白名单 | 分支 push 可能发布 | 🔍 |
| LG-26 | MAJOR | 供应链（token 环境） | 见 §1 | 🔍 t14 | pin 加门禁 | action 可被浮动 | 🔍 |
| LG-41 | MINOR | 官方通道信任判定 | 见 §2 | 🔍 t9 | 指纹来自信任根 | 自述可影响判定 | 🔍 |
| LG-04 | MAJOR | 锁 schema 对账 | 见 §1 | ✅ 本轮 | 表头识别 + 位置 | 门禁静默失效 | ✅ |
| LG-03 | CRITICAL | 见上 | — | — | — | — | — |

**明确“查了、干净”的安全面**：`core/src` 无 I/O/环境/进程/时间调用（t2 F 铁律 0 命中；t9 复核时另发现 2 处文档注释命中，不影响结论）；wasm 侧全部限额接到实现（t8 读码 + 既有钉子）；进程插件的输出/输入/超时/环境各项限额生效（t8/t12 实测）；写路径 `module`/`registry_rule_path` 的根约束严于预览复制（t8 实测 + t12 读码）；插件锁解析拒绝未知 schema/重复身份/空字段（`core/src/registry_core/plugin/catalog/catalog.rs:180-207`，本轮实测确认 7/10 字段与重复身份两条；**其门禁的开启条件有问题，见 LG-04**）。

---

## 4. 可靠性

| id | 严重度 | 影响面 | 触发条件 | 最小复现 | 修复方向 | 不修会怎样 | 复核状态 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| LG-31 | MAJOR | 进程内存 | 见 §2 | ✅ t8（+6.5 MB/10 万串） | 生命周期回收 | 单调增长 | ✅ |
| LG-32 | MAJOR | 插件答复 | 见 §2 | ✅ t8/t12 | 或收窄注释 | 丢答案 + 错误种类 | ✅ |
| LG-22 | MAJOR | 桥会话 | 见 §1 | ✅ t12 | 归入坏帧 | 一帧终止会话 | ✅ |
| LG-13 | MAJOR | 桥的跨工具一致 | 见 §1 | ✅ t12 | 统一命名空间 | 每个面被报成变了身份 | ✅ |
| LG-11 | MAJOR | 只读视图 | 见 §1 | ✅ t8 | 报不可读面 | 少报 + 路径被改写 | ✅ |
| LG-12 | MAJOR | graft 记录健康度 | 见 §1 | ✅ t8/t12 | 两维结论 | 两工具互相矛盾 | ✅ |
| LG-09 | MAJOR | 构建证据 | 见 §1 | 🔍 t13 | 载荷先写、指纹最后 | 混代产物被当 current | 🔍 |
| LG-08 | MAJOR | 路径真值 | 见 §1 | 🔍 t13 | 递归解析 | 视图与运行期不同判 | 🔍 |
| LG-10 | MAJOR | 迁移数据 | 见 §1 | 🔍 t13 | 边界匹配 | 兄弟模块被改写 | 🔍 |
| LG-46 | MINOR | 库内 panic | 见 §2 | ✅ t12（exit 101） | `checked_add` | 配置错误即 panic | ✅ |
| LG-47 | MINOR | 懒激活重试 | 见 §2 | ✅ t12（`#2 → Ok(false)`） | 放回候选或写文档 | 失败只能重装 | ✅ |
| LG-48/49/50 | MINOR | 回滚/顺序/错误吞掉 | 见 §2 | 🔍 t13/t11 | 见逐条 | 失败无人知 | 🔍 |
| LG-53 | MINOR | 门禁红/绿取决于文件名 | 见 §2 | ✅ 现场 | 豁免按目录 | 报告把门禁弄红 | ✅ |

---

## 5. 两种形状专节（本项目历史重灾区）

### 5.1 形状 ①：自我描述与行为不一致

| # | 位置 | 自述 | 行为 | 严重度 / 状态 |
| --- | --- | --- | --- | --- |
| 1 | `run_method/macros/face_registration.rs:9-10` vs `:168-171` | 收集器决定进哪个链接段 | `linked` 什么都不提交 | MAJOR（LG-14）📖 |
| 2 | `mcp/apply.rs:172-196` + 工具自述 vs `run_method/authoring/operations/face_write.rs:59-79` | `handle_contracts`/`part_contracts` 可编辑 | edit 路径不写这两个字段 | MAJOR（LG-21）✅ |
| 3 | `mcp/tools.rs:139-157` vs `mcp/diff.rs:143-239` | 记录只有 ok/stale/re-identified + unreadable | 还有 `undeclared` 桶 | MINOR（LG-12 半边 / BR-8）✅ |
| 4 | `mcp/diff.rs:34`（模块文档） | 两种比较共用 added/gone/re-identified 词汇 | `records:true` 用 ok/undeclared/stale/re-identified | MINOR（LH-09b）✅ |
| 5 | `mcp/README.md:47`、`README.zh-CN.md:35` | 现在时承诺 `unmatched` 桶 | 该桶已删 | MAJOR（BR-1）✅ |
| 6 | `mcp/README.md:164-166` | 「树 diff 仍待做」 | 同文件 120 行前就描述 `xirang.diff` | MAJOR（BR-2）✅ |
| 7 | `mcp/tools.rs:77-80` vs `mcp/callgraph.rs:41-44/108-113` | schema 只有 function/path/root | 实现读 `limit` 并叫调用方 raise 它 | MINOR（LG-43）✅ |
| 8 | `plugin-host/src/process.rs:341-354`、`plugin-host/src/process/child.rs:119-127` | 已送达的答案不会再被报成超时 | 仍会（LG-32） | MAJOR（BR-C1）✅ |
| 9 | `mcp/preview.rs:14-20` | `target/` 是唯一可能很大的目录 | `.git` 也被复制/列举 | MAJOR（BR-C2）✅ |
| 10 | `mcp/mir.rs:20-29` | 快照表头是来源凭据 | 可凭空盖出 | MAJOR（LG-18）✅ |
| 11 | `core/src/registry_core/plugin/catalog/catalog.rs` 自述「unknown schemas 一律拒绝」 | 表头拼错即不拒绝 | MAJOR（LG-04）✅ |
| 12 | `core/src/registry_core/authoring/parse/admission.rs` 模块自述「两种拼法同一个门禁」 | 构造函数形式丢 deny | MAJOR（C-01，与 LG-02 同处）🔍 |
| 13 | `core/src/registry_core/tree/ports` 自述「绝不静默解析」 | `resolve_path` 取第一个 | MAJOR（C-03）🔍 |
| 14 | `CallSite` 的 doc 写成一条边（内核 `core/src/registry_core/declaration/call_evidence.rs`） | 类型不是边 | MAJOR（C-02）🔍 |
| 15 | `studio/studio/app/writers.rs:1-22` | 自称拥有全部写入 | 实际零写入 | MAJOR（C-02 studio）🔍 |
| 16 | `studio/studio/app/navigation.rs:165-171` | `retreat_graph_focus` 注释称「反向走」 | 与 `advance` 逐字节相同 | MAJOR（C-01 studio）🔍 |
| 17 | `studio/studio/app/search_queries.rs:9-17` | 「已去重」 | `dedup_by` 只消相邻 | MAJOR（C-03 studio）🔍 |
| 18 | `studio/studio/app/support.rs:1-2` | 只声明几何 | 含项目解析与 cargo 子进程 | MAJOR（C-04 studio）🔍 |
| 19 | `tools/xirang-package-audit` 头部注释 | 描述一张表 | 该表已不存在 | MAJOR（G-21）🔍 |
| 20 | `conventions/src/lint.rs:225-227` | 带 `#![deny(warnings)]` 即「带着 missing_docs」 | 该 lint 仍 allow | MAJOR（LG-24）✅ |
| 21 | `conventions/src/release_workflow.rs` 文档自述已堵住 `!startsWith` 绕过 | `== false` 等价写法放行 | MAJOR（LG-25）🔍 |
| 22 | `.github/workflows/release.yml` 注释承诺 action pin | 无门禁 | MAJOR（LG-26）🔍 |
| 23 | `build_method/face_view.rs:222-231` | 「绝不打印运行期从未有过的逻辑路径」 | 解析失败时打印 `root/button` | MAJOR（LG-11）✅ |
| 24 | `run_method/runtime/trace/artifact/parse.rs:289-293` | 泄漏「按本 artifact 词表有界」 | 界是进程级历次并集 | MAJOR（LG-31）✅ |
| 25 | `mcp/index.rs:42-50/66-68`（读侧拒绝链接）vs `preview.rs`（写侧跟随） | 同一条边界两个标准 | MAJOR（LG-19）✅ |

**形状 ① 的共性**：这些自述都不是错别字，而是**承诺了一个更强的性质**（决定链接段、可编辑、不再丢答案、来源可证、门禁已开）。修掉这一类的收益不只是文档：其中 8 条的行为本身也错（LG-04/05/14/18/21/24/25/31）。建议在重构方案里把「文档承诺 ↔ 行为」做成可执行的对照表（同 `doc_anchors` 的思路），而不是只靠人工复核。

### 5.2 形状 ②：判定落在「出现了某个东西」而不是「它在语法位置上生效」

| # | 判定 | 位置 | 落空方式 | 后果 | 状态 |
| --- | --- | --- | --- | --- | --- |
| 1 | 锁 schema 门禁「文件里有 `# xirang-schema=` 那一行」 | `core/src/registry_core/plugin/catalog/catalog.rs:131-137` | 写成 `# xirang-schema v9`（少 `=`）→ 当注释跳过 | 门禁静默关闭 | ✅ 本轮 |
| 2 | 同上：判定写在**记录循环内** | 同上 `:138-145` | 无记录 / 表头在记录之后 → 检查不跑 | 门禁对空锁与乱序锁不生效 | ✅ 本轮 |
| 3 | 剪枝符号「这一行含某个魔法标识符」 | `build_method/manifests.rs:200-213` | 中文/别的面/注释里出现同样字样 | 写出错误符号（含硬编码 `Button::…`） | ✅ t13/本轮 |
| 4 | 注册面「这一行含 `kind:`」 | `core/src/registry_core/source/source.rs:434-465` | 注释/字符串里出现 | 过收或漏收 | 🔍 t9 |
| 5 | 注册规则「整文件文本里出现 `rule_method(`」 | 内核 K-10 | 注释里的伪子句 | 读到不存在的规则 | 🔍 t9 |
| 6 | 规则方法「第一处调用」 | S11 | 注释掉的调用在前 | 取到错的字符串 | 🔍 t13 |
| 7 | 「任何以 `_object` 结尾的用户宏都是注册面」 | K-23 | 同名非注册宏 | 错误构建诊断 | 🔍 t9 |
| 8 | 「入口文件**子串**含某段文本即已声明」 | S-08 studio | 注释掉的 `use` | 「已声明」为假 | 🔍 t11 |
| 9 | 门禁豁免「文件名以 audit/design 开头」 | `conventions/src/doc_blocks.rs:69` | 报告改个名即被当活文档 | 门禁红/绿随文件名 | ✅ 现场 |
| 10 | `missing_docs` 保护「文件里有 `#![deny(warnings)]` 字样」 | `conventions/src/lint.rs:225-227` | 该属性对 allow 级 lint 无效 | 保护静默移除 | ✅ t14/本轮 |
| 11 | 上传守卫「条件串含 TAG_GUARD 且不含三个否定记号」 | `conventions/src/release_workflow.rs:96-111` | `… == false` 等价否定 | 条件被读成正确 | 🔍 t14 |
| 12 | 「`compile_error_demo` 这个目录名」 | 四处调用点（LG-16） | 宿主合法命名撞上 | 构建结果被静默改写 | 📖 本轮 |
| 13 | 「相对路径里出现过 `control`/`engine`/…」 | `run_method/authoring/parse/parse.rs:31` | 宿主同名目录 | source 被截断 | 📖 本轮 |
| 14 | 静态拓扑「身份出现过」但不查重复 | K-18 | 两个面同身份 | 静态/运行时不同判 | 🔍 t9 |
| 15 | 「路径键存在即唯一」 | K-05 | 兄弟同名 `registry_name` | 三处静默碰撞 | 🔍 t9 |
| 16 | 符号扫描「字面 `#[cfg(test)` / `mod x;`」 | G-04 | 带空格的写法 | size 门禁漏判 | 🔍 t14 |
| 17 | markdown 围栏「第一个 ``` 配对下一个 ``` 」 | G-03 | 4 反引号/`~~~` 内嵌 | 整段内容漏检 | 🔍 t14 |
| 18 | `doc_anchors` 属性与声明之间 `///` | G-08 | 属性在 `///` 之前 | 把声明误判成文档 | 🔍 t14 |

**形状 ② 的共性与修法方向**：这些判定都在问「文本里有没有这个东西」，而正确的问法是「它在不在生效的位置」（解析后的语法节点、正确的行序、正确的字段序）。**单一修法**：凡门禁/校验/契约，取**解析结果或结构化输入**（内核已有 front end、`face_field` 常量、`FACE_FIELD_ORDER`、`guard_nesting`），文本级扫描只允许出现在「报告」而不允许出现在「判定」。建议把它们排成重构方案的一个独立批次（收益：一次消掉 §5.1 里约一半的实例）。

---

## 6. 附：已降级 / 已证伪 / 数字更正 / 未证实

**已证伪（从总账移除或降级）**
- **v7「debug_method 幻影依赖」**（t7 转达，boundary-architect）：`::xirang_debug_method::submit!` 只在 `collector: debug` 且 `#[cfg(debug_assertions)]` 下发出；全树唯一生产方是测试 `debug_method/tests/collector_integration.rs:7`；两个示例走 `development` 空 arm。只依赖 run-method 的宿主显式 opt-in 时 debug 构建才 E0433、release 通过。**→ 出厂路径上不成立**（t8 LH-11 实测）。残留 MINOR：该 switch 无文档。
- **K-03 的机制**：两条路径同判 `Err`，`UnknownTarget` 兜底不可达；只剩错误文案错指原因 → **降 MINOR**（t9）。
- **V-03 的「静默抹掉 contract 字段」**：`EDIT_FIELD_ORDER` 不含这两个字段，端到端探针后契约行仍在 → **降为「表单显示歧义」（MINOR）**，真缺陷改写为 LG-21（t12）。
- **BR-19 的「符号链接被顺着写入」**：`mcp/preview.rs:27` 先 `remove_dir_all` 删的是链接本身，实测受害目录未被写入 → 机制证伪，改成「删除并重建 + 失败泄漏」（t12）。
- **S1 之前的「不可读面文件静默丢面」**（t3 自证伪）：构建侧会报错——与 LG-11 不冲突，LG-11 讲的是**只读视图**那一边。

**数字/引用更正（沿用复核后的口径，勿照抄原 lane 数字）**
- K-03 → MINOR（见上）；K-11 作者引用的测试断言的是 `Err(MissingOfficialKey)`，「自述指纹那条路」**无测试覆盖**；K-20/C-04 的计数实为 4 处与 10 处（作者写 3/9，且漏了 `reports.rs`）；「F 铁律 0 命中」实为 2 处**文档注释**命中（结论仍成立）。
- K-14 的期望值是 3/当前 2（作者写反）；K-23 作者给的探针文本本身会得到 `None`（要按复核后的写法）。
- G-20：`conventions/src/mounting.rs:10` 的裸 `mod` 实为 **33**（作者初稿 335 是「以 mod 开头的行数」，已自更正）。
- BR-19 的 `.git` 9.9 MB / 树 47 MB（t6/t12 实测一致）；BR-13 是**五种**下场而非四种。

**仍未独立复核（⏳ 视为未证实，不写进结论）**
- K-21（两份 trait 派生）、C-09（字段帮助文本讲错机制）：t9 明确未抽到。
- BR-C6/C7、S9…（凡本报告里标 📖/🔍 但只有单一来源的条目）：单一来源已注明；需要第二来源的已标 `unverified`。
- X-3（`mir_line` 第三种语义）是 K-10 的加强，不单列。

---

## 7. 查了、干净（本轮走通 / 探过的流）

**我自己跑出来的探针（本轮新增，全部在 `/tmp`）**
- `/tmp/nichverify/synthprobe`（dep：`xirang-core`，features `syntax`）：K-01 读→写往返；X-2 表头拼错 vs 正确；K-07 三形态（空锁/表头在后/仅注释）；X-1 十字段空身份。
- `/tmp/nichverify/synprobe`（dep：`syn`）：`syn::parse_str::<syn::Path>` 在 256 KiB 栈下 100 层（301 B）abort，exit 134——复现 LG-05。
- `/tmp/nichverify/`：MCP 原始字节驱动 + `tools/list` + fixture 端到端（t12 的 BR-1…BR-6/19、LG-21/22/23 都在这里跑过）；`g01_*.rs` 三个 rustc 探针（LG-24）。
- `/tmp/nichprobe/`：`idprobe`（LG-01，冷/热 + 六序）、`libleak`（LG-31，RSS 两组对照）、`procprobe`（LG-32/LG-46）、`pkg` fixture。

**走通且干净的流（本轮复核或复用的独立结论）**
1. MCP 读侧路径约束（`../../etc/passwd`、绝对路径、非 `.rs` 全部被拒；`is_safe_child` 组件级比较）——✅ t8/t12。
2. graft 计划文档解析（版本/重复键/未知键/缺键/`target_path` 空段/选择器穿越）——✅ t8 读码 + 内核自带钉子；`plan_rows` 的不可读项计数——✅ t8/t12。
3. MIR JSONL 严格解析与 delta 方向——✅ t8 读码 + 内核钉子。
4. wasm 全部限额与进程插件限额（读码 + 既有钉子 + 本轮 `wasmprobe`）——✅ t8/t12。
5. 创作写路径的根约束（`validate_name`、`..` 组件拒绝、绝对路径 canonicalize + 生成标记）——✅ t8/t12。
6. 插件锁解析拒绝未知 schema/重复身份/空字段——✅ 本轮实测（**注意门禁的开启条件另有问题，LG-04**）。
7. 「17 个 MCP 工具两处都在」（活体分派逐个调）——✅ t12。
8. 内核 F 铁律（`core/src` 零 I/O/环境/进程/时间）——✅ t2，t9 复核（2 处文档注释命中不影响）。
9. `cargo test -p xirang-conventions --offline` = 99 passed（t4/t6 复核口径）——✅（本轮未复跑，引用复核报告）。

**只读了代码、没有探针的（如实标注）**：LG-03（studio 写盘闸门）、LG-06/07/08/09/10/27/29/30（复核报告给了实测/复核，本轮未重跑）、LG-14/16/17/36/37/38/39/41/42/48/49/50、§5.2 里标 🔍/📖 的判定位置。凡标 `unverified` 的条目不应在下一轮重构中被当成已证实前提。

**本轮自己的探针 bug（记录以备核对）**：LG-24 的第一版 `lintprobe.rs` 同时写了 `#![warn(missing_docs)]`，使 `deny(warnings)` 看起来有效；去掉后与 t14 一致。这正是 §5.2 的形状——判定必须落在正确位置，探针也一样。
