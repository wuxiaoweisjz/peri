#[allow(unused_imports)]
fn make_tool(dir: &tempfile::TempDir) -> LineEditTool {
    LineEditTool::new(dir.path().to_str().unwrap())
}

fn make_edit(file: &str, start_line: usize, new_string: &str) -> serde_json::Value {
    serde_json::json!({
        "file_path": file,
        "start_line": start_line,
        "new_string": new_string
    })
}

// ─── action 枚举测试 ─────────────────────────────────────────────────────

#[tokio::test]
async fn test_action_replace_显式() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("f.txt"), "aaa\nbbb\nccc\n").unwrap();
    let tool = make_tool(&dir);
    tool.invoke(serde_json::json!({
        "edits": [{
            "file_path": "f.txt",
            "start_line": 2,
            "action": "replace",
            "new_string": "BBB"
        }]
    }))
    .await
    .unwrap();
    assert_eq!(
        std::fs::read_to_string(dir.path().join("f.txt")).unwrap(),
        "aaa\nBBB\nccc\n"
    );
}

#[tokio::test]
async fn test_action_insert_不删除旧行() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("f.txt"), "aaa\nbbb\n").unwrap();
    let tool = make_tool(&dir);
    let result = tool
        .invoke(serde_json::json!({
            "edits": [{
                "file_path": "f.txt",
                "start_line": 2,
                "action": "insert",
                "new_string": "xxx\nyyy"
            }]
        }))
        .await
        .unwrap();
    assert!(result.contains("insert"), "unexpected: {result}");
    assert_eq!(
        std::fs::read_to_string(dir.path().join("f.txt")).unwrap(),
        "aaa\nxxx\nyyy\nbbb\n"
    );
}

#[tokio::test]
async fn test_action_delete_忽略new_string() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("f.txt"), "aaa\nbbb\nccc\nddd\n").unwrap();
    let tool = make_tool(&dir);
    tool.invoke(serde_json::json!({
        "edits": [{
            "file_path": "f.txt",
            "start_line": 2,
            "end_line": 3,
            "action": "delete",
            "new_string": "ignored content"
        }]
    }))
    .await
    .unwrap();
    assert_eq!(
        std::fs::read_to_string(dir.path().join("f.txt")).unwrap(),
        "aaa\nddd\n"
    );
}

#[tokio::test]
async fn test_action_默认replace() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("f.txt"), "aaa\nbbb\n").unwrap();
    let tool = make_tool(&dir);
    tool.invoke(serde_json::json!({
        "edits": [make_edit("f.txt", 2, "BBB")]
    }))
    .await
    .unwrap();
    assert_eq!(
        std::fs::read_to_string(dir.path().join("f.txt")).unwrap(),
        "aaa\nBBB\n"
    );
}

#[tokio::test]
async fn test_action_默认delete_空new_string() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("f.txt"), "aaa\nbbb\nccc\n").unwrap();
    let tool = make_tool(&dir);
    tool.invoke(serde_json::json!({
        "edits": [make_edit("f.txt", 2, "")]
    }))
    .await
    .unwrap();
    assert_eq!(
        std::fs::read_to_string(dir.path().join("f.txt")).unwrap(),
        "aaa\nccc\n"
    );
}

// ─── expected_lines 验证测试 ──────────────────────────────────────────────

#[tokio::test]
async fn test_expected_lines_匹配() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("f.txt"), "aaa\nbbb\nccc\n").unwrap();
    let tool = make_tool(&dir);
    let result = tool
        .invoke(serde_json::json!({
            "edits": [{
                "file_path": "f.txt",
                "start_line": 2,
                "end_line": 2,
                "expected_lines": "bbb",
                "new_string": "BBB"
            }]
        }))
        .await
        .unwrap();
    assert!(result.contains("✓"), "应标记成功: {result}");
    assert!(!result.contains("不匹配"), "不应有不匹配警告: {result}");
    assert_eq!(
        std::fs::read_to_string(dir.path().join("f.txt")).unwrap(),
        "aaa\nBBB\nccc\n"
    );
}

#[tokio::test]
async fn test_expected_lines_不匹配_警告但执行() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("f.txt"), "aaa\nbbb\nccc\n").unwrap();
    let tool = make_tool(&dir);
    let result = tool
        .invoke(serde_json::json!({
            "edits": [{
                "file_path": "f.txt",
                "start_line": 2,
                "end_line": 2,
                "expected_lines": "wrong content",
                "new_string": "BBB"
            }]
        }))
        .await
        .unwrap();
    assert!(result.contains("⚠"), "应标记警告: {result}");
    assert!(result.contains("不匹配"), "应包含不匹配信息: {result}");
    assert!(result.contains("已执行"), "应继续执行: {result}");
    assert_eq!(
        std::fs::read_to_string(dir.path().join("f.txt")).unwrap(),
        "aaa\nBBB\nccc\n"
    );
}

#[tokio::test]
async fn test_expected_lines_尾部空白归一化() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("f.txt"), "aaa\nbbb   \nccc\n").unwrap();
    let tool = make_tool(&dir);
    let result = tool
        .invoke(serde_json::json!({
            "edits": [{
                "file_path": "f.txt",
                "start_line": 2,
                "expected_lines": "bbb",
                "new_string": "BBB"
            }]
        }))
        .await
        .unwrap();
    assert!(result.contains("✓"), "尾部空白应被归一化: {result}");
}

#[tokio::test]
async fn test_expected_lines_多行验证() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("f.txt"), "aaa\nbbb\nccc\nddd\n").unwrap();
    let tool = make_tool(&dir);
    let result = tool
        .invoke(serde_json::json!({
            "edits": [{
                "file_path": "f.txt",
                "start_line": 2,
                "end_line": 3,
                "expected_lines": "bbb\nccc",
                "new_string": "XXX"
            }]
        }))
        .await
        .unwrap();
    assert!(result.contains("✓"), "多行验证应通过: {result}");
    assert_eq!(
        std::fs::read_to_string(dir.path().join("f.txt")).unwrap(),
        "aaa\nXXX\nddd\n"
    );
}

// ─── 原子性测试 ──────────────────────────────────────────────────────────

#[tokio::test]
async fn test_原子性_全有或全无() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("f.txt"), "aaa\nbbb\nccc\n").unwrap();
    let tool = make_tool(&dir);
    let result = tool
        .invoke(serde_json::json!({
            "edits": [
                {"file_path": "f.txt", "start_line": 1, "new_string": "AAA"},
                {"file_path": "f.txt", "start_line": 99, "new_string": "XXX"}
            ]
        }))
        .await
        .unwrap();
    assert!(result.contains("✗"), "应报告失败: {result}");
    assert!(result.contains("未执行任何编辑"), "应声明未执行: {result}");
    assert_eq!(
        std::fs::read_to_string(dir.path().join("f.txt")).unwrap(),
        "aaa\nbbb\nccc\n",
        "原子性：文件不应被修改"
    );
}

#[tokio::test]
async fn test_原子性_跨文件() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.txt"), "aaa\n").unwrap();
    std::fs::write(dir.path().join("b.txt"), "bbb\n").unwrap();
    let tool = make_tool(&dir);
    let result = tool
        .invoke(serde_json::json!({
            "edits": [
                {"file_path": "a.txt", "start_line": 1, "new_string": "AAA"},
                {"file_path": "b.txt", "start_line": 99, "new_string": "XXX"}
            ]
        }))
        .await
        .unwrap();
    assert!(result.contains("✗"), "应报告失败: {result}");
    assert_eq!(
        std::fs::read_to_string(dir.path().join("a.txt")).unwrap(),
        "aaa\n",
        "跨文件原子性：a.txt 不应被修改"
    );
    assert_eq!(
        std::fs::read_to_string(dir.path().join("b.txt")).unwrap(),
        "bbb\n",
        "跨文件原子性：b.txt 不应被修改"
    );
}

// ─── 重叠检测测试 ─────────────────────────────────────────────────────────

#[tokio::test]
async fn test_重叠检测_拒绝() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("f.txt"), "a\nb\nc\nd\ne\n").unwrap();
    let tool = make_tool(&dir);
    let result = tool
        .invoke(serde_json::json!({
            "edits": [
                {"file_path": "f.txt", "start_line": 2, "end_line": 4, "new_string": "X"},
                {"file_path": "f.txt", "start_line": 3, "end_line": 5, "new_string": "Y"}
            ]
        }))
        .await
        .unwrap();
    assert!(result.contains("重叠"), "应报告重叠: {result}");
    assert_eq!(
        std::fs::read_to_string(dir.path().join("f.txt")).unwrap(),
        "a\nb\nc\nd\ne\n",
        "重叠时文件不应被修改"
    );
}

#[tokio::test]
async fn test_重叠_不同文件不检测() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.txt"), "aaa\n").unwrap();
    std::fs::write(dir.path().join("b.txt"), "bbb\n").unwrap();
    let tool = make_tool(&dir);
    let result = tool
        .invoke(serde_json::json!({
            "edits": [
                {"file_path": "a.txt", "start_line": 1, "end_line": 1, "new_string": "AAA"},
                {"file_path": "b.txt", "start_line": 1, "end_line": 1, "new_string": "BBB"}
            ]
        }))
        .await
        .unwrap();
    assert!(!result.contains("✗"), "不应有失败: {result}");
}

#[tokio::test]
async fn test_重叠_首尾相接不算重叠() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("f.txt"), "a\nb\nc\nd\n").unwrap();
    let tool = make_tool(&dir);
    let result = tool
        .invoke(serde_json::json!({
            "edits": [
                {"file_path": "f.txt", "start_line": 1, "end_line": 2, "new_string": "X"},
                {"file_path": "f.txt", "start_line": 3, "end_line": 4, "new_string": "Y"}
            ]
        }))
        .await
        .unwrap();
    assert!(!result.contains("✗"), "首尾相接不应报重叠: {result}");
    assert_eq!(
        std::fs::read_to_string(dir.path().join("f.txt")).unwrap(),
        "X\nY\n"
    );
}

#[tokio::test]
async fn test_重叠_insert不参与重叠检测() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("f.txt"), "a\nb\nc\n").unwrap();
    let tool = make_tool(&dir);
    let result = tool
        .invoke(serde_json::json!({
            "edits": [
                {"file_path": "f.txt", "start_line": 2, "end_line": 2, "new_string": "X"},
                {"file_path": "f.txt", "start_line": 2, "action": "insert", "new_string": "INSERTED"}
            ]
        }))
        .await
        .unwrap();
    assert!(!result.contains("重叠"), "insert 不参与重叠检测: {result}");
}

// ─── 反馈格式测试 ──────────────────────────────────────────────────────────

#[tokio::test]
async fn test_反馈_含上下文diff() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("f.txt"), "line1\nline2\nline3\nline4\nline5\n").unwrap();
    let tool = make_tool(&dir);
    let result = tool
        .invoke(serde_json::json!({
            "edits": [{
                "file_path": "f.txt",
                "start_line": 3,
                "new_string": "LINE3"
            }]
        }))
        .await
        .unwrap();
    assert!(result.contains("✓"), "应标记成功: {result}");
    assert!(result.contains("|-"), "应包含旧行标记: {result}");
    assert!(result.contains("|+"), "应包含新行标记: {result}");
}

// ─── 基础操作测试 ──────────────────────────────────────────────────────────

#[tokio::test]
async fn test_替换多行() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("f.txt"), "aaa\nbbb\nccc\nddd\n").unwrap();
    let tool = make_tool(&dir);
    tool.invoke(serde_json::json!({
        "edits": [{
            "file_path": "f.txt",
            "start_line": 2,
            "end_line": 3,
            "new_string": "XXX"
        }]
    }))
    .await
    .unwrap();
    assert_eq!(
        std::fs::read_to_string(dir.path().join("f.txt")).unwrap(),
        "aaa\nXXX\nddd\n"
    );
}

#[tokio::test]
async fn test_insert_在行首() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("f.txt"), "aaa\nbbb\n").unwrap();
    let tool = make_tool(&dir);
    tool.invoke(serde_json::json!({
        "edits": [{
            "file_path": "f.txt",
            "start_line": 1,
            "action": "insert",
            "new_string": "NEW"
        }]
    }))
    .await
    .unwrap();
    assert_eq!(
        std::fs::read_to_string(dir.path().join("f.txt")).unwrap(),
        "NEW\naaa\nbbb\n"
    );
}

#[tokio::test]
async fn test_insert_在末尾() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("f.txt"), "aaa\nbbb\n").unwrap();
    let tool = make_tool(&dir);
    tool.invoke(serde_json::json!({
        "edits": [{
            "file_path": "f.txt",
            "start_line": 3,
            "action": "insert",
            "new_string": "APPENDED"
        }]
    }))
    .await
    .unwrap();
    assert_eq!(
        std::fs::read_to_string(dir.path().join("f.txt")).unwrap(),
        "aaa\nbbb\nAPPENDED\n"
    );
}

#[tokio::test]
async fn test_替换整个文件() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("f.txt"), "aaa\nbbb\nccc\n").unwrap();
    let tool = make_tool(&dir);
    tool.invoke(serde_json::json!({
        "edits": [{
            "file_path": "f.txt",
            "start_line": 1,
            "end_line": 3,
            "new_string": "brand new"
        }]
    }))
    .await
    .unwrap();
    assert_eq!(
        std::fs::read_to_string(dir.path().join("f.txt")).unwrap(),
        "brand new\n"
    );
}

#[tokio::test]
async fn test_删除所有行() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("f.txt"), "aaa\nbbb\n").unwrap();
    let tool = make_tool(&dir);
    tool.invoke(serde_json::json!({
        "edits": [{
            "file_path": "f.txt",
            "start_line": 1,
            "end_line": 2,
            "action": "delete",
            "new_string": ""
        }]
    }))
    .await
    .unwrap();
    assert_eq!(
        std::fs::read_to_string(dir.path().join("f.txt")).unwrap(),
        ""
    );
}

#[tokio::test]
async fn test_空文件插入() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("f.txt"), "").unwrap();
    let tool = make_tool(&dir);
    tool.invoke(serde_json::json!({
        "edits": [{
            "file_path": "f.txt",
            "start_line": 1,
            "action": "insert",
            "new_string": "hello"
        }]
    }))
    .await
    .unwrap();
    assert_eq!(
        std::fs::read_to_string(dir.path().join("f.txt")).unwrap(),
        "hello"
    );
}

// ─── 多编辑测试 ──────────────────────────────────────────────────────────

#[tokio::test]
async fn test_同文件多编辑从后往前() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("f.txt"), "a\nb\nc\nd\ne\n").unwrap();
    let tool = make_tool(&dir);
    tool.invoke(serde_json::json!({
        "edits": [
            {"file_path": "f.txt", "start_line": 2, "new_string": "BBB"},
            {"file_path": "f.txt", "start_line": 4, "new_string": "DDD"}
        ]
    }))
    .await
    .unwrap();
    assert_eq!(
        std::fs::read_to_string(dir.path().join("f.txt")).unwrap(),
        "a\nBBB\nc\nDDD\ne\n"
    );
}

#[tokio::test]
async fn test_跨文件多编辑() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.txt"), "aaa\n").unwrap();
    std::fs::write(dir.path().join("b.txt"), "bbb\n").unwrap();
    let tool = make_tool(&dir);
    tool.invoke(serde_json::json!({
        "edits": [
            {"file_path": "a.txt", "start_line": 1, "new_string": "AAA"},
            {"file_path": "b.txt", "start_line": 1, "new_string": "BBB"}
        ]
    }))
    .await
    .unwrap();
    assert_eq!(std::fs::read_to_string(dir.path().join("a.txt")).unwrap(), "AAA\n");
    assert_eq!(std::fs::read_to_string(dir.path().join("b.txt")).unwrap(), "BBB\n");
}

// ─── 错误处理测试 ──────────────────────────────────────────────────────────

#[tokio::test]
async fn test_行号超出范围() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("f.txt"), "aaa\n").unwrap();
    let tool = make_tool(&dir);
    let result = tool.invoke(serde_json::json!({
        "edits": [make_edit("f.txt", 99, "xxx")]
    })).await.unwrap();
    assert!(result.contains("✗"), "应标记失败: {result}");
    assert!(result.contains("超出"), "应报告超出范围: {result}");
    assert_eq!(
        std::fs::read_to_string(dir.path().join("f.txt")).unwrap(),
        "aaa\n",
        "文件不应被修改"
    );
}

#[tokio::test]
async fn test_文件不存在() {
    let dir = tempfile::tempdir().unwrap();
    let tool = make_tool(&dir);
    let result = tool.invoke(serde_json::json!({
        "edits": [make_edit("ghost.txt", 1, "xxx")]
    })).await;
    let err = result.unwrap_err();
    assert!(err.to_string().contains("不存在"), "应报文件不存在: {err}");
}

#[tokio::test]
async fn test_end_line小于start_line() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("f.txt"), "aaa\nbbb\n").unwrap();
    let tool = make_tool(&dir);
    let result = tool.invoke(serde_json::json!({
        "edits": [{
            "file_path": "f.txt",
            "start_line": 3,
            "end_line": 1,
            "new_string": "xxx"
        }]
    }))
    .await
    .unwrap();
    assert!(result.contains("✗"), "应标记失败: {result}");
    assert_eq!(
        std::fs::read_to_string(dir.path().join("f.txt")).unwrap(),
        "aaa\nbbb\n",
        "文件不应被修改"
    );
}

#[tokio::test]
async fn test_start_line为零() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("f.txt"), "aaa\n").unwrap();
    let tool = make_tool(&dir);
    let result = tool.invoke(serde_json::json!({
        "edits": [{
            "file_path": "f.txt",
            "start_line": 0,
            "new_string": "xxx"
        }]
    }))
    .await
    .unwrap();
    assert!(result.contains("✗"), "应标记失败: {result}");
}

// ─── CRLF 换行符保留测试 ──────────────────────────────────────────────────

#[tokio::test]
async fn test_crlf_替换保留() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("f.txt"), "aaa\r\nbbb\r\nccc\r\n").unwrap();
    let tool = make_tool(&dir);
    tool.invoke(serde_json::json!({
        "edits": [make_edit("f.txt", 2, "BBB")]
    }))
    .await
    .unwrap();
    assert_eq!(
        std::fs::read_to_string(dir.path().join("f.txt")).unwrap(),
        "aaa\r\nBBB\r\nccc\r\n"
    );
}

#[tokio::test]
async fn test_crlf_insert保留() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("f.txt"), "aaa\r\nbbb\r\n").unwrap();
    let tool = make_tool(&dir);
    tool.invoke(serde_json::json!({
        "edits": [{
            "file_path": "f.txt",
            "start_line": 2,
            "action": "insert",
            "new_string": "xxx\nyyy"
        }]
    }))
    .await
    .unwrap();
    assert_eq!(
        std::fs::read_to_string(dir.path().join("f.txt")).unwrap(),
        "aaa\r\nxxx\r\nyyy\r\nbbb\r\n"
    );
}

#[tokio::test]
async fn test_crlf_delete保留() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("f.txt"), "aaa\r\nbbb\r\nccc\r\n").unwrap();
    let tool = make_tool(&dir);
    tool.invoke(serde_json::json!({
        "edits": [make_edit("f.txt", 2, "")]
    }))
    .await
    .unwrap();
    assert_eq!(
        std::fs::read_to_string(dir.path().join("f.txt")).unwrap(),
        "aaa\r\nccc\r\n"
    );
}

#[tokio::test]
async fn test_lf_不受影响() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("f.txt"), "aaa\nbbb\nccc\n").unwrap();
    let tool = make_tool(&dir);
    tool.invoke(serde_json::json!({
        "edits": [make_edit("f.txt", 2, "BBB")]
    }))
    .await
    .unwrap();
    assert_eq!(
        std::fs::read_to_string(dir.path().join("f.txt")).unwrap(),
        "aaa\nBBB\nccc\n"
    );
}
