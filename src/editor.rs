//! Small built-in multi-line editor shared by the CLI and TUI.
//! Press Escape, then C to save; Escape, then Q to cancel.
use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Terminal,
};
use std::io;

const ACCENT: Color = Color::Rgb(92, 190, 180);
const MUTED: Color = Color::Rgb(125, 139, 151);

/// Open the editor. `return_to_tui` restores the caller's alternate screen/raw mode.
pub fn edit_text(initial: &str, title: &str, return_to_tui: bool) -> Result<Option<String>> {
    if return_to_tui {
        disable_raw_mode()?;
        execute!(io::stdout(), LeaveAlternateScreen)?;
    }
    enable_raw_mode()?;
    execute!(io::stdout(), EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;
    let result = editor_loop(&mut terminal, initial, title);
    let restore_result = (|| -> Result<()> {
        disable_raw_mode()?;
        execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
        terminal.show_cursor()?;
        if return_to_tui {
            enable_raw_mode()?;
            execute!(io::stdout(), EnterAlternateScreen)?;
        }
        Ok(())
    })();
    restore_result?;
    result
}

fn editor_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    initial: &str,
    title: &str,
) -> Result<Option<String>> {
    let mut lines: Vec<String> = initial.split('\n').map(str::to_owned).collect();
    if lines.is_empty() { lines.push(String::new()); }
    let mut row = lines.len().saturating_sub(1);
    let mut col = lines[row].chars().count();
    let mut escape_pending = false;
    let mut message = String::from("Type your note.  Esc, then C: save   ·   Esc, then Q: cancel");

    loop {
        terminal.draw(|frame| {
            let area = frame.area();
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(3), Constraint::Min(4), Constraint::Length(2)])
                .split(area);
            let heading = Paragraph::new(Line::from(vec![
                Span::styled(" CYBERNOTES ", Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)),
                Span::styled(format!("/ {title}"), Style::default().fg(MUTED)),
            ]))
            .block(Block::default().borders(Borders::BOTTOM));
            frame.render_widget(heading, chunks[0]);

            let content = lines.join("\n");
            let editor = Paragraph::new(content)
                .block(Block::default().title(" NOTE EDITOR ").borders(Borders::ALL).border_style(Style::default().fg(ACCENT)))
                .wrap(Wrap { trim: false });
            let inner_height = chunks[1].height.saturating_sub(2) as usize;
            let scroll = row.saturating_sub(inner_height.saturating_sub(1));
            frame.render_widget(editor.scroll((scroll as u16, 0)), chunks[1]);

            let footer = Paragraph::new(vec![
                Line::from(Span::styled(if escape_pending { "Esc received — press C to save or Q to cancel" } else { message.as_str() }, Style::default().fg(MUTED))),
                Line::from(vec![
                    Span::styled(" Esc + C ", Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)),
                    Span::raw("save    "),
                    Span::styled("Esc + Q", Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)),
                    Span::raw(" cancel    ·    Enter for a new line"),
                ]),
            ]);
            frame.render_widget(footer, chunks[2]);

            let x = chunks[1].x.saturating_add(1).saturating_add(col as u16);
            let y = chunks[1].y.saturating_add(1).saturating_add(row.saturating_sub(scroll) as u16);
            let max_x = chunks[1].x.saturating_add(chunks[1].width.saturating_sub(2));
            let max_y = chunks[1].y.saturating_add(chunks[1].height.saturating_sub(2));
            frame.set_cursor_position((x.min(max_x), y.min(max_y)));
        })?;

        let Event::Key(key) = event::read()? else { continue; };
        if key.kind != KeyEventKind::Press { continue; }

        if escape_pending {
            escape_pending = false;
            match key.code {
                KeyCode::Char('c') | KeyCode::Char('C') => return Ok(Some(lines.join("\n"))),
                KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => return Ok(None),
                _ => message = String::from("Save with Esc, then C; cancel with Esc, then Q"),
            }
            continue;
        }

        match key.code {
            KeyCode::Esc => escape_pending = true,
            KeyCode::Char(ch) if !key.modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) => {
                let byte = char_byte_index(&lines[row], col);
                lines[row].insert(byte, ch);
                col += 1;
            }
            KeyCode::Enter => {
                let byte = char_byte_index(&lines[row], col);
                let tail = lines[row].split_off(byte);
                row += 1;
                col = 0;
                lines.insert(row, tail);
            }
            KeyCode::Backspace => {
                if col > 0 {
                    let start = char_byte_index(&lines[row], col - 1);
                    let end = char_byte_index(&lines[row], col);
                    lines[row].replace_range(start..end, "");
                    col -= 1;
                } else if row > 0 {
                    let current = lines.remove(row);
                    row -= 1;
                    col = lines[row].chars().count();
                    lines[row].push_str(&current);
                }
            }
            KeyCode::Delete => {
                if col < lines[row].chars().count() {
                    let start = char_byte_index(&lines[row], col);
                    let end = char_byte_index(&lines[row], col + 1);
                    lines[row].replace_range(start..end, "");
                } else if row + 1 < lines.len() {
                    let next = lines.remove(row + 1);
                    lines[row].push_str(&next);
                }
            }
            KeyCode::Left => {
                if col > 0 { col -= 1; }
                else if row > 0 { row -= 1; col = lines[row].chars().count(); }
            }
            KeyCode::Right => {
                if col < lines[row].chars().count() { col += 1; }
                else if row + 1 < lines.len() { row += 1; col = 0; }
            }
            KeyCode::Up => {
                row = row.saturating_sub(1);
                col = col.min(lines[row].chars().count());
            }
            KeyCode::Down => {
                if row + 1 < lines.len() { row += 1; }
                col = col.min(lines[row].chars().count());
            }
            KeyCode::Home => col = 0,
            KeyCode::End => col = lines[row].chars().count(),
            _ => {}
        }
    }
}

fn char_byte_index(line: &str, char_index: usize) -> usize {
    line.char_indices().nth(char_index).map(|(i, _)| i).unwrap_or(line.len())
}

#[cfg(test)]
mod tests {
    use super::char_byte_index;

    #[test]
    fn character_index_handles_multibyte_text() {
        let value = "aé日z";
        assert_eq!(&value[..char_byte_index(value, 0)], "");
        assert_eq!(&value[..char_byte_index(value, 1)], "a");
        assert_eq!(&value[..char_byte_index(value, 2)], "aé");
        assert_eq!(&value[..char_byte_index(value, 3)], "aé日");
        assert_eq!(char_byte_index(value, 99), value.len());
    }
}
