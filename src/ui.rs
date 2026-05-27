use std::future::Future;
use std::io::{self, IsTerminal, Write};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::time::Duration;
use tokio::task::JoinHandle;

pub const RESET: &str = "\x1b[0m";
pub const INDIGO: &str = "\x1b[38;2;129;140;248m";
pub const BLUE: &str = "\x1b[38;2;125;207;255m";
pub const SOFT_BLUE: &str = "\x1b[38;2;137;180;250m";

pub fn accent(text: &str) -> String {
    format!("{INDIGO}{text}{RESET}")
}

pub fn heading(text: &str) -> String {
    format!("\x1b[1;38;2;137;180;250m{text}{RESET}")
}

pub fn success(text: &str) -> String {
    format!("\x1b[38;2;166;227;161m✔ {text}{RESET}")
}

pub fn info(text: &str) -> String {
    format!("\x1b[38;2;137;180;250mℹ {text}{RESET}")
}

pub fn warning(text: &str) -> String {
    format!("\x1b[38;2;250;179;135m⚠ {text}{RESET}")
}

pub fn error(text: &str) -> String {
    format!("\x1b[38;2;243;139;168m✖ {text}{RESET}")
}

pub fn print_wordmark() {
    let logo = r#"
      ____        __                  _             
     / __ \____  / /____  ____  ____ _(_)___  ___   
    / / / / __ \/ __/ _ \/ __ \/ __ `/ / __ \/ _ \  
   / /_/ / /_/ / /_/  __/ / / / /_/ / / / / /  __/  
  /_____/\____/\__/\___/_/ /_/\__, /_/_/ /_/\___/   
                             /____/                 
    "#;

    println!();
    let colors = [
        "\x1b[38;2;196;113;245m", // Deep Purple
        "\x1b[38;2;141;106;248m", // Violet
        "\x1b[38;2;86;108;250m",  // Indigo-Blue
        "\x1b[38;2;31;143;253m",  // Blue
        "\x1b[38;2;6;182;212m",   // Cyan
        "\x1b[38;2;20;184;166m",  // Teal
    ];
    
    for (line, color) in logo.lines().zip(colors.iter().cycle()) {
        if !line.trim().is_empty() {
            println!("  {color}{line}{RESET}");
        }
    }
    println!();
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
                    "\r\x1b[2K\x1b[38;2;129;140;248mDotengine\x1b[0m {} \x1b[38;2;137;180;250m{}{RESET}",
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
