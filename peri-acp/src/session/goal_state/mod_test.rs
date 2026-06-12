use super::*;
use peri_agent::goal::InMemoryGoalStore;
use std::sync::Arc;

fn make_state() -> GoalState {
    GoalState::new(
        Arc::new(InMemoryGoalStore::new()),
        "test-thread".to_string(),
    )
}

#[tokio::test]
async fn test_set_goal_写入_store_并触发_objective_updated() {
    let state = make_state();
    state
        .set_goal("完成模块重构".to_string(), Some(200_000))
        .await
        .unwrap();

    let snap = state.snapshot();
    assert_eq!(snap.objective.as_deref(), Some("完成模块重构"));
    assert_eq!(snap.token_budget, Some(200_000));
    assert_eq!(snap.status, Some(GoalStatus::Active));
    assert!(snap.objective_just_updated);
}

#[tokio::test]
async fn test_clear_清空_goal() {
    let state = make_state();
    state.set_goal("临时目标".to_string(), None).await.unwrap();
    state.clear().await.unwrap();

    let snap = state.snapshot();
    assert!(snap.objective.is_none());
    assert!(!snap.objective_just_updated);
}

#[tokio::test]
async fn test_set_goal_覆盖旧_goal_生成新_goal_id() {
    let state = make_state();
    state.set_goal("目标 A".to_string(), None).await.unwrap();
    let id_a = state.snapshot().goal_id.clone().unwrap();

    state.set_goal("目标 B".to_string(), None).await.unwrap();
    let id_b = state.snapshot().goal_id.clone().unwrap();

    assert_ne!(id_a, id_b);
    assert_eq!(state.snapshot().objective.as_deref(), Some("目标 B"));
}

#[tokio::test]
async fn test_store_写入失败_内存镜像仍可读() {
    use async_trait::async_trait;
    use peri_agent::goal::{GoalStore, GoalStoreError, ThreadGoal};

    struct FailingStore;
    #[async_trait]
    impl GoalStore for FailingStore {
        async fn save(&self, _: &str, _: ThreadGoal) -> Result<(), GoalStoreError> {
            Err(GoalStoreError::Io("simulated".to_string()))
        }
        async fn load(&self, _: &str) -> Result<Option<ThreadGoal>, GoalStoreError> {
            Err(GoalStoreError::Io("simulated".to_string()))
        }
        async fn delete(&self, _: &str) -> Result<(), GoalStoreError> {
            Err(GoalStoreError::Io("simulated".to_string()))
        }
    }

    let state = GoalState::new(Arc::new(FailingStore), "test-thread".to_string());
    // set_goal 即使 store 失败也不 panic（内存镜像更新成功）
    let result = state.set_goal("fallback".to_string(), None).await;
    // store 失败返回 Err，但内存镜像已更新
    assert!(result.is_err());
    assert_eq!(state.snapshot().objective.as_deref(), Some("fallback"));
}

#[tokio::test]
async fn test_set_status_合法转换_active_to_paused() {
    let state = make_state();
    state.set_goal("测试".to_string(), None).await.unwrap();
    assert_eq!(state.snapshot().status, Some(GoalStatus::Active));

    state.set_status(GoalStatus::Paused).await.unwrap();
    assert_eq!(state.snapshot().status, Some(GoalStatus::Paused));
}

#[tokio::test]
async fn test_set_status_非法转换_complete_to_active_返回错误() {
    let state = make_state();
    state.set_goal("测试".to_string(), None).await.unwrap();
    state.set_status(GoalStatus::Complete).await.unwrap();

    let result = state.set_status(GoalStatus::Active).await;
    assert!(result.is_err());
    // 状态未改变
    assert_eq!(state.snapshot().status, Some(GoalStatus::Complete));
}

#[tokio::test]
async fn test_set_status_blocked_必须附带_reason() {
    let state = make_state();
    state.set_goal("测试".to_string(), None).await.unwrap();

    let result = state.set_status(GoalStatus::Blocked).await;
    assert!(result.is_err(), "Blocked 必须附带 reason");

    state
        .set_status_with_reason(GoalStatus::Blocked, "缺少依赖".to_string())
        .await
        .unwrap();
    assert_eq!(state.snapshot().status, Some(GoalStatus::Blocked));
}

#[tokio::test]
async fn test_set_status_无_goal_返回错误() {
    let state = make_state();
    let result = state.set_status(GoalStatus::Paused).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_resume_from_complete_返回错误() {
    let state = make_state();
    state.set_goal("测试".to_string(), None).await.unwrap();
    state.set_status(GoalStatus::Complete).await.unwrap();

    // Complete 是终态，不能 resume
    let result = state.set_status(GoalStatus::Active).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_put_pending_user_message_覆盖旧值() {
    let state = make_state();
    state.set_goal("测试".to_string(), None).await.unwrap();

    state.put_pending_user_message("第一条".to_string());
    state.put_pending_user_message("第二条".to_string());

    let taken = state.take_pending_user_message();
    assert_eq!(taken.as_deref(), Some("第二条"));
    // take 后清空
    assert!(state.take_pending_user_message().is_none());
}

#[tokio::test]
async fn test_clear_goal_清零_pending_user_message() {
    let state = make_state();
    state.set_goal("测试".to_string(), None).await.unwrap();
    state.put_pending_user_message("待清空".to_string());

    state.clear().await.unwrap();
    assert!(state.take_pending_user_message().is_none());
}

#[tokio::test]
async fn test_set_status_complete_清零_pending_user_message() {
    let state = make_state();
    state.set_goal("测试".to_string(), None).await.unwrap();
    state.put_pending_user_message("待清空".to_string());

    state.set_status(GoalStatus::Complete).await.unwrap();
    assert!(state.take_pending_user_message().is_none());
}

#[tokio::test]
async fn test_set_status_paused_保留_pending_user_message() {
    let state = make_state();
    state.set_goal("测试".to_string(), None).await.unwrap();
    state.put_pending_user_message("保留".to_string());

    state.set_status(GoalStatus::Paused).await.unwrap();
    assert_eq!(state.take_pending_user_message().as_deref(), Some("保留"));
}
