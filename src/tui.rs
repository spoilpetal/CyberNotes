use anyhow::Result;
use crossterm::{event::{self, Event, KeyCode, KeyEventKind}, execute, terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen}};
use ratatui::{backend::CrosstermBackend, layout::{Constraint, Direction, Layout}, style::{Color, Modifier, Style}, text::{Line, Span}, widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap}, Terminal};
use std::io::{self, Write};
use crate::{clipboard, editor, model::{Note, VaultData}, vault};

const ACCENT: Color = Color::Rgb(92, 190, 180);
const MUTED: Color = Color::Rgb(125, 139, 151);
const PANEL: Color = Color::Rgb(34, 42, 52);

fn prompt(terminal:&mut Terminal<CrosstermBackend<io::Stdout>>,label:&str,default:&str)->Result<String>{
    disable_raw_mode()?;
    execute!(io::stdout(),LeaveAlternateScreen)?;
    let read_result=(||->Result<String>{
        print!("{label}");io::stdout().flush()?;
        let mut value=String::new();io::stdin().read_line(&mut value)?;
        Ok(value)
    })();
    enable_raw_mode()?;
    execute!(io::stdout(),EnterAlternateScreen)?;
    terminal.clear()?;
    let value=read_result?.trim_end_matches(['\r','\n']).to_string();
    Ok(if value.is_empty(){default.to_string()}else{value})
}
fn persist(pass:Option<&str>,data:&VaultData)->Result<()>{vault::save(pass.map(str::as_bytes),data)}
pub fn run(pass:Option<&str>,data:&mut VaultData)->Result<()>{
    enable_raw_mode()?;execute!(io::stdout(),EnterAlternateScreen)?;
    let backend=CrosstermBackend::new(io::stdout());let mut terminal=Terminal::new(backend)?;
    let result=app(&mut terminal,pass,data);
    disable_raw_mode()?;execute!(terminal.backend_mut(),LeaveAlternateScreen)?;terminal.show_cursor()?;result
}
fn app(terminal:&mut Terminal<CrosstermBackend<io::Stdout>>,pass:Option<&str>,data:&mut VaultData)->Result<()>{
    let mut selected=0usize;let mut query=String::new();let mut status=String::from("Ready — your notes stay on this device");
    loop{
        let mut notes:Vec<usize>=data.notes.iter().enumerate().filter(|(_,n)|query.is_empty()||n.title.to_lowercase().contains(&query.to_lowercase())||n.body.to_lowercase().contains(&query.to_lowercase())).map(|(i,_)|i).collect();
        notes.sort_by_key(|&i|(!data.notes[i].pinned,std::cmp::Reverse(data.notes[i].updated_at.timestamp_micros())));
        if selected>=notes.len(){selected=notes.len().saturating_sub(1);}
        terminal.draw(|f|{
            let area=f.area();
            let page=Layout::default().direction(Direction::Vertical).constraints([Constraint::Length(4),Constraint::Min(7),Constraint::Length(3)]).split(area);
            let header=Paragraph::new(vec![
                Line::from(vec![Span::styled(" CYBERNOTES ",Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)),Span::styled("/ PRIVATE NOTE SPACE",Style::default().fg(MUTED))]),
                Line::from(vec![Span::styled(format!(" {} notes  ·  {} pinned",data.notes.len(),data.notes.iter().filter(|n|n.pinned).count()),Style::default().fg(Color::White)),Span::raw("   "),Span::styled(if query.is_empty(){"Browse".to_string()}else{format!("Filter: {query}")},Style::default().fg(ACCENT))])
            ]).block(Block::default().borders(Borders::BOTTOM).border_style(Style::default().fg(PANEL)));
            f.render_widget(header,page[0]);
            let panels=Layout::default().direction(Direction::Horizontal).constraints([Constraint::Percentage(40),Constraint::Percentage(60)]).split(page[1]);
            let items:Vec<ListItem>=notes.iter().map(|&i|{let n=&data.notes[i];let first=n.body.lines().next().unwrap_or("No preview");let preview:String=first.chars().take(30).collect();ListItem::new(vec![Line::from(vec![Span::styled(if n.pinned{"◆ "}else{"  "},Style::default().fg(ACCENT)),Span::styled(n.title.as_str(),Style::default().fg(Color::White).add_modifier(Modifier::BOLD))]),Line::from(Span::styled(format!("  {}  ·  {}",n.id,preview),Style::default().fg(MUTED)))])}).collect();
            let mut state=ListState::default();if !items.is_empty(){state.select(Some(selected));}
            let list=List::new(items).block(Block::default().title(" YOUR NOTES ").borders(Borders::ALL).border_style(Style::default().fg(PANEL))).highlight_style(Style::default().bg(PANEL)).highlight_symbol("▌ ");f.render_stateful_widget(list,panels[0],&mut state);
            let preview=notes.get(selected).map(|&i|{let n=&data.notes[i];format!("{}\n{}  ·  Updated {}\n\n{}",n.title,if n.pinned{"PINNED"}else{"NOTE"},n.updated_at.format("%d %b %Y, %H:%M"),n.body)}).unwrap_or_else(||"Your note space is empty.\n\nPress a to create your first note, or v to capture clipboard text.".into());
            let preview_block=Paragraph::new(preview).block(Block::default().title(" NOTE DETAIL ").borders(Borders::ALL).border_style(Style::default().fg(ACCENT))).wrap(Wrap{trim:false});f.render_widget(preview_block,panels[1]);
            let footer=Paragraph::new(vec![Line::from(vec![Span::styled(" a ",Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)),Span::raw("add  "),Span::styled("e ",Style::default().fg(ACCENT)),Span::raw("edit  "),Span::styled("d ",Style::default().fg(ACCENT)),Span::raw("delete  "),Span::styled("p ",Style::default().fg(ACCENT)),Span::raw("pin  "),Span::styled("c ",Style::default().fg(ACCENT)),Span::raw("copy  "),Span::styled("v ",Style::default().fg(ACCENT)),Span::raw("clipboard  "),Span::styled("/ ",Style::default().fg(ACCENT)),Span::raw("search  "),Span::styled("q ",Style::default().fg(ACCENT)),Span::raw("quit")]),Line::from(Span::styled(status.as_str(),Style::default().fg(MUTED)))]).block(Block::default().borders(Borders::TOP).border_style(Style::default().fg(PANEL)));f.render_widget(footer,page[2]);
        })?;
        if let Event::Key(key)=event::read()?{if key.kind!=KeyEventKind::Press{continue;}match key.code{
            KeyCode::Char('q')=>break,
            KeyCode::Down|KeyCode::Char('j')=>if !notes.is_empty(){selected=(selected+1).min(notes.len()-1);},
            KeyCode::Up|KeyCode::Char('k')=>selected=selected.saturating_sub(1),
            KeyCode::Esc=>{query.clear();selected=0;status="Filter cleared".into();},
            KeyCode::Char('/')=>{query=prompt(terminal,"Search notes: ",&query)?;selected=0;status=format!("Showing matches for '{query}'");},
            KeyCode::Char('a')=>{
                let title=prompt(terminal,"Title: ","")?;
                if !title.trim().is_empty(){
                    let add_text=prompt(terminal,"Add note text now? [y/N] ","n")?;
                    let body=if add_text.eq_ignore_ascii_case("y") || add_text.eq_ignore_ascii_case("yes") {
                        let edited=editor::edit_text("", "NEW NOTE", true)?;
                        terminal.clear()?;
                        match edited {
                            Some(body) => body,
                            None => { status="Note creation cancelled".into(); continue; }
                        }
                    } else { String::new() };
                    data.notes.push(Note::new(title,body));persist(pass,data)?;status="Note created and saved".into();
                }
            },
            KeyCode::Char('v')=>match clipboard::get_text(){Ok(body)=>{let title=prompt(terminal,"Title for captured note: ","Clipboard note")?;if !title.trim().is_empty(){data.notes.push(Note::new(title,body));persist(pass,data)?;status="Clipboard captured as a note".into();}},Err(e)=>status=format!("Clipboard: {e}")},
            KeyCode::Char('e')=>if let Some(&i)=notes.get(selected){
                let title=prompt(terminal,"Title (blank keeps current): ",&data.notes[i].title)?;
                let original=data.notes[i].body.clone();
                let edited=editor::edit_text(&original,"EDIT NOTE",true)?;
                terminal.clear()?;
                if let Some(body)=edited {
                    if !title.trim().is_empty(){data.notes[i].title=title;}
                    data.notes[i].body=body;data.notes[i].updated_at=chrono::Utc::now();persist(pass,data)?;status="Changes saved".into();
                } else { status="Edit cancelled; note unchanged".into(); }
            },
            KeyCode::Char('d')=>if let Some(&i)=notes.get(selected){let title=data.notes[i].title.clone();let yes=prompt(terminal,&format!("Delete '{title}'? [y/N] "),"n")?;if yes.eq_ignore_ascii_case("y"){data.notes.remove(i);persist(pass,data)?;status="Note deleted".into();}},
            KeyCode::Char('p')=>if let Some(&i)=notes.get(selected){data.notes[i].pinned=!data.notes[i].pinned;data.notes[i].updated_at=chrono::Utc::now();persist(pass,data)?;status="Pin state updated".into();},
            KeyCode::Char('c')=>if let Some(&i)=notes.get(selected){match clipboard::set_text(&data.notes[i].body){Ok(())=>status="Note body copied".into(),Err(e)=>status=format!("Clipboard: {e}")}},
            _=>{}
        }}
    }
    Ok(())
}
