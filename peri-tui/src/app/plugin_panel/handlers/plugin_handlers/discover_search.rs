use tui_textarea::{Input, Key};

use crate::app::panel_manager::{EventResult, PanelContext};
use crate::app::plugin_panel::PluginPanel;

impl PluginPanel {
    pub(crate) fn handle_discover_searching(
        &mut self,
        input: Input,
        ctx: &mut PanelContext<'_>,
    ) -> EventResult {
        match input {
            // ── 字符输入 ────────────────────────────────────────────────
            Input {
                key: Key::Char(c),
                ctrl: false,
                alt: false,
                ..
            } => {
                self.discover_search.insert(c);
                self.discover_list.set_items(
                    self.discover_filtered_plugins()
                        .into_iter()
                        .cloned()
                        .collect(),
                );
                EventResult::Consumed
            }
            // ── 光标移动 ────────────────────────────────────────��───────
            Input {
                key: Key::Left,
                ctrl: false,
                ..
            } => {
                self.discover_search.cursor_left();
                EventResult::Consumed
            }
            Input {
                key: Key::Right,
                ctrl: false,
                shift: false,
                ..
            } => {
                self.discover_search.cursor_right();
                EventResult::Consumed
            }
            Input {
                key: Key::Home, ..
            } => {
                self.discover_search.cursor_home();
                EventResult::Consumed
            }
            Input { key: Key::End, .. } => {
                self.discover_search.cursor_end();
                EventResult::Consumed
            }
            // ── 跳词 ────────────────────────────────────────────────────
            Input {
                key: Key::Left,
                ctrl: true,
                ..
            } => {
                self.discover_search.cursor_word_left();
                EventResult::Consumed
            }
            Input {
                key: Key::Right,
                ctrl: true,
                ..
            } => {
                self.discover_search.cursor_word_right();
                EventResult::Consumed
            }
            // ── 删除 ────────────────────────────────────────────────────
            Input {
                key: Key::Backspace,
                alt: false,
                ..
            } => {
                self.discover_search.backspace();
                self.discover_list.set_items(
                    self.discover_filtered_plugins()
                        .into_iter()
                        .cloned()
                        .collect(),
                );
                EventResult::Consumed
            }
            Input {
                key: Key::Backspace,
                alt: true,
                ..
            }
            | Input {
                key: Key::Char('w'),
                ctrl: true,
                ..
            } => {
                self.discover_search.delete_word_backward();
                self.discover_list.set_items(
                    self.discover_filtered_plugins()
                        .into_iter()
                        .cloned()
                        .collect(),
                );
                EventResult::Consumed
            }
            Input {
                key: Key::Delete, ..
            } => {
                self.discover_search.delete();
                self.discover_list.set_items(
                    self.discover_filtered_plugins()
                        .into_iter()
                        .cloned()
                        .collect(),
                );
                EventResult::Consumed
            }
            // ── 退出搜索 ────────────────────────────────────────────────
            Input { key: Key::Up, .. } => {
                self.discover_searching = false;
                self.discover_list.move_cursor(-1);
                EventResult::Consumed
            }
            Input { key: Key::Down, .. } => {
                self.discover_searching = false;
                self.discover_list.move_cursor(1);
                EventResult::Consumed
            }
            Input { key: Key::Esc, .. } => {
                self.discover_searching = false;
                EventResult::Consumed
            }
            Input {
                key: Key::Enter, ..
            } => {
                self.discover_searching = false;
                self.discover_list.set_items(
                    self.discover_filtered_plugins()
                        .into_iter()
                        .cloned()
                        .collect(),
                );
                self.spawn_install_current(ctx);
                EventResult::Consumed
            }
            _ => EventResult::Consumed,
        }
    }
}
