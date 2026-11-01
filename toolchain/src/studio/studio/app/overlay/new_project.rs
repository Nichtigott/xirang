//! New Project overlay keyboard handling.
//! 新建项目浮层键盘处理。

use super::super::*;

impl App {
    pub(super) fn handle_new_project_overlay_key(
        &mut self,
        key: KeyEvent,
        mut project: NewProjectState,
    ) {
        if !project.editing && matches!(key.code, KeyCode::Char('q')) {
            self.overlay = None;
            return;
        }
        if project.editing {
            match key.code {
                KeyCode::Enter => project.editing = false,
                KeyCode::Backspace => {
                    project.values[project.field].pop();
                }
                KeyCode::Char(character) => project.values[project.field].push(character),
                _ => {}
            }
        } else {
            match key.code {
                KeyCode::Up => project.field = project.field.saturating_sub(1),
                KeyCode::Down | KeyCode::Tab => {
                    project.field = (project.field + 1).min(new_project_field::COUNT - 1)
                }
                KeyCode::Enter if project.field == new_project_field::KIND => {
                    project.values[new_project_field::KIND] =
                        if project.values[new_project_field::KIND] == "binary" {
                            "library".to_owned()
                        } else {
                            "binary".to_owned()
                        };
                }
                KeyCode::Enter => project.editing = true,
                KeyCode::Char('s') => {
                    self.submit_new_project(&project);
                    return;
                }
                _ => {}
            }
        }
        self.overlay = Some(Overlay::NewProject(project));
    }
}
