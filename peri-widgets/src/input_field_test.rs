use super::*;

#[test]
fn input_state_insert_and_backspace() {
    let mut s = InputState::new();
    s.insert('a');
    s.insert('b');
    s.insert('c');
    assert_eq!(s.value(), "abc");
    s.backspace();
    assert_eq!(s.value(), "ab");
    assert_eq!(s.cursor(), 2);
}

#[test]
fn input_state_cursor_movement() {
    let mut s = InputState::with_value("abc".into());
    s.cursor_end();
    assert_eq!(s.cursor(), 3);
    s.cursor_left();
    assert_eq!(s.cursor(), 2);
    s.cursor_home();
    assert_eq!(s.cursor(), 0);
}

#[test]
fn input_state_delete_at_cursor() {
    let mut s = InputState::with_value("abc".into());
    s.cursor_home(); // cursor at 0
    s.cursor_right(); // cursor at 1 ('b')
    s.delete();
    assert_eq!(s.value(), "ac");
}

#[test]
fn input_state_paste() {
    let mut s = InputState::new();
    s.paste("hello");
    assert_eq!(s.value(), "hello");
    assert_eq!(s.cursor(), 5);
}

#[test]
fn input_state_utf8_multibyte() {
    let mut s = InputState::new();
    s.insert('中');
    s.insert('文');
    assert_eq!(s.value(), "中文");
    s.cursor_left();
    s.cursor_left();
    assert_eq!(s.cursor(), 0);
    s.insert('你');
    assert_eq!(s.value(), "你中文");
}

#[test]
fn input_state_masked_display() {
    let s = InputState::with_value("sk-1234567890".into()).masked(true);
    let display = s.display_text('•');
    assert_eq!(display, "sk-1••••7890");
}

#[test]
fn input_state_masked_short() {
    let s = InputState::with_value("abc".into()).masked(true);
    let display = s.display_text('•');
    assert_eq!(display, "•••");
}

#[test]
fn input_field_to_line_focused() {
    let s = InputState::with_value("test".into());
    let field = InputField::new("Name").focused(true);
    let line = field.to_line(&s);
    let line_str: String = line.spans.iter().map(|s| s.content.clone()).collect();
    assert!(
        line_str.contains('█'),
        "Expected cursor char, got: {}",
        line_str
    );
}

#[test]
fn input_field_to_line_unfocused() {
    let s = InputState::with_value("test".into());
    let field = InputField::new("Name").focused(false);
    let line = field.to_line(&s);
    let line_str: String = line.spans.iter().map(|s| s.content.clone()).collect();
    assert!(
        !line_str.contains('█'),
        "Expected no cursor char, got: {}",
        line_str
    );
}

#[test]
fn test_cursor_word_left_basic() {
    let mut s = InputState::with_value("hello world".to_string());
    s.cursor_end();
    s.cursor_word_left();
    // "hello world" = 11 bytes, cursor should be at "world" start = 6
    assert_eq!(s.cursor(), 6, "第一次跳词应到 world 开头");
    s.cursor_word_left();
    assert_eq!(s.cursor(), 0, "第二次跳词应到 hello 开头");
    s.cursor_word_left();
    assert_eq!(s.cursor(), 0, "已在开头不移动");
}

#[test]
fn test_cursor_word_right_basic() {
    let mut s = InputState::with_value("hello world foo".to_string());
    s.cursor_home();
    s.cursor_word_right();
    assert_eq!(s.cursor(), 5, "第一次跳词应到 hello 末尾");
    s.cursor_word_right();
    assert_eq!(s.cursor(), 11, "第二次跳词应到 world 末尾");
    s.cursor_word_right();
    assert_eq!(s.cursor(), 15, "第三次跳词应到 foo 末尾");
    s.cursor_word_right();
    assert_eq!(s.cursor(), 15, "已在末尾不移动");
}

#[test]
fn test_delete_word_backward_basic() {
    let mut s = InputState::with_value("hello world".to_string());
    s.cursor_end();
    s.delete_word_backward();
    assert_eq!(s.value(), "hello ", "第一次删词应删掉 world");
    assert_eq!(s.cursor(), 6);
}

#[test]
fn test_delete_word_backward_empty() {
    let mut s = InputState::new();
    s.delete_word_backward();
    assert_eq!(s.value(), "");
    assert_eq!(s.cursor(), 0);
}

#[test]
fn test_delete_word_backward_mid_word() {
    let mut s = InputState::with_value("hello world".to_string());
    s.cursor = 3; // byte 3 = after "hel"
    s.delete_word_backward();
    assert_eq!(s.value(), "lo world", "应删掉 hel");
    assert_eq!(s.cursor(), 0);
}
