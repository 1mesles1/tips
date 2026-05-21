use std::fs;
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, BorderType, Clear, List, ListItem, Paragraph, Wrap},
};
use crate::engine::{DbNode, NodeType};
use crate::i18n::Translations;
use crate::config::Config;

#[derive(PartialEq, Clone, Copy)]
pub enum InputState {
    Normal,
    TextSearching,
    TagSearching,
    Creating,
    ConfirmDelete,
    WorkspaceSwitching,
    WorkspaceCreating,
    QuickInboxInput,
    SettingsMenu,
    SettingsPathInput,
    HelpMenu,
}

pub fn draw_ui(
    f: &mut Frame, 
    nodes: &[DbNode], 
    selected_index: usize, 
    text: &Translations, 
    buffer: &str,
    state: &InputState,
    is_right_focused: bool,
    scroll_index: u16,
    workspaces: &[String],
    active_workspace_idx: usize,
    cfg: &Config,
    settings_row_idx: usize,
    help_scroll_index: u16,
) {
    let theme_active_border = Color::Blue;
    let theme_inactive_border = Color::DarkGray;
    let theme_text_selected = Color::Yellow;
    let theme_bg_selected = Color::Indexed(236);
    let theme_tag_color = Color::Magenta;

    let left_border_color = if is_right_focused { theme_inactive_border } else { theme_active_border };
    let right_border_color = if is_right_focused { theme_active_border } else { theme_inactive_border };

    // 1. Делим весь экран по вертикали: Основная область и Нижний Статус-бар
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(3), Constraint::Length(1)])
        .split(f.size());
    let main_area = chunks[0];
    let status_area = chunks[1];

    // 2. Делим основную область по горизонтали: Левая панель (30%), Правая панель (70%)
    let panels_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
        .split(main_area);
    let left_pane = panels_chunks[0];
    let right_pane = panels_chunks[1];

    // --- ЛЕВАЯ ПАНЕЛЬ (ДЕРЕВО) ---
    let items: Vec<ListItem> = nodes
        .iter()
        .enumerate()
        .map(|(idx, node)| {
            let prefix = match node.node_type {
                NodeType::Folder => "📁 ".to_string(),
                NodeType::File => "📝 ".to_string(),
                NodeType::MarkdownHeader { level } => {
                    let indent = "  ".repeat(level.saturating_sub(1));
                    format!("{}└─ ", indent)
                }
            };
            
            let spans = vec![
                Span::styled(prefix, Style::default()),
                Span::styled(node.name.clone(), Style::default()),
            ];

            let mut item = ListItem::new(Line::from(spans));
            if idx == selected_index {
                item = item.style(Style::default()
                    .bg(theme_bg_selected)
                    .fg(theme_text_selected)
                    .add_modifier(Modifier::BOLD));
            }
            item
        })
        .collect();

    let left_block = Block::default()
        .title(Span::styled(text.window_title, Style::default().fg(left_border_color).add_modifier(Modifier::BOLD)))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(left_border_color));

    let list = List::new(items)
        .block(left_block)
        .highlight_symbol(if is_right_focused { " " } else { "▎" })
        .highlight_style(Style::default().fg(theme_text_selected));
        
    f.render_widget(list, left_pane);

    // 3. Делим правую панель по вертикали: Окно текста и Окно тегов (высота 3)
    let right_sub_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(3), Constraint::Length(3)])
        .split(right_pane);
    let right_text_pane = right_sub_chunks[0];
    let right_tags_pane = right_sub_chunks[1];

    // --- ПРАВАЯ ПАНЕЛЬ: ПРОСМОТР ТЕКСТА ---
    let preview_text = text.no_preview.to_string();
    let mut preview_lines: Vec<Line> = Vec::new();
    let mut current_tags: Vec<String> = Vec::new();
    
    if !nodes.is_empty() && selected_index < nodes.len() {
        let selected_node = &nodes[selected_index];
        current_tags = selected_node.tags.clone();

        if selected_node.path.is_file() {
            if let Ok(content) = fs::read_to_string(&selected_node.path) {
                let mut raw_text = String::new();
                match selected_node.node_type {
                    NodeType::MarkdownHeader { .. } => {
                        let mut capture = false;
                        let mut section_lines = Vec::new();
                        for line in content.lines() {
                            if line.to_lowercase().starts_with("tags:") {
                                for word in line.split_whitespace().skip(1) {
                                    if word.starts_with('#') {
                                        current_tags.push(word.trim_matches('#').to_string());
                                    }
                                }
                            }

                            if line.starts_with('#') {
                                let clean_name = line.trim_start_matches('#').trim();
                                if clean_name == selected_node.name {
                                    capture = true;
                                    section_lines.push(line.to_string());
                                    continue;
                                } else if capture {
                                    break;
                                }
                            }
                            if capture {
                                section_lines.push(line.to_string());
                            }
                        }
                        if !section_lines.is_empty() {
                            raw_text = section_lines.join("\n");
                        }
                    }
                    _ => {
                        if !content.trim().is_empty() {
                            raw_text = content;
                        }
                    }
                }

                if !raw_text.is_empty() {
                    for line in raw_text.lines() {
                        if line.to_lowercase().starts_with("tags:") {
                            continue;
                        }
                        if line.starts_with('#') {
                            preview_lines.push(Line::from(Span::styled(
                                format!(" {}", line), 
                                Style::default().fg(Color::LightCyan).add_modifier(Modifier::BOLD)
                            )));
                        } else if line.starts_with('`') || line.ends_with('`') {
                            preview_lines.push(Line::from(Span::styled(
                                format!("   {}", line.replace('`', "")), 
                                Style::default().fg(Color::LightGreen)
                            )));
                        } else {
                            preview_lines.push(Line::from(format!(" {}", line)));
                        }
                    }
                }
            }
        }
    }

    if preview_lines.is_empty() {
        preview_lines.push(Line::from(Span::styled(preview_text, Style::default().fg(Color::DarkGray))));
    }

    let right_title = format!("{} (Скролл: {}) ", text.preview_title, scroll_index);
    let text_block = Block::default()
        .title(Span::styled(right_title, Style::default().fg(right_border_color).add_modifier(Modifier::BOLD)))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(right_border_color));

    let preview_paragraph = Paragraph::new(preview_lines)
        .block(text_block)
        .wrap(Wrap { trim: false })
        .scroll((scroll_index, 0));
    f.render_widget(preview_paragraph, right_text_pane);

    // --- ОКНО ТЕГОВ ---
    let tags_title = " 🏷️ Теги темы ";
    let tags_block = Block::default()
        .title(Span::styled(tags_title, Style::default().fg(right_border_color)))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(right_border_color));

    current_tags.dedup();
    let tags_display_text = if current_tags.is_empty() {
        " нет привязанных тегов".to_string()
    } else {
        format!(" #{}", current_tags.join(" #"))
    };

    let tags_paragraph = Paragraph::new(Span::styled(
        tags_display_text, 
        Style::default().fg(theme_tag_color).add_modifier(Modifier::ITALIC)
    )).block(tags_block);
    
    f.render_widget(tags_paragraph, right_tags_pane);

    // --- ВЫДЕЛЕННЫЙ СТАТУС БАР ---
    let bg_color = Color::Indexed(238);
    let key_style = Style::default().bg(Color::Blue).fg(Color::White).add_modifier(Modifier::BOLD);
    let text_style = Style::default().bg(bg_color).fg(Color::Indexed(254));
    let divider_style = Style::default().bg(bg_color).fg(Color::DarkGray);

    let footer_line;

    match state {
        InputState::TextSearching => {
            footer_line = Line::from(vec![
                Span::styled(text.search_prompt, Style::default().bg(Color::LightCyan).fg(Color::Black).add_modifier(Modifier::BOLD)),
                Span::styled(format!(" {} ", buffer), Style::default().bg(Color::Indexed(235)).fg(Color::White)),
            ]);
        }
        InputState::TagSearching => {
            footer_line = Line::from(vec![
                Span::styled(text.tag_search_prompt, Style::default().bg(Color::Magenta).fg(Color::White).add_modifier(Modifier::BOLD)),
                Span::styled(format!(" {} ", buffer), Style::default().bg(Color::Indexed(235)).fg(Color::White)),
            ]);
        }
        InputState::WorkspaceCreating => {
            footer_line = Line::from(vec![
                Span::styled(text.ws_status_prompt, Style::default().bg(Color::Green).fg(Color::Black).add_modifier(Modifier::BOLD)),
                Span::styled(format!(" {} ", buffer), Style::default().bg(Color::Indexed(235)).fg(Color::White)),
            ]);
        }
        InputState::QuickInboxInput => {
            footer_line = Line::from(vec![
                Span::styled(text.inbox_status_prompt, Style::default().bg(Color::Yellow).fg(Color::Black).add_modifier(Modifier::BOLD)),
                Span::styled(format!(" {} ", buffer), Style::default().bg(Color::Indexed(235)).fg(Color::White)),
            ]);
        }
        InputState::SettingsPathInput => {
            footer_line = Line::from(vec![
                Span::styled(text.settings_path_prompt, Style::default().bg(Color::LightCyan).fg(Color::Black).add_modifier(Modifier::BOLD)),
                Span::styled(format!(" {} ", buffer), Style::default().bg(Color::Indexed(235)).fg(Color::White)),
            ]);
        }
        _ => {
            if is_right_focused {
                footer_line = Line::from(vec![
                    Span::styled(" Tab ", key_style), Span::styled(text.status_back, text_style), Span::styled(" │ ", divider_style),
                    Span::styled(" ↑↓ ", key_style), Span::styled(text.status_scroll, text_style), Span::styled(" │ ", divider_style),
                    Span::styled(" F4 ", key_style), Span::styled(text.status_edit_vim, text_style),
                ]);
            } else {
                footer_line = Line::from(vec![
                    Span::styled(" Ctrl+S ", key_style), Span::styled(text.status_settings, text_style), Span::styled(" │ ", divider_style),
                    Span::styled(" ? ", key_style), Span::styled(text.status_help, text_style), Span::styled(" │ ", divider_style),
                    Span::styled(" w ", key_style), Span::styled(text.status_workspaces, text_style), Span::styled(" │ ", divider_style),
                    Span::styled(" i ", key_style), Span::styled(text.status_inbox, text_style), Span::styled(" │ ", divider_style),
                    Span::styled(" t ", key_style), Span::styled(text.status_tags, text_style), Span::styled(" │ ", divider_style),
                    Span::styled(" F4 ", key_style), Span::styled(text.status_edit, text_style), Span::styled(" │ ", divider_style),
                    Span::styled(" q ", Style::default().bg(Color::Red).fg(Color::White).add_modifier(Modifier::BOLD)), Span::styled(text.status_exit, text_style),
                ]);
            }
        }
    }

    let help_paragraph = Paragraph::new(footer_line).style(Style::default().bg(bg_color));
    f.render_widget(help_paragraph, status_area);

    // --- ВСПЛЫВАЮЩЕЕ ОКНО: ВЫБОР БАЗ ДАННЫХ (w) ---
    if *state == InputState::WorkspaceSwitching {
        let ws_block = Block::default()
            .title(Span::styled(text.ws_title, Style::default().fg(Color::LightMagenta).add_modifier(Modifier::BOLD)))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::LightMagenta));

        let ws_items: Vec<ListItem> = workspaces
            .iter()
            .enumerate()
            .map(|(w_idx, w_name)| {
                let line_str = if w_idx == active_workspace_idx {
                    format!(" 🔘 {}{}", w_name, text.ws_active)
                } else {
                    format!(" ⚪ {}", w_name)
                };
                let mut item = ListItem::new(line_str);
                if w_idx == active_workspace_idx {
                    item = item.style(Style::default().bg(Color::Indexed(238)).fg(Color::LightCyan).add_modifier(Modifier::BOLD));
                }
                item
            })
            .collect();

        let ws_list = List::new(ws_items).block(ws_block);
        let area = centered_rect(50, 40, f.size());
        f.render_widget(Clear, area);
        f.render_widget(ws_list, area);
    }

    // --- ВСПЛЫВАЮЩЕЕ ОКНО: НАСТРОЙКИ (Ctrl + S) ---
    if *state == InputState::SettingsMenu || *state == InputState::SettingsPathInput {
        let st_block = Block::default()
            .title(Span::styled(text.settings_title, Style::default().fg(Color::LightYellow).add_modifier(Modifier::BOLD)))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::LightYellow));

        let labels = [
            format!("{}{}", text.settings_lang, cfg.language.to_uppercase()),
            format!(" ⚙️ Текстовый редактор:  [{}]", cfg.editor.to_lowercase()), // Добавлен пункт карусели редакторов
            format!("{}{}", text.settings_path, cfg.db_root_path),
            text.settings_save.to_string(),
        ];

        let st_items: Vec<ListItem> = labels
            .iter()
            .enumerate()
            .map(|(s_idx, s_label)| {
                let mut item = ListItem::new(s_label.clone());
                if s_idx == settings_row_idx {
                    item = item.style(Style::default().bg(Color::Indexed(238)).fg(Color::Yellow).add_modifier(Modifier::BOLD));
                }
                item
            })
            .collect();

        let st_list = List::new(st_items).block(st_block);
        let area = centered_rect(60, 30, f.size());
        f.render_widget(Clear, area);
        f.render_widget(st_list, area);
    }

    // --- ВСПЛЫВАЮЩЕЕ ОКНО: СПРАВКА ПО КЛАВИШАМ (?) ---
    if *state == InputState::HelpMenu {
        let help_block = Block::default()
            .title(Span::styled(text.help_title, Style::default().fg(Color::LightGreen).add_modifier(Modifier::BOLD)))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::LightGreen));

        let help_lines = if cfg.language == "en" {
            vec![
                Line::from(Span::styled(text.help_sec_nav, Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))),
                Line::from("  ↑ / ↓ (j / k)   — Move up and down the list"),
                Line::from("  Enter / → (l)   — Open directory / Dive deeper"),
                Line::from("  Esc / ← (h)     — Move up one directory level"),
                Line::from("  Tab             — Switch panel focus (Left Tree ↔ Right Content)"),
                Line::from("  q               — Gracefully exit application"),
                Line::from(""),
                Line::from(Span::styled(text.help_sec_search, Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))),
                Line::from("  /               — Open context-aware text search"),
                Line::from("  t               — Open strict context-aware tag search"),
                Line::from(""),
                Line::from(Span::styled(text.help_sec_db, Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))),
                Line::from("  w               — Open interactive database/workspace switcher"),
                Line::from("  Shift + N       — (Inside switcher) Create a brand new workspace"),
                Line::from(""),
                Line::from(Span::styled(text.help_sec_crud, Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))),
                Line::from("  + (or =)        — Create new directory or file from smart template"),
                Line::from("  -               — Delete selected directory, file, or header"),
                Line::from("  F4              — Suspend TUI and edit notes instantly in Vim"),
                Line::from("  i               — Quick note stream directly into Inbox.md"),
                Line::from("  F3              — Export current note as a clean .txt file"),
                Line::from(""),
                Line::from(Span::styled(text.help_sec_sys, Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))),
                Line::from("  F2              — Hot-swap language (RU / EN)"),
                Line::from("  Ctrl + S        — Open full interactive configuration panel"),
                Line::from("  ?               — Toggle this help reference window"),
            ]
        } else {
            vec![
                Line::from(Span::styled(text.help_sec_nav, Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))),
                Line::from("  ↑ / ↓ (j / k)   — Перемещение вверх и вниз по списку"),
                Line::from("  Enter / → (l)   — Открыть папку / Спуститься глубже"),
                Line::from("  Esc / ← (h)     — Подняться на уровень выше по иерархии"),
                Line::from("  Tab             — Переключить фокус ввода (Левая панель ↔ Правая)"),
                Line::from("  q               — Корректный выход из программы"),
                Line::from(""),
                Line::from(Span::styled(text.help_sec_search, Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))),
                Line::from("  /               — Запуск контекстного поиска по названиям"),
                Line::from("  t               — Запуск контекстного поиска СТРОГО по тегам"),
                Line::from(""),
                Line::from(Span::styled(text.help_sec_db, Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))),
                Line::from("  w               — Открыть интерактивное меню выбора баз знаний"),
                Line::from("  Shift + N       — (Внутри меню баз) Создать новую базу на диске"),
                Line::from(""),
                Line::from(Span::styled(text.help_sec_crud, Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))),
                Line::from("  + (или =)       — Создать подпапку или файл темы (.md шаблон)"),
                Line::from("  -               — Удалить выбранную папку, файл или совет"),
                Line::from("  F4              — Свернуть TUI и открыть файл в Vim для правок"),
                Line::from("  i               — Быстрая потоковая запись мысли в Inbox.md"),
                Line::from("  F3              — Экспорт совета в чистый текстовый .txt файл"),
                Line::from(""),
                Line::from(Span::styled(text.help_sec_sys, Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))),
                Line::from("  F2              — Быстрая «на лету» смена языка (RU / EN)"),
                Line::from("  Ctrl + S        — Открыть полноценное меню настроек программы"),
                Line::from("  ?               — Открыть/закрыть это окно справки"),
            ]
        };

        let help_paragraph = Paragraph::new(help_lines)
            .block(help_block)
            .wrap(Wrap { trim: false })
            .scroll((help_scroll_index, 0));

        let area = centered_rect(65, 60, f.size());
        f.render_widget(Clear, area);
        f.render_widget(help_paragraph, area);
    }

    // --- СТАНДАРТНЫЕ МОДАЛЬНЫЕ ОКНА ---
    if *state == InputState::Creating || *state == InputState::ConfirmDelete {
        let popup_block = Block::default()
            .title(" Уведомление ")
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::LightRed));

        let prompt = match state {
            InputState::Creating => format!(" {}\n > {}", text.create_prompt, buffer),
            InputState::ConfirmDelete => format!(" {}\n > {}", text.delete_confirm, buffer),
            _ => String::new(),
        };

        let popup_paragraph = Paragraph::new(prompt).block(popup_block);
        let area = centered_rect(50, 15, f.size());
        f.render_widget(Clear, area);
        f.render_widget(popup_paragraph, area);
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    let final_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1]); // ИСПРАВЛЕНО: Извлекаем центральный элемент по индексу

    final_layout[1] // ИСПРАВЛЕНО: Возвращаем итоговый центр по индексу
}
