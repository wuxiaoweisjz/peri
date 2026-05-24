use ratatui::style::Style;
use ratatui::text::Span;
use tui_textarea::TextArea;

use crate::ui::theme;

/// 确保光标在滚动视口内可见，返回调整后的 scroll_offset
pub fn ensure_cursor_visible(cursor_row: u16, scroll_offset: u16, visible_height: u16) -> u16 {
    if visible_height == 0 {
        return 0;
    }
    if cursor_row < scroll_offset {
        cursor_row
    } else if cursor_row >= scroll_offset + visible_height {
        cursor_row.saturating_sub(visible_height - 1)
    } else {
        scroll_offset
    }
}

/// 找到 word 左边界的字符索引。cjk 字符视为独立 word。
/// cursor: 当前字符索引（不含），从 cursor-1 向前扫描。
/// 规则：
///   - 跳过 whitespace
///   - 同类字符连续作为同一个 word（alphanumeric 一类，其他符号各自独立但同类合并）
pub fn find_word_start(chars: &[char], cursor: usize) -> usize {
    if cursor == 0 {
        return 0;
    }
    let len = chars.len();
    if cursor > len {
        return len;
    }
    let mut pos = cursor;
    // 跳过空白
    while pos > 0 && chars[pos - 1].is_whitespace() {
        pos -= 1;
    }
    if pos == 0 {
        return 0;
    }
    let cat = char_category(chars[pos - 1]);
    // 向前扫描同类字符
    while pos > 0 {
        let prev = chars[pos - 1];
        if prev.is_whitespace() {
            return pos;
        }
        if char_category(prev) != cat {
            return pos;
        }
        pos -= 1;
    }
    pos
}

/// 找到 word 右边界的字符索引。
/// cursor: 当前字符索引（含），从 cursor 向后扫描。
pub fn find_word_end(chars: &[char], cursor: usize) -> usize {
    let len = chars.len();
    if cursor >= len {
        return len;
    }
    let mut pos = cursor;
    // 跳过空白
    while pos < len && chars[pos].is_whitespace() {
        pos += 1;
    }
    if pos >= len {
        return len;
    }
    let cat = char_category(chars[pos]);
    while pos < len {
        if chars[pos].is_whitespace() {
            return pos;
        }
        if char_category(chars[pos]) != cat {
            return pos;
        }
        pos += 1;
    }
    pos
}

/// 字符类别：alphanumeric / other。用于 word 边界判断。
fn char_category(c: char) -> u8 {
    if c.is_alphanumeric() || c == '_' {
        0
    } else {
        1
    }
}

// ─── 公共单行文本编辑辅助 ────────────────────────────────────────────────────

/// 对单行 `String` + 光标位置统一处理编辑按键。
/// 返回 `true` 表示该按键已被消费（调用方应停止 match）。
///
/// 支持的按键：Char、Backspace、Delete、Left、Right、Home、End、
/// Ctrl+A(Home)、Ctrl+E(End)、Ctrl+K(kill to end)、Ctrl+U(kill to start)、
/// Ctrl+Left(word left)、Ctrl+Right(word right)、Ctrl+W(delete word backward)、
/// Alt+Backspace(delete word backward)
pub fn handle_edit_key(buf: &mut String, cursor: &mut usize, input: tui_textarea::Input) -> bool {
    use tui_textarea::Key;
    match input {
        // ── 字符输入 ────────────────────────────────────────────────────────
        tui_textarea::Input {
            key: Key::Char(c),
            ctrl: false,
            alt: false,
            ..
        } => {
            let char_count = buf.chars().count();
            if *cursor > char_count {
                *cursor = char_count;
            }
            let byte_pos = buf
                .char_indices()
                .nth(*cursor)
                .map(|(i, _)| i)
                .unwrap_or(buf.len());
            buf.insert(byte_pos, c);
            *cursor += 1;
            true
        }
        // ── Backspace：删除光标前一个字符 ──────────────────────────────────
        tui_textarea::Input {
            key: Key::Backspace,
            alt: false,
            ..
        } => {
            let char_count = buf.chars().count();
            if *cursor > 0 && *cursor <= char_count {
                let byte_pos = buf.char_indices().nth(*cursor - 1).map(|(i, _)| i);
                let next_byte = buf
                    .char_indices()
                    .nth(*cursor)
                    .map(|(i, _)| i)
                    .unwrap_or(buf.len());
                if let Some(bp) = byte_pos {
                    buf.drain(bp..next_byte);
                    *cursor -= 1;
                }
            }
            true
        }
        // ── Delete：删除光标后一个字符 ─────────────────────────────────────
        tui_textarea::Input {
            key: Key::Delete, ..
        } => {
            let char_count = buf.chars().count();
            if *cursor < char_count {
                let byte_pos = buf.char_indices().nth(*cursor).map(|(i, _)| i);
                let next_byte = buf
                    .char_indices()
                    .nth(*cursor + 1)
                    .map(|(i, _)| i)
                    .unwrap_or(buf.len());
                if let Some(bp) = byte_pos {
                    buf.drain(bp..next_byte);
                }
            }
            true
        }
        // ── Left / Ctrl+A(Home) ────────────────────────────────────────────
        tui_textarea::Input {
            key: Key::Left,
            ctrl: false,
            ..
        } => {
            if *cursor > 0 {
                *cursor -= 1;
            }
            true
        }
        tui_textarea::Input { key: Key::Home, .. }
        | tui_textarea::Input {
            key: Key::Char('a'),
            ctrl: true,
            ..
        } => {
            *cursor = 0;
            true
        }
        // ── Right / Ctrl+E(End) ────────────────────────────────────────────
        tui_textarea::Input {
            key: Key::Right,
            ctrl: false,
            ..
        } => {
            if *cursor < buf.chars().count() {
                *cursor += 1;
            }
            true
        }
        tui_textarea::Input { key: Key::End, .. }
        | tui_textarea::Input {
            key: Key::Char('e'),
            ctrl: true,
            ..
        } => {
            *cursor = buf.chars().count();
            true
        }
        // ── Ctrl+K：删除光标到末尾 ──────────────────────────────────────────
        tui_textarea::Input {
            key: Key::Char('k'),
            ctrl: true,
            ..
        } => {
            if *cursor < buf.chars().count() {
                let byte_pos = buf
                    .char_indices()
                    .nth(*cursor)
                    .map(|(i, _)| i)
                    .unwrap_or(buf.len());
                buf.truncate(byte_pos);
            }
            true
        }
        // ── Ctrl+U：删除开头到光标 ──────────────────────────────────────────
        tui_textarea::Input {
            key: Key::Char('u'),
            ctrl: true,
            ..
        } => {
            let char_count = buf.chars().count();
            if *cursor > 0 && *cursor <= char_count {
                let byte_pos = buf
                    .char_indices()
                    .nth(*cursor)
                    .map(|(i, _)| i)
                    .unwrap_or(buf.len());
                buf.drain(..byte_pos);
                *cursor = 0;
            }
            true
        }
        // ── Ctrl+Left：跳词到左边界 ──────────────────────────────────────
        tui_textarea::Input {
            key: Key::Left,
            ctrl: true,
            ..
        } => {
            let chars: Vec<char> = buf.chars().collect();
            *cursor = find_word_start(&chars, *cursor);
            true
        }
        // ── Ctrl+Right：跳词到右边界 ─────────────────────────────────────
        tui_textarea::Input {
            key: Key::Right,
            ctrl: true,
            ..
        } => {
            let chars: Vec<char> = buf.chars().collect();
            *cursor = find_word_end(&chars, *cursor);
            true
        }
        // ── Ctrl+W：删除光标前一个 word ──────────────────────────────────
        tui_textarea::Input {
            key: Key::Char('w'),
            ctrl: true,
            ..
        } => {
            let char_count = buf.chars().count();
            if *cursor > char_count {
                *cursor = char_count;
            }
            if *cursor > 0 {
                let chars: Vec<char> = buf.chars().collect();
                let start = find_word_start(&chars, *cursor);
                let byte_start = buf.char_indices().nth(start).map(|(i, _)| i).unwrap_or(0);
                let byte_end = buf
                    .char_indices()
                    .nth(*cursor)
                    .map(|(i, _)| i)
                    .unwrap_or(buf.len());
                buf.drain(byte_start..byte_end);
                *cursor = start;
            }
            true
        }
        // ── Alt+Backspace：删除光标前一个 word ──────────────────────────
        tui_textarea::Input {
            key: Key::Backspace,
            alt: true,
            ..
        } => {
            let char_count = buf.chars().count();
            if *cursor > char_count {
                *cursor = char_count;
            }
            if *cursor > 0 {
                let chars: Vec<char> = buf.chars().collect();
                let start = find_word_start(&chars, *cursor);
                let byte_start = buf.char_indices().nth(start).map(|(i, _)| i).unwrap_or(0);
                let byte_end = buf
                    .char_indices()
                    .nth(*cursor)
                    .map(|(i, _)| i)
                    .unwrap_or(buf.len());
                buf.drain(byte_start..byte_end);
                *cursor = start;
            }
            true
        }
        _ => false,
    }
}

/// 将 `(buf, cursor)` 渲染为带光标块的字符串元组 `(before_cursor, after_cursor)`。
/// 调用方在两者之间插入 `█` 或 `▏` Span 即可。
pub fn edit_display_parts(buf: &str, cursor: usize) -> (String, String) {
    let chars: Vec<char> = buf.chars().collect();
    let clamped = cursor.min(chars.len());
    let before: String = chars[..clamped].iter().collect();
    let after: String = chars[clamped..].iter().collect();
    (before, after)
}

pub fn build_textarea(disabled: bool) -> TextArea<'static> {
    build_textarea_with_hint(disabled, "")
}

fn build_textarea_with_hint(_disabled: bool, hint: &str) -> TextArea<'static> {
    let mut ta = TextArea::default();

    // 统一灰色边框
    let border_color = theme::MUTED;

    ta.set_cursor_line_style(Style::default());
    ta.set_style(Style::default().fg(theme::TEXT));
    let mut block = ratatui::widgets::Block::default()
        .borders(ratatui::widgets::Borders::TOP | ratatui::widgets::Borders::BOTTOM)
        .border_style(Style::default().fg(border_color))
        .padding(ratatui::widgets::Padding::new(2, 0, 0, 0));
    if !hint.is_empty() {
        block = block.title(Span::styled(
            hint.to_owned(),
            Style::default().fg(theme::MUTED),
        ));
    }
    ta.set_block(block);
    ta
}

#[cfg(test)]
#[path = "edit_utils_test.rs"]
mod tests;
