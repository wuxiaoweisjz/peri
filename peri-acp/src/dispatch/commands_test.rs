use super::commands::build_available_commands;
use peri_middlewares::skills::SkillMetadata;

#[test]
fn test_build_available_commands_includes_builtins() {
    let cmds = build_available_commands(&[]);
    // 至少 22 个内置命令
    assert!(cmds.len() >= 22, "至少 22 个内置命令");
    // 验证关键命令存在
    let names: Vec<&str> = cmds.iter().map(|c| c.name.as_ref()).collect();
    assert!(names.contains(&"help"), "help 命令应存在");
    assert!(names.contains(&"clear"), "clear 命令应存在");
    assert!(names.contains(&"compact"), "compact 命令应存在");
    assert!(names.contains(&"model"), "model 命令应存在");
}

#[test]
fn test_build_available_commands_includes_skills() {
    let skills = vec![
        SkillMetadata {
            name: "my-skill".into(),
            description: "My custom skill".into(),
        },
        SkillMetadata {
            name: "other".into(),
            description: "Other skill".into(),
        },
    ];
    let cmds = build_available_commands(&skills);
    let names: Vec<&str> = cmds.iter().map(|c| c.name.as_ref()).collect();
    assert!(names.contains(&"skill:my-skill"), "skill:my-skill 应存在");
    assert!(names.contains(&"skill:other"), "skill:other 应存在");
}

#[test]
fn test_build_available_commands_no_skills_only_builtins() {
    let cmds = build_available_commands(&[]);
    assert!(
        !cmds.iter().any(|c| c.name.as_ref().starts_with("skill:")),
        "无 skills 时不应包含 skill: 前缀命令"
    );
}
