//! Plugin overlay keyboard handling.
//! 插件浮层键盘处理。

use super::super::*;

impl App {
    pub(super) fn handle_plugin_overlay_key(&mut self, key: KeyEvent, mut plugin: PluginState) {
        if !plugin.editing && matches!(key.code, KeyCode::Char('q')) {
            self.overlay = None;
            return;
        }
        if plugin.editing {
            match key.code {
                KeyCode::Enter => plugin.editing = false,
                KeyCode::Backspace => {
                    plugin.values[plugin.field].pop();
                }
                KeyCode::Char(character) => plugin.values[plugin.field].push(character),
                _ => {}
            }
        } else {
            // Any key consumes an armed write, so confirming takes two presses on
            // the same form and nothing else can carry the arm into a write.
            // 任何按键都会消费掉待写状态，因此确认需要在同一份表单上按两次，别的动作不可能把
            // 这个状态带进一次写入。
            let armed = std::mem::take(&mut plugin.pending_submit);
            match key.code {
                KeyCode::Up => plugin.field = plugin.field.saturating_sub(1),
                KeyCode::Down | KeyCode::Tab => {
                    plugin.field = (plugin.field + 1).min(plugin_field::COUNT - 1)
                }
                KeyCode::Enter
                    if plugin.field == plugin_field::SOURCE
                        || plugin.field == plugin_field::MODE =>
                {
                    plugin.values[plugin.field] = match plugin.field {
                        plugin_field::SOURCE
                            if plugin.values[plugin_field::SOURCE] == "official" =>
                        {
                            "user".to_owned()
                        }
                        plugin_field::SOURCE => "official".to_owned(),
                        plugin_field::MODE if plugin.values[plugin_field::MODE] == "extension" => {
                            "replacement".to_owned()
                        }
                        _ => "extension".to_owned(),
                    };
                }
                KeyCode::Enter => plugin.editing = true,
                KeyCode::Char('s') if armed => {
                    self.submit_plugin(&plugin);
                    return;
                }
                KeyCode::Char('s') => {
                    plugin.pending_submit = true;
                    self.event =
                        "Plugin: press s again to write the entry line and the lock record"
                            .to_owned();
                }
                _ => {}
            }
        }
        self.overlay = Some(Overlay::Plugin(plugin));
    }
}
