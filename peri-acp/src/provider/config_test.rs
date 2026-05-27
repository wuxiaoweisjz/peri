use super::*;
use std::collections::HashMap;

fn make_global() -> AppConfig {
    AppConfig {
        active_alias: "sonnet".to_string(),
        active_provider_id: "openai-1".to_string(),
        providers: vec![ProviderConfig {
            id: "openai-1".to_string(),
            provider_type: "openai".to_string(),
            api_key: "sk-global".to_string(),
            ..Default::default()
        }],
        thinking: Some(ThinkingConfig {
            enabled: true,
            budget_tokens: 8000,
            effort: "medium".to_string(),
            max_tokens: 32000,
        }),
        language: Some("zh".to_string()),
        diff_enabled: true,
        ..Default::default()
    }
}

#[test]
fn test_merge_workspace_empty_changes_nothing() {
    let mut global = make_global();
    let workspace = AppConfig::default();
    global.merge_overrides(workspace);
    assert_eq!(global.active_alias, "sonnet");
    assert_eq!(global.providers.len(), 1);
    assert!(global.thinking.is_some());
    // diff_enabled: bool 直接覆盖，default 为 false
    assert!(!global.diff_enabled);
}

#[test]
fn test_merge_workspace_complete_overrides_all() {
    let mut global = make_global();
    let workspace = AppConfig {
        active_alias: "opus".to_string(),
        active_provider_id: "anthro-1".to_string(),
        providers: vec![ProviderConfig {
            id: "anthro-1".to_string(),
            provider_type: "anthropic".to_string(),
            api_key: "sk-ws".to_string(),
            ..Default::default()
        }],
        language: Some("en".to_string()),
        diff_enabled: false,
        ..Default::default()
    };
    global.merge_overrides(workspace);
    assert_eq!(global.active_alias, "opus");
    assert_eq!(global.active_provider_id, "anthro-1");
    assert_eq!(global.providers.len(), 1);
    assert_eq!(global.providers[0].provider_type, "anthropic");
    assert_eq!(global.language, Some("en".to_string()));
    assert!(!global.diff_enabled);
    assert!(global.thinking.is_some());
}

#[test]
fn test_merge_providers_empty_array_does_not_override() {
    let mut global = make_global();
    let workspace = AppConfig {
        providers: vec![],
        ..Default::default()
    };
    global.merge_overrides(workspace);
    assert_eq!(global.providers.len(), 1);
    assert_eq!(global.providers[0].api_key, "sk-global");
}

#[test]
fn test_merge_single_field_override() {
    let mut global = make_global();
    let workspace = AppConfig {
        active_alias: "haiku".to_string(),
        ..Default::default()
    };
    global.merge_overrides(workspace);
    assert_eq!(global.active_alias, "haiku");
    assert_eq!(global.providers.len(), 1);
    assert_eq!(global.providers[0].api_key, "sk-global");
}

#[test]
fn test_merge_env_override() {
    let mut global = AppConfig {
        env: Some(HashMap::from([("FOO".to_string(), "bar".to_string())])),
        ..make_global()
    };
    let workspace = AppConfig {
        env: Some(HashMap::from([("BAZ".to_string(), "qux".to_string())])),
        ..Default::default()
    };
    global.merge_overrides(workspace);
    let env = global.env.unwrap();
    assert!(!env.contains_key("FOO"));
    assert_eq!(env.get("BAZ"), Some(&"qux".to_string()));
}

#[test]
fn test_merge_diff_enabled_false_overrides_global_true() {
    let mut global = make_global(); // diff_enabled: true
    let workspace = AppConfig {
        diff_enabled: false,
        ..Default::default()
    };
    global.merge_overrides(workspace);
    assert!(!global.diff_enabled);
}
