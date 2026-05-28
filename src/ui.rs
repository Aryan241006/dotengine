use std::future::Future;
use std::io::{self, IsTerminal, Write};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::time::Duration;
use tokio::task::JoinHandle;

pub const RESET: &str = "\x1b[0m";
pub const OLIVE: &str = "\x1b[38;2;120;138;62m";
pub const DARK_OLIVE: &str = "\x1b[38;2;85;102;40m";

pub fn accent(text: &str) -> String {
    format!("{OLIVE}{text}{RESET}")
}

pub fn heading(text: &str) -> String {
    format!("\x1b[1;38;2;120;138;62m{text}{RESET}")
}

pub fn success(text: &str) -> String {
    format!("\x1b[38;2;166;227;161m✔ {text}{RESET}")
}

pub fn info(text: &str) -> String {
    format!("\x1b[38;2;120;138;62mℹ {text}{RESET}")
}

pub fn warning(text: &str) -> String {
    format!("\x1b[38;2;250;179;135m⚠ {text}{RESET}")
}

pub fn error(text: &str) -> String {
    format!("\x1b[38;2;243;139;168m✖ {text}{RESET}")
}

pub fn print_wordmark() {
    println!("\n  \x1b[1;38;2;120;138;62m⚡ dotengine\x1b[0m \x1b[38;2;85;102;40m| hyprland ai agent\x1b[0m\n");
}

struct ActivityIndicator {
    running: Arc<AtomicBool>,
    task: Option<JoinHandle<()>>,
}

impl ActivityIndicator {
    fn start(message: &str) -> Self {
        if !io::stderr().is_terminal() {
            return Self {
                running: Arc::new(AtomicBool::new(false)),
                task: None,
            };
        }

        let running = Arc::new(AtomicBool::new(true));
        let task_running = running.clone();
        let message = message.to_string();
        let task = tokio::spawn(async move {
            let mut frame_index = 0;
            let frames = [".  ", ".. ", "...", "   "];
            while task_running.load(Ordering::Relaxed) {
                eprint!(
                    "\r\x1b[2K\x1b[38;2;120;138;62mDotengine\x1b[0m {} \x1b[38;2;85;102;40m{}{RESET}",
                    message,
                    frames[frame_index]
                );
                let _ = io::stderr().flush();
                frame_index = (frame_index + 1) % frames.len();
                tokio::time::sleep(Duration::from_millis(220)).await;
            }
        });

        Self {
            running,
            task: Some(task),
        }
    }

    async fn finish(mut self) {
        self.running.store(false, Ordering::Relaxed);
        if let Some(task) = self.task.take() {
            let _ = task.await;
            eprint!("\r\x1b[2K");
            let _ = io::stderr().flush();
        }
    }
}

pub async fn activity<T, F>(message: &str, operation: F) -> T
where
    F: Future<Output = T>,
{
    let indicator = ActivityIndicator::start(message);
    let output = operation.await;
    indicator.finish().await;
    output
}
