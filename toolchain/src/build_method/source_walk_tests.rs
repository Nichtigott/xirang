//! Where an out-of-tree implementation stops being a defect — the rule, pinned apart from the walk that produces it.
//! 树外实现在哪里不再算缺陷——把这条规则与产生它的遍历分开钉住。

mod notice_tests {
    use crate::build_method::source_walk::{
        EXTERNAL_FACE_PHASE, UnplacedFace, notices_and_defects,
    };

    fn face(relative: &str, phase: &'static str) -> UnplacedFace {
        UnplacedFace {
            relative: relative.to_owned(),
            line: 1,
            phase,
            message: format!("{phase} at {relative}"),
        }
    }

    /// The maintainer's ruling of 2026-10-18 (option B1): a host that follows the documented graft workflow must not
    /// get a red `check`. So an external face is printed as a **notice**, and only the other phases reach the
    /// diagnostics that decide the exit code — one function, because the rule and the exit code have to be the same
    /// rule. Measured at the real entry as well: `xirang check examples/control-button-graft` exits 0 and prints the
    /// three notices on stderr (`evidence-external-notice-2026-10-18.md`).
    /// 维护者 2026-10-18 的裁定（选项 B1）：照文档的 graft 工作流不该让 `check` 变红。因此树外面的面被当作**提示**
    /// 打印，只有其它阶段进入决定退出码的那些诊断——只用一个函数，因为"规则"与"退出码"必须是同一条规则。真入口也
    /// 量过：`xirang check examples/control-button-graft` 退出 0，三条提示在 stderr 上。
    #[test]
    fn an_external_face_is_a_notice_and_every_other_phase_is_still_a_defect() {
        let defects = notices_and_defects(vec![
            face("button_fast.rs", EXTERNAL_FACE_PHASE),
            face("control_fast.rs", EXTERNAL_FACE_PHASE),
            face("broken.rs", "face-layout"),
            face("unresolved.rs", "face-parent"),
        ]);
        assert_eq!(
            defects.len(),
            2,
            "the two real defects survive: {defects:?}"
        );
        assert!(
            defects.iter().all(|face| face.phase != EXTERNAL_FACE_PHASE),
            "and no notice reaches the exit code: {defects:?}"
        );
        // The positive control: with no external face at all the list is unchanged, so this filter is not a blanket
        // that would quietly empty the diagnostics.
        // 正控制：完全没有树外面时清单保持不变，因此这个过滤不是一张会把诊断悄悄清空的毯子。
        let only_defects = notices_and_defects(vec![face("broken.rs", "face-layout")]);
        assert_eq!(only_defects.len(), 1);
    }
}
