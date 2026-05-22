//! Load session messages from ThreadStore.

use peri_agent::messages::BaseMessage;
use peri_agent::thread::{ThreadId, ThreadStore};

/// Load message history for a session thread.
///
/// Returns an empty `Vec` if the thread does not exist (with a warning log).
pub async fn load_session_messages(
    thread_store: &dyn ThreadStore,
    thread_id: &str,
) -> Vec<BaseMessage> {
    match thread_store
        .load_messages(&ThreadId::from(thread_id.to_string()))
        .await
    {
        Ok(msgs) => msgs,
        Err(e) => {
            tracing::warn!(thread_id = %thread_id, error = %e, "session/load: thread not found, returning empty history");
            Vec::new()
        }
    }
}
