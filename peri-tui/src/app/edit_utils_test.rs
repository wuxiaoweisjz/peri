use super::*;

// ─── find_word_start ──────────────────────────────────────────────────

#[test]
fn test_find_word_start_at_start() {
    let chars: Vec<char> = "hello world".chars().collect();
    // cursor=0，已经在开头
    assert_eq!(find_word_start(&chars, 0), 0);
}

#[test]
fn test_find_word_start_inside_word() {
    let chars: Vec<char> = "hello world".chars().collect();
    // cursor=4（'o'），word 起始位置是 0
    assert_eq!(find_word_start(&chars, 4), 0);
}

#[test]
fn test_find_word_start_after_space() {
    let chars: Vec<char> = "hello world".chars().collect();
    // cursor=6（'w'），向前跳过空格→回到 'o'→同类回扫到 word 起始 0
    assert_eq!(find_word_start(&chars, 6), 0);
}

#[test]
fn test_find_word_start_trailing_spaces() {
    let chars: Vec<char> = "foo   ".chars().collect();
    // cursor=6（末尾空格后），向前跳过空格→ foo 末尾=3，word 起始=0
    assert_eq!(find_word_start(&chars, 6), 0);
}

#[test]
fn test_find_word_start_single_char_word() {
    let chars: Vec<char> = "a b c".chars().collect();
    // cursor=3（'b'），向前：blank→跳过，'a'→不同类别→停在 2
    assert_eq!(find_word_start(&chars, 3), 2);
}

#[test]
fn test_find_word_start_cjk() {
    let chars: Vec<char> = "你好 世界".chars().collect();
    // cursor=3（'世'，空白之后），向前跳过空格→回到 '好'→同类回扫到 0
    assert_eq!(find_word_start(&chars, 3), 0);
}

// ─── find_word_end ────────────────────────────────────────────────────

#[test]
fn test_find_word_end_at_end() {
    let chars: Vec<char> = "hello".chars().collect();
    assert_eq!(find_word_end(&chars, 5), 5);
}

#[test]
fn test_find_word_end_inside_word() {
    let chars: Vec<char> = "hello world".chars().collect();
    // cursor=0，向后：h/e/l/l/o 同类→空格→停在 5
    assert_eq!(find_word_end(&chars, 0), 5);
}

#[test]
fn test_find_word_end_skipping_spaces() {
    let chars: Vec<char> = "hello   world".chars().collect();
    // cursor=7（在空格区域），向后跳过空格→w=8→word end=13
    assert_eq!(find_word_end(&chars, 7), 13);
}

// ─── handle_edit_key word jumps ───────────────────────────────────────

use tui_textarea::{Input, Key};

fn make_input(key: Key, ctrl: bool, alt: bool) -> Input {
    Input {
        key,
        ctrl,
        alt,
        shift: false,
    }
}

#[test]
fn test_handle_edit_key_ctrl_left_word_jump() {
    let mut buf = "hello world foo".to_string();
    let mut cursor = 13; // 末尾
                         // Ctrl+Left 一次：跳到 'f'（第三个 word 起始=12）
    assert!(handle_edit_key(
        &mut buf,
        &mut cursor,
        make_input(Key::Left, true, false)
    ));
    assert_eq!(cursor, 12);
    // Ctrl+Left 一次：跳到 'w'（第二个 word 起始=6）
    assert!(handle_edit_key(
        &mut buf,
        &mut cursor,
        make_input(Key::Left, true, false)
    ));
    assert_eq!(cursor, 6);
    // Ctrl+Left 一次：跳到 'h'（第一个 word 起始=0）
    assert!(handle_edit_key(
        &mut buf,
        &mut cursor,
        make_input(Key::Left, true, false)
    ));
    assert_eq!(cursor, 0);
    // 再按不动
    assert!(handle_edit_key(
        &mut buf,
        &mut cursor,
        make_input(Key::Left, true, false)
    ));
    assert_eq!(cursor, 0);
}

#[test]
fn test_handle_edit_key_ctrl_right_word_jump() {
    let mut buf = "hello world foo".to_string();
    let mut cursor = 0;
    // Ctrl+Right 一次：跳到 'h' 所在 word 结束 = 5
    assert!(handle_edit_key(
        &mut buf,
        &mut cursor,
        make_input(Key::Right, true, false)
    ));
    assert_eq!(cursor, 5);
    // Ctrl+Right 一次：跳过空格→ 'w' word 结束 = 11
    assert!(handle_edit_key(
        &mut buf,
        &mut cursor,
        make_input(Key::Right, true, false)
    ));
    assert_eq!(cursor, 11);
    // Ctrl+Right 一次：跳过空格→ 'f' word 结束 = 15
    assert!(handle_edit_key(
        &mut buf,
        &mut cursor,
        make_input(Key::Right, true, false)
    ));
    assert_eq!(cursor, 15);
}

#[test]
fn test_handle_edit_key_ctrl_w_delete_word() {
    let mut buf = "hello world".to_string();
    let mut cursor = 11; // 末尾
    assert!(handle_edit_key(
        &mut buf,
        &mut cursor,
        make_input(Key::Char('w'), true, false)
    ));
    // 删除 "world"（保留前导空格），剩下 "hello "
    assert_eq!(buf, "hello ");
    assert_eq!(cursor, 6);

    // 再删一次，删除空格后剩余 "hello"
    assert!(handle_edit_key(
        &mut buf,
        &mut cursor,
        make_input(Key::Char('w'), true, false)
    ));
    // 空格作为独立 word：find_word_start(chars, 6) → 跳过空格到 'o' → 回扫到 0
    // 删除 chars[0..6] = "hello "
    assert_eq!(buf, "");
    assert_eq!(cursor, 0);

    // 空字符串：不 panic
    assert!(handle_edit_key(
        &mut buf,
        &mut cursor,
        make_input(Key::Char('w'), true, false)
    ));
    assert_eq!(buf, "");
}

#[test]
fn test_handle_edit_key_ctrl_w_middle_of_word() {
    let mut buf = "hello world".to_string();
    let mut cursor = 8; // 'r'（"world" 中第三个字符）
    assert!(handle_edit_key(
        &mut buf,
        &mut cursor,
        make_input(Key::Char('w'), true, false)
    ));
    // 删除 "wo"，剩下 "hello rld"
    assert_eq!(buf, "hello rld");
    assert_eq!(cursor, 6);
}

#[test]
fn test_handle_edit_key_alt_backspace() {
    let mut buf = "hello world".to_string();
    let mut cursor = 11; // 末尾
    assert!(handle_edit_key(
        &mut buf,
        &mut cursor,
        make_input(Key::Backspace, false, true)
    ));
    // 删除 "world"，剩下 "hello "（前导空格保留）
    assert_eq!(buf, "hello ");
    assert_eq!(cursor, 6);
}

#[test]
fn test_handle_edit_key_ctrl_left_cjk() {
    let mut buf = "你好 世界 foo".to_string();
    let mut cursor = buf.chars().count(); // 末尾
                                          // Ctrl+Left 一次：跳到 'f'
    assert!(handle_edit_key(
        &mut buf,
        &mut cursor,
        make_input(Key::Left, true, false)
    ));
    assert_eq!(cursor, 6); // "你好 世界 " 之后
                           // Ctrl+Left 一次：跳到 '世'
    assert!(handle_edit_key(
        &mut buf,
        &mut cursor,
        make_input(Key::Left, true, false)
    ));
    assert_eq!(cursor, 3); // "你好 " 之后
                           // Ctrl+Left 一次：跳到 '你'
    assert!(handle_edit_key(
        &mut buf,
        &mut cursor,
        make_input(Key::Left, true, false)
    ));
    assert_eq!(cursor, 0);
}
