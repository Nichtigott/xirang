//! The add/edit form state machine, shared by both overlays.
//! 添加/编辑共用的表单状态机。
//!
//! Add and Edit show the same rows under the same rules; only what a submit writes
//! differs. Keeping one transition table is what stops the two from drifting apart
//! (audit `STU-S-20`).
//! 添加与编辑显示同样的行、遵循同样的规则，只有提交时写的东西不同。只留一份转移表，正是让两者
//! 不会各自漂移的原因（审计 `STU-S-20`）。

use super::super::*;

/// What one form key press decided.
/// 一次表单按键决定了什么。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum FormAction {
    /// Keep the form open with the state the caller handed in.
    /// 保持表单打开，沿用调用方递进来的状态。
    Stay,
    /// Submit that state: the caller knows which face it belongs to.
    /// 提交那份状态：调用方知道它属于哪个面。
    Submit,
    /// Close the overlay without submitting.
    /// 不提交，直接关闭浮层。
    Close,
}

/// Run one key through the add/edit transitions.
/// 把一个按键跑过添加/编辑的转移。
pub(super) fn face_form_key(key: KeyEvent, form: &mut AddState) -> FormAction {
    if !form.editing && matches!(key.code, KeyCode::Char('q')) {
        return FormAction::Close;
    }
    if form.editing {
        match key.code {
            KeyCode::Enter => form.editing = false,
            KeyCode::Backspace => {
                form.values[form.field].pop();
            }
            KeyCode::Char(character) => form.values[form.field].push(character),
            _ => {}
        }
        return FormAction::Stay;
    }
    match key.code {
        KeyCode::Up => move_face_field(form, -1),
        KeyCode::Down | KeyCode::Tab => move_face_field(form, 1),
        KeyCode::Enter | KeyCode::Char(' ') if form.field == face_field::NEEDS_REGISTRY => {
            form.values[face_field::NEEDS_REGISTRY] =
                (form.values[face_field::NEEDS_REGISTRY] != "true").to_string();
        }
        KeyCode::Enter if form.is_editable(form.field) => form.editing = true,
        // `l` cycles the languages this build can author; the form greys out the rows the
        // selected language's registration record does not carry.
        // `l` 循环本构建可以创作的语言；表单把选中语言注册记录不携带的行画成暗色。
        KeyCode::Char('l') if form.allow_language => form.cycle_language(),
        KeyCode::Char('s') => return FormAction::Submit,
        _ => {}
    }
    FormAction::Stay
}
