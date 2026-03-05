use crate::widgets::WidgetModule;
use std::path::Path;

/// Scan the host and current directory, returning widget modules that apply.
pub fn discover() -> Vec<Box<dyn WidgetModule>> {
    let mut modules: Vec<Box<dyn WidgetModule>> = Vec::new();

    // ── Always load ──
    modules.push(Box::new(crate::widgets::system::SystemWidget::new()));
    modules.push(Box::new(crate::widgets::processes::ProcessesWidget::new()));
    modules.push(Box::new(crate::widgets::disk::DiskWidget::new()));
    modules.push(Box::new(crate::widgets::network::NetworkWidget::new()));

    // ── Git — only if .git exists in cwd ──
    if Path::new(".git").exists() {
        modules.push(Box::new(crate::widgets::git::GitWidget::new()));
    }

    // ── Docker — only if daemon socket is accessible ──
    if docker_available() {
        modules.push(Box::new(crate::widgets::docker::DockerWidget::new()));
    }

    // ── Listening ports — always useful for devs ──
    modules.push(Box::new(crate::widgets::ports::PortsWidget::new()));

    // Init all
    for m in modules.iter_mut() {
        m.init();
    }

    modules
}

fn docker_available() -> bool {
    Path::new("/var/run/docker.sock").exists()
        || std::env::var("DOCKER_HOST").is_ok()
}
