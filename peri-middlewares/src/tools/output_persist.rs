use std::env;
use std::fs;
use std::path::PathBuf;

/// 当输出被截断时，将完整内容写入临时文件。
/// 返回追加到截断信息后的提示字符串。
/// 文件路径：`{temp_dir}/peri-tool-output-{uuid}.txt`
pub fn persist_truncated_output(full_content: &str) -> String {
    let id = uuid::Uuid::new_v4();
    let dir = env::temp_dir();
    let file_name = format!("peri-tool-output-{id}.txt");
    let file_path: PathBuf = dir.join(&file_name);

    match fs::write(&file_path, full_content) {
        Ok(_) => format!(
            "\n\n[完整输出已保存至 {} —— 使用 Read 工具查看完整内容]",
            file_path.display()
        ),
        Err(e) => format!("\n\n[保存完整输出至 {} 失败：{e}]", file_path.display()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_persist_writes_file_and_returns_hint() {
        let content = "line1\nline2\nline3";
        let hint = persist_truncated_output(content);
        // 提示应包含文件名
        assert!(
            hint.contains("peri-tool-output-"),
            "hint should contain filename: {hint}"
        );
        // 提示应引导用户使用 Read 工具
        assert!(
            hint.contains("Read"),
            "hint should guide to use Read tool: {hint}"
        );
        // 从提示中提取文件路径并验证内容
        let path_start = hint.find('/').expect("hint should contain path");
        let path_end = hint[path_start..]
            .find(' ')
            .map(|i| path_start + i)
            .unwrap_or(hint.len());
        let path = &hint[path_start..path_end];
        let saved = fs::read_to_string(path).unwrap();
        assert_eq!(saved, content);
        fs::remove_file(path).ok();
    }

    #[test]
    fn test_persist_empty_string() {
        let hint = persist_truncated_output("");
        // 空内容也应生成提示
        assert!(
            hint.contains("Read"),
            "empty content should also produce hint: {hint}"
        );
    }
}
