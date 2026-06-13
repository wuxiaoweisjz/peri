use crate::filter::{detect_package_items, extract_skill_name, filter_items};
use crate::resolver::PackageType;
use crate::types::DependencySpec;

#[test]
fn test_filter_simple_passes_through() {
    let spec = DependencySpec::Simple("abc123".into());
    let items = vec![("interview".into(), "skills/interview/SKILL.md".into())];
    let out = filter_items(&items, &spec).unwrap();
    assert_eq!(out.len(), 1);
}

#[test]
fn test_filter_pick_by_name() {
    let spec = DependencySpec::Detailed {
        version: "abc123".into(),
        pick: vec!["grill-*".into()],
        omit: vec![],
    };
    let items = vec![
        ("grill-me".into(), "skills/grill-me/SKILL.md".into()),
        ("interview".into(), "skills/interview/SKILL.md".into()),
    ];
    let out = filter_items(&items, &spec).unwrap();
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].0, "grill-me");
}

#[test]
fn test_filter_omit_by_path() {
    let spec = DependencySpec::Detailed {
        version: "abc123".into(),
        pick: vec![],
        omit: vec!["skills/test/**".into()],
    };
    let items = vec![
        ("grill-me".into(), "skills/grill-me/SKILL.md".into()),
        ("foo".into(), "skills/test/foo/SKILL.md".into()),
    ];
    let out = filter_items(&items, &spec).unwrap();
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].0, "grill-me");
}

#[test]
fn test_filter_pick_and_omit() {
    let spec = DependencySpec::Detailed {
        version: "abc123".into(),
        pick: vec!["skill-*".into()],
        omit: vec!["skill-test".into()],
    };
    let items = vec![
        ("skill-a".into(), "skills/skill-a/SKILL.md".into()),
        ("skill-test".into(), "skills/skill-test/SKILL.md".into()),
        ("other".into(), "skills/other/SKILL.md".into()),
    ];
    let out = filter_items(&items, &spec).unwrap();
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].0, "skill-a");
}

#[test]
fn test_filter_invalid_glob_errors() {
    let spec = DependencySpec::Detailed {
        version: "abc123".into(),
        pick: vec!["[invalid".into()],
        omit: vec![],
    };
    let items = vec![("a".into(), "skills/a/SKILL.md".into())];
    assert!(filter_items(&items, &spec).is_err());
}

#[test]
fn test_detect_package_items_from_manifest() {
    let tmp = tempfile::TempDir::new().unwrap();
    let store = tmp.path();
    std::fs::write(
        store.join("agm.package.json"),
        r#"{
            "name": "pkg",
            "version": "1.0.0",
            "skills": [
                ".claude/skills/interview/SKILL.md",
                ".claude/skills/grill-me/SKILL.md"
            ],
            "agents": [".claude/agents/my-agent.md"],
            "mcp": []
        }"#,
    )
    .unwrap();

    let skills = detect_package_items(store, PackageType::Skills, "pkg").unwrap();
    assert_eq!(skills.len(), 2);
    assert!(skills.iter().any(|(n, _)| n == "interview"));
    assert!(skills.iter().any(|(n, _)| n == "grill-me"));

    let agents = detect_package_items(store, PackageType::Agents, "pkg").unwrap();
    assert_eq!(agents.len(), 1);
    assert_eq!(agents[0].0, "my-agent.md");
}

#[test]
fn test_detect_package_items_auto_detect_skills() {
    let tmp = tempfile::TempDir::new().unwrap();
    let skill_dir = tmp.path().join(".claude").join("skills").join("foo");
    std::fs::create_dir_all(&skill_dir).unwrap();
    std::fs::write(skill_dir.join("SKILL.md"), "# Foo").unwrap();

    let skills = detect_package_items(tmp.path(), PackageType::Skills, "pkg").unwrap();
    assert_eq!(skills.len(), 1);
    assert_eq!(skills[0].0, "foo");
}

#[test]
fn test_extract_skill_name_from_glob() {
    assert_eq!(
        extract_skill_name(".claude/skills/interview/SKILL.md"),
        "interview"
    );
    assert_eq!(
        extract_skill_name(".claude/agents/my-agent.md"),
        "my-agent.md"
    );
    assert_eq!(extract_skill_name(".claude/mcp/my-mcp/SKILL.md"), "my-mcp");
    assert_eq!(extract_skill_name("skills/grill-me/SKILL.md"), "grill-me");
}
