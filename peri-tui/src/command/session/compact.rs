use crate::app::App;
use crate::command::Command;
use crate::i18n::LcRegistry;

pub struct CompactCommand;

impl Command for CompactCommand {
    fn name(&self) -> &str {
        "compact"
    }

    fn description(&self, _lc: &LcRegistry) -> String {
        _lc.tr("command-compact-description")
    }

    fn execute(&self, app: &mut App, _args: &str) {
        if app.session_mgr.sessions[app.session_mgr.active].ui.loading {
            app.session_mgr.sessions[app.session_mgr.active]
                .messages
                .view_messages
                .push(crate::app::MessageViewModel::system(
                    app.services.lc.tr("compact-agent-running"),
                ));
            return;
        }
        app.submit_message("/compact".to_string());
    }
}
