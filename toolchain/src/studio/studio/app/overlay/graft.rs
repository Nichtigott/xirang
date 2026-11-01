//! External graft overlay keyboard handling.
//! 外部 graft 浮层键盘处理。

use super::super::*;

impl App {
    pub(super) fn handle_graft_overlay_key(&mut self, key: KeyEvent, mut graft: GraftState) {
        if !graft.editing && matches!(key.code, KeyCode::Char('q')) {
            self.overlay = None;
            return;
        }
        if graft.editing {
            match key.code {
                KeyCode::Enter => graft.editing = false,
                KeyCode::Backspace => {
                    graft.selector.pop();
                }
                KeyCode::Char(character) => graft.selector.push(character),
                _ => {}
            }
        } else {
            // Any key consumes an armed deletion, so confirming takes two
            // deliberate presses on the same record and nothing else can carry
            // the arm into an unrelated action.
            // 任何按键都会消费掉待删状态，因此确认需要针对同一条记录两次有意的按键，别的动作
            // 不可能把这个状态带过去。
            let armed = graft.pending_delete.take();
            match key.code {
                KeyCode::Tab => {
                    graft.pane = if graft.pane == 0 && !graft.plans.is_empty() {
                        1
                    } else {
                        0
                    };
                }
                KeyCode::Up => {
                    if graft.pane == 1 {
                        graft.plan_selected = graft.plan_selected.saturating_sub(1);
                    } else {
                        graft.field = graft.field.saturating_sub(1);
                    }
                }
                KeyCode::Down => {
                    if graft.pane == 1 {
                        graft.plan_selected =
                            (graft.plan_selected + 1).min(graft.plans.len().saturating_sub(1));
                    } else {
                        graft.field = (graft.field + 1).min(1);
                    }
                }
                KeyCode::Enter | KeyCode::Char(' ') if graft.pane == 0 => {
                    if graft.field == 1 {
                        graft.full = !graft.full;
                    } else {
                        graft.editing = true;
                    }
                }
                KeyCode::Enter if graft.pane == 1 => {
                    if let Some(plan) = graft.plans.get(graft.plan_selected) {
                        let selector = plan.selector.clone();
                        self.open_graft_plan(&selector);
                    }
                }
                KeyCode::Char('s') => {
                    self.submit_graft(&graft);
                    self.overlay = Some(Overlay::Graft(graft));
                    self.refresh_graft();
                    return;
                }
                KeyCode::Char('o') => {
                    let selector = self.selected_graft_selector(&graft);
                    if let Some(selector) = selector {
                        self.open_graft_plan(&selector);
                    } else {
                        self.note("Graft: no plan exists for this selector yet".to_owned());
                    }
                    self.overlay = Some(Overlay::Graft(graft));
                    return;
                }
                KeyCode::Char('f') => {
                    let plan = self.selected_graft_selector(&graft).and_then(|selector| {
                        graft
                            .plans
                            .iter()
                            .find(|plan| plan.selector == selector)
                            .map(|plan| (selector, plan.full))
                    });
                    if let Some((selector, full)) = plan {
                        self.toggle_graft_plan(&selector, full);
                    } else {
                        self.note("Graft: no plan exists for this selector yet".to_owned());
                    }
                    self.overlay = Some(Overlay::Graft(graft));
                    self.refresh_graft();
                    return;
                }
                KeyCode::Char('d') => {
                    let selector = self.selected_graft_selector(&graft);
                    match selector {
                        Some(selector) if armed.as_deref() == Some(selector.as_str()) => {
                            self.delete_graft_plan(&selector);
                        }
                        Some(selector) => {
                            self.event =
                                format!("Graft: press d again to move `{selector}` to the trash");
                            graft.pending_delete = Some(selector);
                        }
                        None => {
                            self.note("Graft: no plan exists for this selector yet".to_owned());
                        }
                    }
                    self.overlay = Some(Overlay::Graft(graft));
                    self.refresh_graft();
                    return;
                }
                _ => {}
            }
        }
        self.overlay = Some(Overlay::Graft(graft));
    }
}
