use super::Command;
use crate::app::App;

pub struct CronCommand;

impl Command for CronCommand {
    fn name(&self) -> &str {
        "cron"
    }

    fn description(&self) -> &str {
        "查看和管理定时任务"
    }

    fn execute(&self, app: &mut App, _args: &str) {
        app.open_cron_panel();
    }
}
