use crate::domain::ImagePayload;
use crate::ui::{accent, heading};
use std::io::{self, IsTerminal, Read, Write};
use std::process::{Command, Stdio};

// Box drawing and sizing is dynamic based on terminal width.

pub struct Composition {
    pub instruction: String,
    pub images: Vec<ImagePayload>,
}

pub fn compose(
    initial_text: String,
    initial_images: Vec<ImagePayload>,
    version: String,
    provider: String,
) -> Result<Composition, Box<dyn std::error::Error + Send + Sync>> {
    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        return compose_line_mode(initial_text, initial_images, version, provider);
    }

    Composer::new(initial_text, initial_images, version, provider).run()
}

fn compose_line_mode(
    initial_text: String,
    images: Vec<ImagePayload>,
    version: String,
    provider: String,
) -> Result<Composition, Box<dyn std::error::Error + Send + Sync>> {
    println!(
        "\x1b[1;38;2;120;138;62mDOTENGINE\x1b[0m (v{}) | Provider: {}",
        version, provider
    );
    println!("{}", heading("Dotengine prompt"));
    if !initial_text.is_empty() {
        println!("Suggested prompt: {}", initial_text);
    }
    print!("> ");
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let instruction = if input.trim().is_empty() {
        initial_text
    } else {
        input.trim().to_string()
    };
    Ok(Composition {
        instruction,
        images,
    })
}

struct Composer {
    text: String,
    cursor: usize,
    images: Vec<ImagePayload>,
    notice: String,
    version: String,
    provider: String,
    selected_suggestion: Option<usize>,
}

impl Composer {
    fn new(text: String, images: Vec<ImagePayload>, version: String, provider: String) -> Self {
        let cursor = text.len();
        Self {
            text,
            cursor,
            images,
            notice: String::new(),
            version,
            provider,
            selected_suggestion: None,
        }
    }

    fn run(mut self) -> Result<Composition, Box<dyn std::error::Error + Send + Sync>> {
        let mut _terminal = RawTerminal::enter()?;
        self.redraw()?;

        let mut stdin = io::stdin().lock();
        loop {
            let byte = read_byte(&mut stdin)?;
            match byte {
                b'\r' | b'\n' => {
                    let trimmed = self.text.trim();
                    let mut suggestion_cmd = None;
                    if self.text.starts_with('/') {
                        let matches = get_matching_commands(&self.text);
                        if let Some(idx) = self.selected_suggestion {
                            if idx < matches.len() {
                                suggestion_cmd = Some(matches[idx].0.to_string());
                            }
                        }
                    }

                    let command_str = if let Some(ref cmd) = suggestion_cmd {
                        cmd.as_str()
                    } else {
                        trimmed
                    };

                    if command_str.starts_with('/') {
                        let parts: Vec<&str> = command_str.split_whitespace().collect();
                        let command = parts[0];
                        match command {
                            "/help" => {
                                self.text.clear();
                                self.cursor = 0;
                                self.selected_suggestion = None;
                                self.notice = "Commands: /help (show help), /provider (change provider), /change-key (update key)".to_string();
                                self.redraw()?;
                                continue;
                            }
                            "/provider" | "/change-provider" => {
                                drop(stdin);
                                drop(_terminal);
                                print!("\r\x1b[J");
                                io::stdout().flush()?;

                                if let Ok(credentials) = crate::infrastructure::CredentialStore::new() {
                                    if let Ok(new_prov) = credentials.prompt_and_save_provider() {
                                        self.provider = new_prov.display_name().to_string();
                                        self.notice = format!("AI provider changed to {}.", self.provider);

                                        if let Ok(false) = credentials.has_key(new_prov) {
                                            println!("\nNo stored API key found for {}. Setup is required.", new_prov.display_name());
                                            if credentials.prompt_and_save_key(new_prov).is_ok() {
                                                self.notice = format!("AI provider changed to {} and API key configured.", self.provider);
                                            } else {
                                                self.notice = format!("AI provider changed to {}, but API key was not configured.", self.provider);
                                            }
                                        }
                                    } else {
                                        self.notice = "AI provider change cancelled.".to_string();
                                    }
                                } else {
                                    self.notice = "Failed to load credentials store.".to_string();
                                }

                                _terminal = RawTerminal::enter()?;
                                stdin = io::stdin().lock();
                                self.text.clear();
                                self.cursor = 0;
                                self.selected_suggestion = None;
                                self.redraw()?;
                                continue;
                            }
                            "/change-key" => {
                                drop(stdin);
                                drop(_terminal);
                                print!("\r\x1b[J");
                                io::stdout().flush()?;

                                if let Ok(credentials) = crate::infrastructure::CredentialStore::new() {
                                    let current_prov = if let Ok(p) = credentials.select_provider(None, false) {
                                        p
                                    } else {
                                        crate::infrastructure::AiProvider::Gemini
                                    };
                                    if credentials.prompt_and_save_key(current_prov).is_ok() {
                                        self.notice = format!("API key for {} successfully updated.", current_prov.display_name());
                                    } else {
                                        self.notice = "API key update failed.".to_string();
                                    }
                                } else {
                                    self.notice = "Failed to load credentials store.".to_string();
                                }

                                _terminal = RawTerminal::enter()?;
                                stdin = io::stdin().lock();
                                self.text.clear();
                                self.cursor = 0;
                                self.selected_suggestion = None;
                                self.redraw()?;
                                continue;
                            }
                            _ => {
                                self.notice = format!("Unknown command: {}. Type /help for options.", command);
                                self.selected_suggestion = None;
                                self.redraw()?;
                                continue;
                            }
                        }
                    }

                    if self.text.trim().is_empty() && self.images.is_empty() {
                        self.notice = "Enter a prompt before submitting.".to_string();
                        self.redraw()?;
                        continue;
                    }
                    print!("\r\x1b[J");
                    io::stdout().flush()?;
                    if self.text.trim().is_empty() && !self.images.is_empty() {
                        return Ok(Composition {
                            instruction: String::new(),
                            images: self.images,
                        });
                    }
                    return Ok(Composition {
                        instruction: self.text.trim().to_string(),
                        images: self.images,
                    });
                }
                3 => return Err("Prompt entry cancelled".into()),
                1 => {
                    self.cursor = 0;
                    self.redraw()?;
                }
                5 => {
                    self.cursor = self.text.len();
                    self.redraw()?;
                }
                8 | 127 => {
                    if let Some((position, _)) = self.text[..self.cursor].char_indices().last() {
                        self.text.drain(position..self.cursor);
                        self.cursor = position;
                    }
                    self.selected_suggestion = None;
                    self.redraw()?;
                }
                21 => {
                    self.text.clear();
                    self.cursor = 0;
                    self.notice = "Prompt cleared.".to_string();
                    self.selected_suggestion = None;
                    self.redraw()?;
                }
                11 => {
                    self.text.truncate(self.cursor);
                    self.selected_suggestion = None;
                    self.redraw()?;
                }
                23 => {
                    while self.text[..self.cursor]
                        .chars()
                        .last()
                        .is_some_and(|value| value.is_whitespace())
                    {
                        let position = self.text[..self.cursor].char_indices().last().unwrap().0;
                        self.text.drain(position..self.cursor);
                        self.cursor = position;
                    }
                    while self.text[..self.cursor]
                        .chars()
                        .last()
                        .is_some_and(|value| !value.is_whitespace())
                    {
                        let position = self.text[..self.cursor].char_indices().last().unwrap().0;
                        self.text.drain(position..self.cursor);
                        self.cursor = position;
                    }
                    self.selected_suggestion = None;
                    self.redraw()?;
                }
                18 => {
                    self.remove_last_image();
                    self.redraw()?;
                }
                22 => {
                    match image_from_clipboard() {
                        Ok(image) => {
                            self.images.push(image);
                            self.notice =
                                format!("Attached clipboard image as img{}.", self.images.len());
                        }
                        Err(error) => self.notice = error,
                    }
                    self.redraw()?;
                }
                27 => match read_escape_sequence(&mut stdin)?.as_deref() {
                    Some("[200~") => {
                        let pasted = read_bracketed_paste(&mut stdin)?;
                        self.insert_text(&pasted);
                        self.notice = "Pasted text into prompt.".to_string();
                        self.selected_suggestion = None;
                        self.redraw()?;
                    }
                    Some("[A") // Up Arrow
                        if self.text.starts_with('/') => {
                            let matches = get_matching_commands(&self.text);
                            if !matches.is_empty() {
                                let len = matches.len();
                                self.selected_suggestion = match self.selected_suggestion {
                                    None => Some(len - 1),
                                    Some(idx) => Some((idx + len - 1) % len),
                                };
                                self.redraw()?;
                                continue;
                            }
                        }
                    Some("[B") // Down Arrow
                        if self.text.starts_with('/') => {
                            let matches = get_matching_commands(&self.text);
                            if !matches.is_empty() {
                                let len = matches.len();
                                self.selected_suggestion = match self.selected_suggestion {
                                    None => Some(0),
                                    Some(idx) => Some((idx + 1) % len),
                                };
                                self.redraw()?;
                                continue;
                            }
                        }
                    Some("[D") => {
                        if let Some((position, _)) = self.text[..self.cursor].char_indices().last()
                        {
                            self.cursor = position;
                        }
                        self.redraw()?;
                    }
                    Some("[C") => {
                        if let Some(character) = self.text[self.cursor..].chars().next() {
                            self.cursor += character.len_utf8();
                        }
                        self.redraw()?;
                    }
                    Some("[H") | Some("[1~") => {
                        self.cursor = 0;
                        self.redraw()?;
                    }
                    Some("[F") | Some("[4~") => {
                        self.cursor = self.text.len();
                        self.redraw()?;
                    }
                    Some("[3~") => {
                        if let Some(character) = self.text[self.cursor..].chars().next() {
                            self.text
                                .drain(self.cursor..self.cursor + character.len_utf8());
                        }
                        self.redraw()?;
                    }
                    _ => {}
                },
                value if value >= 32 => {
                    let character = read_utf8_character(value, &mut stdin)?;
                    self.insert_text(&character.to_string());
                    self.selected_suggestion = None;
                    self.redraw()?;
                }
                _ => {}
            }
        }
    }

    fn insert_text(&mut self, text: &str) {
        self.text.insert_str(self.cursor, text);
        self.cursor += text.len();
    }

    fn remove_last_image(&mut self) {
        if self.images.is_empty() {
            self.notice = "No attached images to remove.".to_string();
            return;
        }

        self.images.pop();
        if self.images.is_empty() {
            self.notice = "Removed all attached images.".to_string();
        } else {
            self.notice = format!("Removed last image. {} remaining.", self.images.len());
        }
    }

    fn redraw(&self) -> io::Result<()> {
        let box_width = get_terminal_width();
        let content_width = box_width - 4;

        let mut lines = Vec::new();
        lines.push(String::new());
        lines.push("  \x1b[1;38;2;120;138;62m ___   ___ _____ ___ _  _  ___ ___ _  _ ___ \x1b[0m".to_string());
        lines.push("  \x1b[1;38;2;120;138;62m|   \\ / _ \\_   _| __| \\| |/ __|_ _| \\| | __|\x1b[0m".to_string());
        lines.push("  \x1b[1;38;2;120;138;62m| |) | (_) || | | _|| .` | (_ || || .` | _| \x1b[0m".to_string());
        lines.push("  \x1b[1;38;2;120;138;62m|___/ \\___/ |_| |___|_|\\_|\\___|___|_|\\_|___|\x1b[0m".to_string());
        lines.push(String::new());
        lines.push(format!(
            "  \x1b[38;2;85;102;40mVersion  :: v{}\x1b[0m",
            self.version
        ));
        lines.push(format!(
            "  \x1b[38;2;85;102;40mProvider :: {}\x1b[0m",
            self.provider
        ));
        lines.push(String::new());
        lines.push("  \x1b[1;38;2;120;138;62mDescribe your design\x1b[0m".to_string());
        lines.push(heading(&make_top_border(box_width)));

        let plain_text = if self.text.is_empty() {
            if self.images.is_empty() {
                "> _ Ask Dotengine to style your desktop...".to_string()
            } else {
                "> _ Paste a screenshot or describe the setup...".to_string()
            }
        } else {
            let mut display = self.text.clone();
            display.insert(self.cursor, '_');
            format!("> {}", display)
        };

        for line in wrap_text(&plain_text, content_width) {
            lines.push(input_row_styled(
                &line,
                box_width,
                true,
                self.text.is_empty(),
            ));
        }

        if !self.images.is_empty() {
            lines.push(input_row_styled("", box_width, false, false));
            lines.push(input_row_styled(
                &format!(
                    "  {}",
                    (1..=self.images.len())
                        .map(|index| format!("[ img{} ]", index))
                        .collect::<Vec<_>>()
                        .join(" ")
                ),
                box_width,
                false,
                false,
            ));
        }

        if self.text.starts_with('/') {
            let matches = get_matching_commands(&self.text);
            if !matches.is_empty() {
                lines.push(heading(&make_divider(box_width)));
                lines.push(input_row_styled("  Suggestions:", box_width, false, false));
                for (i, (cmd, desc)) in matches.iter().enumerate() {
                    let is_selected = self.selected_suggestion == Some(i);
                    lines.push(suggestion_row(cmd, desc, box_width, is_selected));
                }
            }
        }

        lines.push(heading(&make_bottom_border(box_width)));
        lines.push(shortcut_footer());
        if !self.notice.is_empty() {
            lines.push(format!("  {} {}", accent("Note"), self.notice));
        }

        print!("\x1b[H\x1b[2J{}", lines.join("\r\n"));
        io::stdout().flush()
    }
}

struct RawTerminal {
    saved_state: String,
}

impl RawTerminal {
    fn enter() -> Result<Self, io::Error> {
        let state = Command::new("stty")
            .arg("-g")
            .stdin(Stdio::inherit())
            .output()?;
        if !state.status.success() {
            return Err(io::Error::other("Unable to read terminal settings"));
        }
        let saved_state = String::from_utf8_lossy(&state.stdout).trim().to_string();
        let status = Command::new("stty").args(["raw", "-echo"]).status()?;
        if !status.success() {
            return Err(io::Error::other("Unable to enable interactive input"));
        }
        print!("\x1b[?1049h\x1b[?2004h");
        io::stdout().flush()?;
        Ok(Self { saved_state })
    }
}

impl Drop for RawTerminal {
    fn drop(&mut self) {
        print!("\x1b[?2004l\x1b[?1049l");
        let _ = io::stdout().flush();
        let _ = Command::new("stty").arg(&self.saved_state).status();
    }
}

fn image_from_clipboard() -> Result<ImagePayload, String> {
    for media_type in ["image/png", "image/jpeg"] {
        if let Ok(output) = Command::new("wl-paste")
            .args(["--no-newline", "--type", media_type])
            .stderr(Stdio::null())
            .output()
        {
            if output.status.success() && !output.stdout.is_empty() {
                return ImagePayload::from_reference_image_bytes(&output.stdout);
            }
        }
    }
    Err("Clipboard does not contain a PNG or JPEG image (requires wl-paste).".to_string())
}

fn read_byte(reader: &mut impl Read) -> io::Result<u8> {
    let mut byte = [0_u8; 1];
    reader.read_exact(&mut byte)?;
    Ok(byte[0])
}

fn read_escape_sequence(reader: &mut impl Read) -> io::Result<Option<String>> {
    let first = read_byte(reader)?;
    if first != b'[' {
        return Ok(None);
    }
    let mut sequence = String::from("[");
    loop {
        let value = read_byte(reader)? as char;
        sequence.push(value);
        if value.is_ascii_alphabetic() || value == '~' {
            break;
        }
    }
    Ok(Some(sequence))
}

fn read_bracketed_paste(reader: &mut impl Read) -> io::Result<String> {
    const END: &[u8] = b"\x1b[201~";
    let mut pasted = Vec::new();
    loop {
        pasted.push(read_byte(reader)?);
        if pasted.ends_with(END) {
            pasted.truncate(pasted.len() - END.len());
            return Ok(String::from_utf8_lossy(&pasted).replace('\r', ""));
        }
    }
}

fn read_utf8_character(first: u8, reader: &mut impl Read) -> io::Result<char> {
    let width = if first & 0b1000_0000 == 0 {
        1
    } else if first & 0b1110_0000 == 0b1100_0000 {
        2
    } else if first & 0b1111_0000 == 0b1110_0000 {
        3
    } else {
        4
    };
    let mut bytes = vec![first];
    for _ in 1..width {
        bytes.push(read_byte(reader)?);
    }
    Ok(String::from_utf8_lossy(&bytes)
        .chars()
        .next()
        .unwrap_or('?'))
}

fn wrap_text(text: &str, width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    for source_line in text.lines() {
        let mut line = String::new();
        for character in source_line.chars() {
            if line.chars().count() == width {
                lines.push(line);
                line = String::new();
            }
            line.push(character);
        }
        lines.push(line);
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}

fn get_terminal_width() -> usize {
    if let Ok(output) = Command::new("stty").arg("size").output() {
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let mut parts = stdout.split_whitespace();
            if let (Some(_rows), Some(cols)) = (parts.next(), parts.next()) {
                if let Ok(width) = cols.parse::<usize>() {
                    return width.max(84);
                }
            }
        }
    }
    84
}

fn shortcut_footer() -> String {
    format!(
        "  {} send    {} attach image    {} remove last image    {} clear    {} commands",
        accent("Enter"),
        accent("Ctrl+V"),
        accent("Ctrl+R"),
        accent("Ctrl+U"),
        accent("/help")
    )
}

fn make_top_border(width: usize) -> String {
    let dashes = "─".repeat(width - 2);
    format!("┌{}┐", dashes)
}

fn make_bottom_border(width: usize) -> String {
    let dashes = "─".repeat(width - 2);
    format!("└{}┘", dashes)
}

fn make_divider(width: usize) -> String {
    let dashes = "─".repeat(width - 2);
    format!("├{}┤", dashes)
}

fn get_matching_commands(input: &str) -> Vec<(&'static str, &'static str)> {
    let commands = vec![
        ("/help", "Show composer help and shortcuts"),
        ("/provider", "Change preferred default AI provider"),
        ("/change-key", "Update API key for active provider"),
    ];
    if input == "/" {
        commands
    } else {
        commands
            .into_iter()
            .filter(|(cmd, _)| cmd.starts_with(input))
            .collect()
    }
}

fn suggestion_row(cmd: &str, desc: &str, box_width: usize, is_selected: bool) -> String {
    let content_width = box_width - 4;

    let prefix = if is_selected { "  ▶ " } else { "  • " };
    let middle = " :: ";
    let cmd_padded = format!("{:<12}", cmd);
    let visual_text = format!("{}{}{}{}", prefix, cmd_padded, middle, desc);

    let visible_chars: Vec<char> = visual_text.chars().take(content_width).collect();
    let visible_len = visible_chars.len();
    let padding_needed = content_width.saturating_sub(visible_len);

    let final_padded = if visible_len >= 16 {
        let cmd_padded_colored = if is_selected {
            format!("\x1b[1;38;2;166;227;161m{:<12}\x1b[0m", cmd)
        } else {
            format!("\x1b[1;38;2;120;138;62m{:<12}\x1b[0m", cmd)
        };
        let desc_part: String = visible_chars[16..].iter().collect();
        let styled_desc = if is_selected {
            format!("\x1b[1;37m{}\x1b[0m", desc_part)
        } else {
            desc_part
        };
        let styled_prefix = if is_selected {
            "\x1b[1;38;2;166;227;161m  ▶ \x1b[0m"
        } else {
            "  • "
        };
        format!("{}{}{}{}", styled_prefix, cmd_padded_colored, styled_desc, " ".repeat(padding_needed))
    } else {
        let visible_str: String = visible_chars.iter().collect();
        format!("{}{}", visible_str, " ".repeat(padding_needed))
    };

    let border = "\x1b[38;2;120;138;62m│\x1b[0m";
    format!("{} {} {}", border, final_padded, border)
}

#[allow(dead_code)]
fn input_row(content: &str, box_width: usize) -> String {
    let content_width = box_width - 4;
    let visible_content: String = content.chars().take(content_width).collect();
    format!("│ {:<width$} │", visible_content, width = content_width)
}

fn input_row_styled(
    content: &str,
    box_width: usize,
    is_styled_prompt: bool,
    is_empty: bool,
) -> String {
    let content_width = box_width - 4;
    let visible_content: String = content.chars().take(content_width).collect();
    let mut padded_content = format!("{:<width$}", visible_content, width = content_width);

    if is_styled_prompt {
        if padded_content.starts_with("> ") {
            if is_empty {
                padded_content = format!(
                    "\x1b[38;2;120;138;62m>\x1b[0m \x1b[38;2;85;102;40m{}\x1b[0m",
                    &padded_content[2..]
                );
            } else {
                padded_content = format!("\x1b[38;2;120;138;62m>\x1b[0m {}", &padded_content[2..]);
            }
        } else if is_empty {
            padded_content = format!("\x1b[38;2;85;102;40m{}\x1b[0m", padded_content);
        }
    }

    let border = "\x1b[38;2;120;138;62m│\x1b[0m";
    format!("{} {} {}", border, padded_content, border)
}

#[cfg(test)]
mod tests {
    use super::{
        input_row, make_bottom_border, make_top_border, shortcut_footer, wrap_text, Composer,
    };
    use crate::domain::ImagePayload;

    #[test]
    fn wraps_composer_input_to_box_width() {
        assert_eq!(wrap_text("abcdef", 3), vec!["abc", "def"]);
        assert_eq!(wrap_text("a\nb", 3), vec!["a", "b"]);
    }

    #[test]
    fn input_box_rows_have_consistent_width() {
        let box_width = 72;
        let top = make_top_border(box_width);
        let bottom = make_bottom_border(box_width);
        assert_eq!(top.chars().count(), box_width);
        assert_eq!(bottom.chars().count(), box_width);
        assert_eq!(
            input_row("[ img1 ] [ img2 ]", box_width).chars().count(),
            box_width
        );
        assert_eq!(
            input_row("Ask Dotengine to style your desktop...", box_width)
                .chars()
                .count(),
            box_width
        );
    }

    #[test]
    fn shortcut_footer_highlights_keys_and_includes_image_removal() {
        let footer = shortcut_footer();

        assert!(footer.contains("\x1b["));
        assert!(footer.contains("Enter"));
        assert!(footer.contains("Ctrl+V"));
        assert!(footer.contains("Ctrl+R"));
        assert!(footer.contains("remove last image"));
        assert!(footer.contains("Ctrl+U"));
    }

    #[test]
    fn ctrl_r_action_removes_last_attached_image() {
        let png = ImagePayload::from_bytes(b"\x89PNG\r\n\x1a\nfirst").unwrap();
        let jpeg = ImagePayload::from_bytes(&[0xff, 0xd8, 0xff, 0xe0]).unwrap();
        let mut composer = Composer::new(
            String::new(),
            vec![png, jpeg],
            "test".to_string(),
            "Gemini".to_string(),
        );

        composer.remove_last_image();

        assert_eq!(composer.images.len(), 1);
        assert_eq!(composer.images[0].media_type, "image/png");
        assert_eq!(composer.notice, "Removed last image. 1 remaining.");
    }

    #[test]
    fn ctrl_r_action_reports_when_no_images_are_attached() {
        let mut composer = Composer::new(
            String::new(),
            Vec::new(),
            "test".to_string(),
            "Gemini".to_string(),
        );

        composer.remove_last_image();

        assert!(composer.images.is_empty());
        assert_eq!(composer.notice, "No attached images to remove.");
    }

    #[test]
    fn test_get_matching_commands() {
        use super::get_matching_commands;
        let all = get_matching_commands("/");
        assert_eq!(all.len(), 3);
        assert_eq!(all[0].0, "/help");
        assert_eq!(all[1].0, "/provider");
        assert_eq!(all[2].0, "/change-key");

        let h = get_matching_commands("/h");
        assert_eq!(h.len(), 1);
        assert_eq!(h[0].0, "/help");

        let c = get_matching_commands("/c");
        assert_eq!(c.len(), 1);
        assert_eq!(c[0].0, "/change-key");

        let none = get_matching_commands("/invalid");
        assert!(none.is_empty());
    }

    #[test]
    fn test_suggestion_row_formatting() {
        use super::suggestion_row;
        let box_width = 80;
        
        // Test non-selected suggestion row formatting
        let row = suggestion_row("/help", "Test description", box_width, false);
        let mut plain = String::new();
        let mut in_escape = false;
        for c in row.chars() {
            if c == '\x1b' {
                in_escape = true;
            } else if in_escape {
                if c == 'm' {
                    in_escape = false;
                }
            } else {
                plain.push(c);
            }
        }
        assert_eq!(plain.chars().count(), box_width);
        assert!(row.contains("│"));
        assert!(row.contains("/help"));
        assert!(row.contains("Test description"));
        assert!(row.contains("•"));

        // Test selected suggestion row formatting
        let row_sel = suggestion_row("/help", "Test description", box_width, true);
        let mut plain_sel = String::new();
        let mut in_escape_sel = false;
        for c in row_sel.chars() {
            if c == '\x1b' {
                in_escape_sel = true;
            } else if in_escape_sel {
                if c == 'm' {
                    in_escape_sel = false;
                }
            } else {
                plain_sel.push(c);
            }
        }
        assert_eq!(plain_sel.chars().count(), box_width);
        assert!(row_sel.contains("│"));
        assert!(row_sel.contains("/help"));
        assert!(row_sel.contains("Test description"));
        assert!(row_sel.contains("▶"));
    }
}
