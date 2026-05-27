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
) -> Result<Composition, Box<dyn std::error::Error + Send + Sync>> {
    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        return compose_line_mode(initial_text, initial_images);
    }

    Composer::new(initial_text, initial_images).run()
}

fn compose_line_mode(
    initial_text: String,
    images: Vec<ImagePayload>,
) -> Result<Composition, Box<dyn std::error::Error + Send + Sync>> {
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
}

impl Composer {
    fn new(text: String, images: Vec<ImagePayload>) -> Self {
        let cursor = text.len();
        Self {
            text,
            cursor,
            images,
            notice: String::new(),
        }
    }

    fn run(mut self) -> Result<Composition, Box<dyn std::error::Error + Send + Sync>> {
        let _terminal = RawTerminal::enter()?;
        self.redraw()?;

        let mut stdin = io::stdin().lock();
        loop {
            let byte = read_byte(&mut stdin)?;
            match byte {
                b'\r' | b'\n' => {
                    if self.text.trim().is_empty() {
                        self.notice = "Enter a prompt before submitting.".to_string();
                        self.redraw()?;
                        continue;
                    }
                    print!("\r\x1b[J");
                    io::stdout().flush()?;
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
                    self.redraw()?;
                }
                21 => {
                    self.text.clear();
                    self.cursor = 0;
                    self.notice = "Prompt cleared.".to_string();
                    self.redraw()?;
                }
                11 => {
                    self.text.truncate(self.cursor);
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
                    self.redraw()?;
                }
                18 => {
                    if self.images.is_empty() {
                        self.notice = "No attached images to remove.".to_string();
                    } else {
                        self.images.pop();
                        if self.images.is_empty() {
                            self.notice = "Removed all attached images.".to_string();
                        } else {
                            self.notice = format!(
                                "Removed last image. {} remaining.",
                                self.images.len()
                            );
                        }
                    }
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
                        self.redraw()?;
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

    fn redraw(&self) -> io::Result<()> {
        let box_width = get_terminal_width();
        let content_width = box_width - 4;
        let mut lines = vec![heading(&make_top_border(box_width))];
        let display_text = if self.text.is_empty() {
            "_ Ask Dotengine to style your desktop...".to_string()
        } else {
            let mut display = self.text.clone();
            display.insert(self.cursor, '_');
            display
        };
        for line in wrap_text(&display_text, content_width) {
            lines.push(input_row(&line, box_width));
        }
        if !self.images.is_empty() {
            lines.push(input_row("", box_width));
            lines.push(input_row(&format!(
                "  {}",
                (1..=self.images.len())
                    .map(|index| format!("[ img{} ]", index))
                    .collect::<Vec<_>>()
                    .join(" ")
            ), box_width));
        }
        lines.push(heading(&make_bottom_border(box_width)));
        lines.push(format!(
            "  {} send    {} attach image    {} remove image    {} clear prompt",
            accent("Enter"),
            accent("Ctrl+V"),
            accent("Ctrl+R"),
            accent("Ctrl+U")
        ));
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
                return ImagePayload::from_bytes(&output.stdout);
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
                    return width.max(40);
                }
            }
        }
    }
    72
}

fn make_top_border(width: usize) -> String {
    let dashes = "─".repeat(width - 2);
    format!("╭{}╮", dashes)
}

fn make_bottom_border(width: usize) -> String {
    let dashes = "─".repeat(width - 2);
    format!("╰{}╯", dashes)
}

fn input_row(content: &str, box_width: usize) -> String {
    let content_width = box_width - 4;
    let visible_content: String = content.chars().take(content_width).collect();
    format!("│ {:<width$} │", visible_content, width = content_width)
}

#[cfg(test)]
mod tests {
    use super::{input_row, wrap_text, make_bottom_border, make_top_border};

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
        assert_eq!(input_row("[ img1 ] [ img2 ]", box_width).chars().count(), box_width);
        assert_eq!(
            input_row("Ask Dotengine to style your desktop...", box_width)
                .chars()
                .count(),
            box_width
        );
    }
}
