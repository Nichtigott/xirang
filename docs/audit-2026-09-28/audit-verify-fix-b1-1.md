# 独立验证 B1-1（第二条装置）：admission 读回不再丢 deny

- 任务：t55（attempt 2，studio-auditor）；复核对象：**t43**（kernel-auditor）对 `parse_admission_expression` 的修复。
- **这是 B1-1 的第二条独立验证**；第一条是 **t51**（gates-auditor）。两条的装置不同：t51 走内核侧测试与命令链，本条用我在 t9 就装在树外的 `/tmp/nk-probe` 探针，并额外断言**语义**（重建门禁后 deny 仍是否决），而不是只断言文本里还有 deny。
- 被验证状态（钉住）：`core/src/registry_core/authoring/parse/admission.rs` sha256 `c14af278b8e1fcc1165d89b6364b38cef00b7a265e8b7730de9e97cda77a69cd`（t43 的修 + t50 格式化之后）；工作树 HEAD `a524956` + 未提交改动。
- 本轮只写本报告；临时源码变异只存在于**我自己的 /tmp 备份 + 一次还原窗口**中，逐字节还原（见 §3）；未 commit、未跑任何 git checkout/restore/stash、未动他人产物。

---

## 1. 装置（与作者不同）

- 树外探针工程 `/tmp/nk-probe`（`[workspace]` 独立，target 与产物全在 `/tmp`；依赖只**读**检出：`xirang = { package = "xirang-core", path = "…/core", features = ["syntax"] }`）。
- 探针 `B11`（我自己的 18 项检查，**不调用作者的测试**）。作者的钉子只作为“变异灵敏度”的对照单独跑，见 §3。
- 语义断言的做法：把读回的 `OwnedAdmission` 两张列表用 `Box::leak` 还原成运行期 `Admission`（`Admission::new` 需要 `&'static`），再问 `Admission::accepts`。因此断言的是**门禁**，不是字符串：
  - 被 deny 的路径必须仍被否决（`accepts("ui/experimental") == false`）；
  - 被 allow 的路径必须仍被接纳（`accepts("ui/controls") == true`）；
  - 允许列表之外的路径必须仍被拒（`accepts("elsewhere/x") == false`）。
- 复现：`cd /tmp/nk-probe && CARGO_TARGET_DIR=/tmp/nk-probe/target cargo build --offline && /tmp/nk-probe/target/debug/nk-probe B11`。

## 2. 绿：当前树上的实测输出（完整 18 行）

输入声明：`crate::Admission::new(&["ui", "control"], &["ui/experimental", "ui/x"])`

```text
[probe] B11.compact = Ok("allow:ui,control;deny:ui/experimental,ui/x")
[probe] B11.owned = Ok(OwnedAdmission { allowed_paths: ["ui", "control"], denied_paths: ["ui/experimental", "ui/x"] })
[probe] B11.lists_survive_the_read = ok compact="allow:ui,control;deny:ui/experimental,ui/x" owned=Ok(OwnedAdmission { allowed_paths: ["ui", "control"], denied_paths: ["ui/experimental", "ui/x"] })
[probe] B11.gate_denies_the_vetoed_path = ok accepts(ui/experimental)=false
[probe] B11.gate_admits_an_allowed_path = ok accepts(ui/controls)=true
[probe] B11.gate_refuses_outside_the_allow_list = ok accepts(elsewhere/x)=false
[probe] B11.rendered = Ok("crate::Admission::new(&[\"ui\", \"control\"], &[\"ui/experimental\", \"ui/x\"])")
[probe] B11.render_carries_the_deny_list = ok crate::Admission::new(&["ui", "control"], &["ui/experimental", "ui/x"])
[probe] B11.round_trip_is_a_fixed_point = ok reread="allow:ui,control;deny:ui/experimental,ui/x" compact="allow:ui,control;deny:ui/experimental,ui/x"
[probe] B11.legacy[ANY] = ok Ok(OwnedAdmission { allowed_paths: [], denied_paths: [] })
[probe] B11.legacy[allow:a,b] = ok Ok(OwnedAdmission { allowed_paths: ["a", "b"], denied_paths: [] })
[probe] B11.legacy[deny:c] = ok Ok(OwnedAdmission { allowed_paths: [], denied_paths: ["c"] })
[probe] B11.legacy[ALLOW : a] = ok Ok(OwnedAdmission { allowed_paths: ["a"], denied_paths: [] })
[probe] B11.legacy[allow:a; deny:b] = ok Ok(OwnedAdmission { allowed_paths: ["a"], denied_paths: ["b"] })
[probe] B11.refuses[allow:a;allow:b] = ok parse_err=true render_err=true
[probe] B11.refuses[allow:a;veto:b] = ok parse_err=true render_err=true
[probe] B11.refuses[allow:a;deny:] = ok parse_err=true render_err=true
[probe] B11.refuses[allow:a;b] = ok parse_err=true render_err=true
[probe] B11.refuses[allow:] = ok parse_err=true render_err=true
[probe] B11.VERDICT = pass
```

逐条对上验收：① 两张列表非空时读回不丢（`lists_survive_the_read`、`owned` 两行）；② **语义**：重建门禁后被 deny 的路径仍被否决（`gate_denies_the_vetoed_path = ok`，另两条把“allow 仍生效、列表外仍拒”也钉住）；渲染带 deny、读→渲染→读是定点、历史单列表拼法（`ANY`/`allow:`/`deny:`/大小写与空格）不变、5 个畸形子句被两侧一致拒绝。

## 3. 变异测试（把实现改回旧行为 → 变红 → 还原 → 零残留）

**协议（每一步都留痕）**

1. `cp core/src/registry_core/authoring/parse/admission.rs /tmp/nk-probe/backup/admission.rs.orig`，并钉住两个锚：文件 sha256 `c14af278…`、`git diff -- <file>` 的 sha256 `1955a873…`。
2. 变异：把 `parse_admission_expression` 里**那一行**改回旧逻辑（即 t2 记录的 LGC-LG-02 行为）：

```text
-        return Ok(compact_admission(&allow, &deny));
+        return match (allow.is_empty(), deny.is_empty()) {
+            (true, true) => Ok("ANY".to_owned()),
+            (false, _) => Ok(format!("allow:{}", allow.join(","))),
+            (true, false) => Ok(format!("deny:{}", deny.join(","))),
+        };
```

3. 重新构建探针（关键：探针静态链接内核 rlib，必须重建才反映新源码）后运行。
4. 还原：`cp /tmp/nk-probe/backup/admission.rs.orig <file>`，再核对 §“零残留”三项。
5. 复跑作者钉子与我的探针，确认双双复绿。

**变异后的红（我的探针，节选关键 3 行 + 判决行）**

```text
[probe] B11.compact = Ok("allow:ui,control")
[probe] B11.owned = Ok(OwnedAdmission { allowed_paths: ["ui", "control"], denied_paths: [] })
[probe] B11.lists_survive_the_read = FAIL compact="allow:ui,control" owned=Ok(OwnedAdmission { allowed_paths: ["ui", "control"], denied_paths: [] })
[probe] B11.gate_denies_the_vetoed_path = FAIL accepts(ui/experimental)=true
[probe] B11.render_carries_the_deny_list = FAIL crate::Admission::new(&["ui", "control"], &[])
[probe] B11.VERDICT = FAIL (lists_survive_the_read, gate_denies_the_vetoed_path, render_carries_the_deny_list)
```

`gate_denies_the_vetoed_path = FAIL` 那一行是本条复核最有价值的一行：旧实现下，被 deny 的路径**变成了被接纳**——门禁确实被放宽，而不只是文本少了一个词。

**同一变异下，作者自己的钉子也红（对照，证明红来自实现而非我的装置）**

```text
test registry_core::authoring::parse::admission::tests::both_lists_survive_the_read_and_write_round_trip ... FAILED
thread '…' panicked at core/src/registry_core/authoring/parse/admission.rs:324:9:
assertion `left == right` failed: reading `crate::Admission::new(&["ui"], &["ui/experimental"])` dropped the deny list: `allow:ui`
  left: OwnedAdmission { allowed_paths: ["ui"], denied_paths: [] }
 right: OwnedAdmission { allowed_paths: ["ui"], denied_paths: ["ui/experimental"] }
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 224 filtered out; finished in 0.00s
```

注意：这条钉子**只在 `--features syntax` 下存在**——不带该特性时 `cargo test -p xirang-core --offline both_lists_survive` 得到 `0 passed; 176 filtered out`（测试根本不在二进制里）。契约里点名这个特性开关是承重的。

**还原后的零残留核对（全部通过）**

```text
sha 还原一致:      YES  (c14af278b8e1fcc1165d89b6364b38cef00b7a265e8b7730de9e97cda77a69cd)
git diff 哈希一致: YES  (1955a873bd3ac045f4804ceefec115636417ffc4d90d6c7d7cff3a697b19849f)
旧分支残留行数:    0    (grep '(true, false) => Ok(format!("deny:{}", deny.join(",")))')
实现行仍在:        1    (grep 'return Ok(compact_admission(&allow, &deny));')
作者钉子复绿:      ok 1 passed / 0 failed
```

**一次装置卫生自纠（记下来以免误读）**：变异窗口结束后我先复跑了一次探针，得到 `VERDICT = FAIL`——但当时源码**已经还原**（作者钉子是绿的）。原因是探针二进制仍是变异期编译的那份（静态链着旧 rlib）。重建后同一条命令 `VERDICT = pass`。这条既解释了我为什么不把那次 FAIL 当作结论，也顺带说明：**红来自被链接进去的实现本身**。

## 4. 门禁（静置树）

静置判定：轮询直到 `find core studio run_method build_method mcp cli -name '*.rs' -newermt '-90 seconds'` 为空 → **18:34:10 静置**；跑完两条门禁后再查“自 18:34:10 起有新写入的文件”为空，且被验证文件的 sha 未变（`admission.rs c14af278…`、`build_method/node_id.rs 6ffd5f2c…`、`build_method/identity_cache.rs c1d8411e…`）。HEAD 仍 `a524956`。

**门禁 1：`cargo test -p xirang-core --offline --features syntax`（exit 0）**

```text
test result: ok. 225 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.48s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.72s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

**门禁 2：`cargo test --workspace --offline`（exit 0）**

```text
（无任何 FAILED 行；52 个 test binary 全部 test result: ok）
```

**我为什么特意等到 18:34:10**：第一次（非静置）跑 workspace 时看到的是别人的在途写入造成的瞬时红——`build_method::identity_cache` 两条（`build_method/node_id.rs` 当时刚在 18:31:34 被写）与 `admission` 钉子一条（18:31:56）；其中 admission 那条在同一份 sha 下隔离复跑是绿的，`identity_cache` 两条随后也消失。按契约“静置树上贴输出”，以 §4 这两段为准；第一次的红与 B1-1 的结论无关，故只作说明、不列为发现。

## 5. 结论

**证实**（B1-1 的修复成立），三方面都有实测：

1. **读回不丢**：`crate::Admission::new(&["ui", "control"], &["ui/experimental", "ui/x"])` 读成 `allow:ui,control;deny:ui/experimental,ui/x`，`OwnedAdmission` 两张列表都在；渲染回源码仍是两张列表；读→渲染→读是定点。
2. **语义未被放宽**（比“文本里还有 deny”更强）：把读回的两张列表还原成运行期门禁后，被 deny 的路径**仍被否决**（`accepts("ui/experimental")==false`），允许路径仍接纳、列表外仍拒。
3. **探针敏感**：把实现改回旧行为后，探针三行变红（其中 `accepts(ui/experimental)=true` 直接显示门禁被放宽），作者的钉子在 `--features syntax` 下同时变红；还原后两者复绿，且文件 sha 与 `git diff` 哈希都与变异前逐字节相同。

**未覆盖/不声称**：我没有复核 t43 对紧凑语法其他扩展的选择（如 `;` 作为子句分隔符、重复子句拒绝的语义取舍），只验证了“两张列表不丢 + 门禁不放宽 + 与内核规范拼法一致”这一条主线；`admission.rs:48/52` 仍有一条历史单列表分支自行拼 `allow:`/`deny:`（一次只看一张列表，不会丢字段），建议下一轮并入 `compact_admission`——那属于内核 API 面，不在 t55 范围。
