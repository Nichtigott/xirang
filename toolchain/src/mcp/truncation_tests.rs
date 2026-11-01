//! Pins for the one truncation sentence: the three facts it must carry, and the
//! fact that no other module spells it.
//! 唯一那句截断说明的钉子：它必须携带的三件事，以及没有别的模块拼它这件事。

use super::{bounded, withheld, withheld_uncounted};

/// The sentence carries all three facts at once: how many rows were withheld, which
/// cap did it, and how to reach the rest.
/// 这句话一次携带全部三件事：扣下多少行、哪道上限扣的、怎么拿到其余部分。
#[test]
fn the_sentence_carries_the_count_the_limit_and_the_way_out() {
    let line = withheld(2, 4, 2, "plan rows", "raise `limit`");
    assert!(line.contains("2 of 4 plan rows withheld"), "{line}");
    assert!(line.contains("at the limit of 2"), "{line}");
    assert!(line.contains("raise `limit`"), "{line}");
}

/// The block form keeps exactly `limit` lines and replaces the rest with the
/// sentence, and leaves a block that fits untouched — a bound that fired on a short
/// answer would make every answer look truncated.
/// 整块形式恰好保留 `limit` 行、其余用那句话替代，而装得下的块原样返回——上限对短答案生效会让
/// 每条答案看起来都被截断过。
#[test]
fn the_block_form_bounds_and_otherwise_leaves_text_alone() {
    let short = "a\nb\nc\n";
    assert_eq!(bounded(short, 3, "lines", "hint"), short);
    let long = "a\nb\nc\nd\ne\n";
    let cut = bounded(long, 3, "lines", "hint");
    assert!(cut.starts_with("a\nb\nc\n"), "{cut}");
    assert!(
        cut.contains("2 of 5 lines withheld at the limit of 3"),
        "{cut}"
    );
}

/// The uncounted form still names the cap and the way out, and says out loud that it
/// has no number — the one fact it cannot produce.
/// 未计数形式仍然点名上限与出路，并说出它没有数字——它唯一产不出的那件事。
#[test]
fn the_uncounted_form_says_which_fact_it_cannot_give() {
    let line = withheld_uncounted(200, "diff lines", "read the written files for the rest");
    assert!(line.contains("at the limit of 200"), "{line}");
    assert!(line.contains("the exact count is not computed"), "{line}");
    assert!(
        line.contains("read the written files for the rest"),
        "{line}"
    );
}

/// The phrase has one outlet: `truncation.rs` spells it and every other module in
/// this bridge calls [`withheld`]/[`withheld_uncounted`]/[`bounded`] instead. A second
/// copy is how the wording drifted into the three shapes this module replaced.
/// 这句话只有一个出口：`truncation.rs` 拼它，本桥其它模块改为调用
/// [`withheld`]/[`withheld_uncounted`]/[`bounded`]。第二份副本正是措辞漂移成被本模块替换掉的
/// 那三种形状的方式。
#[test]
fn the_truncation_phrase_has_one_outlet() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/mcp");
    let mut offenders = Vec::new();
    let mut stack = vec![root];
    while let Some(directory) = stack.pop() {
        for entry in std::fs::read_dir(&directory)
            .expect("mcp/src is readable")
            .flatten()
        {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            if path.extension().and_then(|extension| extension.to_str()) != Some("rs") {
                continue;
            }
            // `Path::ends_with` compares whole components, so the suffix tests are on the
            // rendered name.
            // `Path::ends_with` 比的是整个分量，因此这些后缀判断落在渲染出的名字上。
            let name = path.to_string_lossy();
            if name.ends_with("truncation.rs")
                || name.ends_with("_tests.rs")
                || name.ends_with(file!())
            {
                // The outlet itself, and the test modules — which spell the sentence
                // because pinning the wording is their job.
                // 出口本身，以及测试模块——它们拼这句话，因为钉住措辞正是它们的活。
                continue;
            }
            let source = std::fs::read_to_string(&path).expect("a readable module");
            if source.contains(super::PHRASE) {
                offenders.push(path.display().to_string());
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "these modules spell the truncation sentence instead of calling the outlet: {offenders:?}"
    );
}
