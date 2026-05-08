use super::Command;
use crate::app::App;

pub struct McpCommand;

impl Command for McpCommand {
    fn name(&self) -> &str {
        "mcp"
    }

    fn description(&self) -> &str {
        "管理 MCP 服务器连接"
    }

    fn execute(&self, app: &mut App, _args: &str) {
        app.open_mcp_panel();
    }
}
