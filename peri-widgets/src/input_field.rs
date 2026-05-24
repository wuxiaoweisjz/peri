use ratatui::{layout::Rect, prelude::*, style::Style, text::Line, widgets::StatefulWidget};

/// 文本输入状态——管理 buffer + cursor（UTF-8 字节偏移）+ masked 标志
#[derive(Debug, Clone)]
pub struct InputState {
    buffer: String,
    /// UTF-8 字节偏移（不是字符偏移）
    cursor: usize,
    masked: bool,
}

impl Default for InputState {
    fn default() -> Self {
        Self::new()
    }
}

impl InputState {
    pub fn new() -> Self {
        Self {
            buffer: String::new(),
            cursor: 0,
            masked: false,
        }
    }

    pub fn with_value(value: String) -> Self {
        let cursor = value.len();
        Self {
            buffer: value,
            cursor,
            masked: false,
        }
    }

    pub fn masked(mut self, masked: bool) -> Self {
        self.masked = masked;
        self
    }

    pub fn value(&self) -> &str {
        &self.buffer
    }

    pub fn set_value(&mut self, value: String) {
        self.cursor = value.len();
        self.buffer = value;
    }

    pub fn cursor(&self) -> usize {
        self.cursor
    }

    /// 在 cursor 位置插入一个字符
    pub fn insert(&mut self, c: char) {
        self.buffer.insert(self.cursor, c);
        self.cursor += c.len_utf8();
    }

    /// 删除 cursor 前一个字符（Backspace）
    pub fn backspace(&mut self) {
        if self.cursor > 0 {
            let prev = self.buffer[..self.cursor].chars().last().unwrap();
            self.cursor -= prev.len_utf8();
            self.buffer.remove(self.cursor);
        }
    }

    /// 删除 cursor 位置字符（Delete）
    pub fn delete(&mut self) {
        if self.cursor < self.buffer.len() {
            self.buffer.remove(self.cursor);
        }
    }

    /// cursor 左移一个字符
    pub fn cursor_left(&mut self) {
        if self.cursor > 0 {
            let prev = self.buffer[..self.cursor].chars().last().unwrap();
            self.cursor -= prev.len_utf8();
        }
    }

    /// cursor 右移一个字符
    pub fn cursor_right(&mut self) {
        if self.cursor < self.buffer.len() {
            let next = self.buffer[self.cursor..].chars().next().unwrap();
            self.cursor += next.len_utf8();
        }
    }

    /// cursor 移到开头
    pub fn cursor_home(&mut self) {
        self.cursor = 0;
    }

    /// cursor 移到末尾
    pub fn cursor_end(&mut self) {
        self.cursor = self.buffer.len();
    }

    /// cursor 跳到前一个 word 的开头
    pub fn cursor_word_left(&mut self) {
        if self.cursor == 0 {
            return;
        }
        let chars: Vec<char> = self.buffer.chars().collect();
        let char_idx = self.buffer[..self.cursor].chars().count();
        if char_idx == 0 {
            return;
        }
        // 向前跳过空白
        let mut pos = char_idx;
        while pos > 0 && chars[pos - 1].is_whitespace() {
            pos -= 1;
        }
        if pos == 0 {
            self.cursor = 0;
            return;
        }
        // 确定当前字符类别
        let cat = if chars[pos - 1].is_alphanumeric() || chars[pos - 1] == '_' {
            0u8
        } else {
            1u8
        };
        // 向前扫描同类字符
        while pos > 0 {
            let prev = chars[pos - 1];
            if prev.is_whitespace() {
                break;
            }
            let prev_cat = if prev.is_alphanumeric() || prev == '_' {
                0u8
            } else {
                1u8
            };
            if prev_cat != cat {
                break;
            }
            pos -= 1;
        }
        self.cursor = self.byte_offset_at(&chars, pos);
    }

    /// cursor 跳到下一个 word 的开头（跳过当前 word）
    pub fn cursor_word_right(&mut self) {
        let chars: Vec<char> = self.buffer.chars().collect();
        let char_idx = self.buffer[..self.cursor].chars().count();
        let len = chars.len();
        if char_idx >= len {
            return;
        }
        // 跳过空白
        let mut pos = char_idx;
        while pos < len && chars[pos].is_whitespace() {
            pos += 1;
        }
        if pos >= len {
            self.cursor = self.buffer.len();
            return;
        }
        let cat = if chars[pos].is_alphanumeric() || chars[pos] == '_' {
            0u8
        } else {
            1u8
        };
        while pos < len {
            if chars[pos].is_whitespace() {
                break;
            }
            let cur_cat = if chars[pos].is_alphanumeric() || chars[pos] == '_' {
                0u8
            } else {
                1u8
            };
            if cur_cat != cat {
                break;
            }
            pos += 1;
        }
        self.cursor = self.byte_offset_at(&chars, pos);
    }

    /// 删除光标前一个 word
    pub fn delete_word_backward(&mut self) {
        if self.cursor == 0 {
            return;
        }
        let chars: Vec<char> = self.buffer.chars().collect();
        let char_idx = self.buffer[..self.cursor].chars().count();
        if char_idx == 0 {
            return;
        }
        let mut pos = char_idx;
        while pos > 0 && chars[pos - 1].is_whitespace() {
            pos -= 1;
        }
        if pos == 0 {
            let byte_end = self.cursor;
            self.buffer.drain(..byte_end);
            self.cursor = 0;
            return;
        }
        let cat = if chars[pos - 1].is_alphanumeric() || chars[pos - 1] == '_' {
            0u8
        } else {
            1u8
        };
        while pos > 0 {
            let prev = chars[pos - 1];
            if prev.is_whitespace() {
                break;
            }
            let prev_cat = if prev.is_alphanumeric() || prev == '_' {
                0u8
            } else {
                1u8
            };
            if prev_cat != cat {
                break;
            }
            pos -= 1;
        }
        let byte_start = self.byte_offset_at(&chars, pos);
        let byte_end = self.cursor;
        self.buffer.drain(byte_start..byte_end);
        self.cursor = byte_start;
    }

    /// 辅助：char index → byte offset
    fn byte_offset_at(&self, chars: &[char], idx: usize) -> usize {
        if idx >= chars.len() {
            return self.buffer.len();
        }
        chars.iter().take(idx).map(|c| c.len_utf8()).sum()
    }

    /// 在 cursor 位置粘贴文本
    pub fn paste(&mut self, text: &str) {
        self.buffer.insert_str(self.cursor, text);
        self.cursor += text.len();
    }

    /// 计算显示宽度（考虑 Unicode 宽字符）
    pub fn display_width(&self) -> usize {
        unicode_width::UnicodeWidthStr::width(self.buffer.as_str())
    }

    /// 获取用于显示的文本（masked 时返回遮罩字符串）
    pub fn display_text(&self, mask_char: char) -> String {
        if self.masked {
            let w = self.buffer.chars().count();
            if w <= 8 {
                mask_char.to_string().repeat(w)
            } else {
                let chars: Vec<char> = self.buffer.chars().collect();
                format!(
                    "{}{}{}{}{}{}",
                    &chars[..4].iter().collect::<String>(),
                    mask_char,
                    mask_char,
                    mask_char,
                    mask_char,
                    &chars[w - 4..].iter().collect::<String>()
                )
            }
        } else {
            self.buffer.clone()
        }
    }

    /// 计算显示文本中 cursor 对应的显示位置
    pub fn display_cursor(&self, _mask_char: char) -> usize {
        if self.masked {
            // masked 模式下每个字符宽度为 1
            self.buffer[..self.cursor].chars().count()
        } else {
            unicode_width::UnicodeWidthStr::width(&self.buffer[..self.cursor])
        }
    }
}

/// InputField 渲染样式配置
#[derive(Debug, Clone)]
pub struct InputFieldStyle {
    pub label_focused: Style,
    pub label_unfocused: Style,
    pub value_focused: Style,
    pub value_unfocused: Style,
    pub cursor_char: char,
    pub mask_char: char,
}

impl Default for InputFieldStyle {
    fn default() -> Self {
        Self {
            label_focused: Style::default(),
            label_unfocused: Style::default(),
            value_focused: Style::default(),
            value_unfocused: Style::default(),
            cursor_char: '█',
            mask_char: '•',
        }
    }
}

/// 文本输入框 widget——实现 ratatui StatefulWidget
///
/// 渲染为单行：`  Label  Value█`（focused 时显示光标字符）
/// unfocused 时：`  Label  Value`（无光标）
/// masked 时：Value 显示为遮罩字符串
pub struct InputField<'a> {
    label: &'a str,
    focused: bool,
    style: InputFieldStyle,
}

impl<'a> InputField<'a> {
    pub fn new(label: &'a str) -> Self {
        Self {
            label,
            focused: false,
            style: InputFieldStyle::default(),
        }
    }

    pub fn focused(mut self, focused: bool) -> Self {
        self.focused = focused;
        self
    }

    pub fn style(mut self, style: InputFieldStyle) -> Self {
        self.style = style;
        self
    }

    /// 获取当前渲染的单行内容（用于外部 Paragraph 组装场景）
    pub fn to_line(&self, state: &InputState) -> Line<'static> {
        let (label_style, value_style) = if self.focused {
            (self.style.label_focused, self.style.value_focused)
        } else {
            (self.style.label_unfocused, self.style.value_unfocused)
        };

        let display = state.display_text(self.style.mask_char);
        let value_text = if self.focused {
            format!("{}{}", display, self.style.cursor_char)
        } else {
            display
        };

        Line::from(vec![
            Span::styled(format!("  {} ", self.label), label_style),
            Span::styled(format!(" {}", value_text), value_style),
        ])
    }
}

impl StatefulWidget for InputField<'_> {
    type State = InputState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let line = self.to_line(state);
        Widget::render(line, area, buf);
    }
}

#[cfg(test)]
#[path = "input_field_test.rs"]
mod tests;
