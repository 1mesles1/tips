mod config;
mod engine;
mod i18n;
mod keys;
mod ui;

use std::fs;
use std::io::stdout;
use std::path::{Path, PathBuf};
use std::process::Command;

use crossterm::{
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::prelude::*;

use crate::config::Config;
use crate::engine::DbNode;
use crate::i18n::{Language, get_bundle};
use crate::keys::{Action, poll_action};
use crate::ui::{InputState, draw_ui};

fn main() -> anyhow::Result<()> {
    // =========================================================================
    // ДОБАВЛЕНО: ОБРАБОТКА АРГУМЕНТОВ КОМАНДНОЙ СТРОКИ (-v / --version)
    // =========================================================================
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 {
        if args[1] == "-v" || args[1] == "--version" {
            println!("v0.0.1");
            return Ok(());
        }
    }
    // =========================================================================

    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;

    // 1. ЗАГРУЗКА КОНФИГУРАЦИИ
    let mut cfg = Config::load();
    let mut current_lang = if cfg.language == "en" { Language::EN } else { Language::RU };
    let mut text = get_bundle(current_lang);
    
    let mut base_dir = PathBuf::from(&cfg.db_root_path);
    fs::create_dir_all(&base_dir)?;

    // 2. ИНИЦИАЛИЗАЦИЯ РАБОЧИХ БАЗ (WORKSPACES)
    let mut workspaces = engine::list_workspaces(&base_dir);
    if workspaces.is_empty() {
        let default_db = base_dir.join("Main_DB");
        fs::create_dir_all(&default_db)?;
        workspaces = engine::list_workspaces(&base_dir);
    }

    let mut active_workspace_idx = 0;
    let mut db_path = workspaces[active_workspace_idx].clone();

    let mut inbox_path = db_path.join("Inbox.md");
    if !inbox_path.exists() {
        fs::write(&inbox_path, "Tags: #inbox #new\n\n# Входящие заметки\n")?;
    }

    // Состояния навигации и интерфейса
    let mut history_stack: Vec<(Vec<DbNode>, usize)> = Vec::new();
    let mut current_nodes = engine::load_database(&db_path);
    let mut filtered_nodes = current_nodes.clone();
    
    let mut selected_index = 0;
    let mut input_mode = InputState::Normal;
    let mut buffer = String::new();

    let mut right_panel_focused = false;
    let mut text_scroll_index = 0u16;
    
    // Индексы для меню
    let mut settings_row_idx = 0;
    let mut help_scroll_index = 0u16;

    loop {
        terminal.draw(|f| {
            let is_searching = input_mode == InputState::TextSearching || input_mode == InputState::TagSearching;
            let display_nodes = if is_searching { &filtered_nodes } else { &current_nodes };
            
            let workspace_names: Vec<String> = workspaces
                .iter()
                .map(|p| p.file_name().unwrap_or_default().to_string_lossy().into_owned())
                .collect();

            draw_ui(
                f, 
                display_nodes, 
                selected_index, 
                &text, 
                &buffer, 
                &input_mode, 
                right_panel_focused, 
                text_scroll_index,
                &workspace_names,
                active_workspace_idx,
                &cfg,
                settings_row_idx,
                help_scroll_index,
            );
        })?;

        // Флаг режима ввода текста
        let is_input = input_mode == InputState::TextSearching 
            || input_mode == InputState::TagSearching 
            || input_mode == InputState::Creating 
            || input_mode == InputState::ConfirmDelete 
            || input_mode == InputState::WorkspaceCreating
            || input_mode == InputState::QuickInboxInput
            || input_mode == InputState::SettingsPathInput;

        match poll_action(is_input)? {
            Action::Quit => if input_mode == InputState::Normal { break; },
            
            Action::ToggleLanguage => {
                current_lang = if current_lang == Language::RU { Language::EN } else { Language::RU };
                cfg.language = if current_lang == Language::RU { "ru".to_string() } else { "en".to_string() };
                text = get_bundle(current_lang);
                cfg.save();
            }
            
            Action::ToggleFocus => {
                if input_mode == InputState::Normal && !current_nodes.is_empty() {
                    right_panel_focused = !right_panel_focused;
                    text_scroll_index = 0;
                }
            }

            Action::StartTextSearch => {
                if !right_panel_focused && input_mode == InputState::Normal {
                    input_mode = InputState::TextSearching;
                    buffer.clear();
                }
            }

            Action::StartTagSearch => {
                if !right_panel_focused && input_mode == InputState::Normal {
                    input_mode = InputState::TagSearching;
                    buffer.clear();
                }
            }

            Action::CreateItem => {
                if !right_panel_focused && input_mode == InputState::Normal {
                    input_mode = InputState::Creating;
                    buffer.clear();
                }
            }

            Action::DeleteItem => {
                if !right_panel_focused && !current_nodes.is_empty() && input_mode == InputState::Normal {
                    input_mode = InputState::ConfirmDelete;
                    buffer.clear();
                }
            }

            Action::OpenWorkspaceSwitcher => {
                if input_mode == InputState::Normal {
                    input_mode = InputState::WorkspaceSwitching;
                }
            }

            Action::CreateWorkspace => {
                if input_mode == InputState::WorkspaceSwitching {
                    input_mode = InputState::WorkspaceCreating;
                    buffer.clear();
                }
            }

            Action::StartQuickInbox => {
                if input_mode == InputState::Normal {
                    input_mode = InputState::QuickInboxInput;
                    buffer.clear();
                }
            }

            Action::OpenSettingsMenu => {
                if input_mode == InputState::Normal {
                    input_mode = InputState::SettingsMenu;
                    settings_row_idx = 0;
                }
            }

            Action::OpenHelpMenu => {
                if input_mode == InputState::Normal {
                    input_mode = InputState::HelpMenu;
                    help_scroll_index = 0;
                }
            }

            Action::EditItem => {
                if !current_nodes.is_empty() && input_mode == InputState::Normal {
                    let node = &current_nodes[selected_index];
                    
                    disable_raw_mode()?;
                    stdout().execute(LeaveAlternateScreen)?;

                    let editor = cfg.editor.clone();

                    if Command::new(&editor).arg(&node.path).status().is_err() {
                        let _ = Command::new("nano").arg(&node.path).status();
                    }

                    enable_raw_mode()?;
                    stdout().execute(EnterAlternateScreen)?;
                    terminal.clear()?;

                    current_nodes = engine::load_database(&db_path);
                }
            }

            Action::ExportItem => {
                if !current_nodes.is_empty() && input_mode == InputState::Normal {
                    let node = &current_nodes[selected_index];
                    let export_path = db_path.join(format!("{}_export.txt", node.name));
                    export_clean_text(&node.path, &export_path)?;
                }
            }

            Action::MoveUp => {
                if input_mode == InputState::WorkspaceSwitching {
                    if active_workspace_idx > 0 { active_workspace_idx -= 1; }
                } else if input_mode == InputState::SettingsMenu {
                    if settings_row_idx > 0 { settings_row_idx -= 1; }
                } else if input_mode == InputState::HelpMenu {
                    help_scroll_index = help_scroll_index.saturating_sub(1);
                } else if right_panel_focused {
                    text_scroll_index = text_scroll_index.saturating_sub(1);
                } else {
                    if selected_index > 0 { 
                        selected_index -= 1; 
                        text_scroll_index = 0;
                    }
                }
            }
            
            Action::MoveDown => {
                if input_mode == InputState::WorkspaceSwitching {
                    if active_workspace_idx < workspaces.len() - 1 { active_workspace_idx += 1; }
                } else if input_mode == InputState::SettingsMenu {
                    if settings_row_idx < 3 { settings_row_idx += 1; } 
                } else if input_mode == InputState::HelpMenu {
                    help_scroll_index = help_scroll_index.saturating_add(1);
                } else if right_panel_focused {
                    text_scroll_index = text_scroll_index.saturating_add(1);
                } else {
                    let is_searching = input_mode == InputState::TextSearching || input_mode == InputState::TagSearching;
                    let limit = if is_searching { filtered_nodes.len() } else { current_nodes.len() };
                    if limit > 0 && selected_index < limit - 1 { 
                        selected_index += 1; 
                        text_scroll_index = 0;
                    }
                }
            }

            Action::MoveRight | Action::MoveLeft | Action::Select => {
                if input_mode == InputState::SettingsMenu {
                    if settings_row_idx == 0 {
                        cfg.language = if cfg.language == "ru" { "en".to_string() } else { "ru".to_string() };
                        current_lang = if cfg.language == "ru" { Language::RU } else { Language::EN };
                        text = get_bundle(current_lang);
                    } else if settings_row_idx == 1 {
                        cfg.editor = match cfg.editor.as_str() {
                            "nano" => "vi".to_string(),
                            "vi" => "vim".to_string(),
                            "vim" => "neovim".to_string(),
                            _ => "nano".to_string(),
                        };
                    } else if settings_row_idx == 2 {
                        input_mode = InputState::SettingsPathInput;
                        buffer = cfg.db_root_path.clone();
                    } else if settings_row_idx == 3 {
                        cfg.save();
                        input_mode = InputState::Normal;
                    }
                } else if !right_panel_focused && input_mode == InputState::Normal {
                    let selected = &current_nodes[selected_index];
                    if !selected.children.is_empty() {
                        history_stack.push((current_nodes.clone(), selected_index));
                        current_nodes = selected.children.clone();
                        filtered_nodes = current_nodes.clone();
                        selected_index = 0;
                        text_scroll_index = 0;
                    }
                } else if input_mode == InputState::WorkspaceSwitching {
                    db_path = workspaces[active_workspace_idx].clone();
                    inbox_path = db_path.join("Inbox.md");
                    history_stack.clear();
                    current_nodes = engine::load_database(&db_path);
                    selected_index = 0;
                    text_scroll_index = 0;
                    input_mode = InputState::Normal;
                }
            }

            Action::GoBack => {
                if input_mode == InputState::SettingsMenu || input_mode == InputState::WorkspaceSwitching || input_mode == InputState::HelpMenu {
                    input_mode = InputState::Normal;
                } else if right_panel_focused {
                    right_panel_focused = false;
                } else {
                    if let Some((prev_nodes, prev_index)) = history_stack.pop() {
                        current_nodes = prev_nodes;
                        filtered_nodes = current_nodes.clone();
                        selected_index = prev_index;
                        text_scroll_index = 0;
                    }
                }
            }

            Action::InputChar(c) => {
                buffer.push(c);
                if input_mode == InputState::TextSearching {
                    filtered_nodes = engine::search_by_text(&current_nodes, &buffer);
                } else if input_mode == InputState::TagSearching {
                    filtered_nodes = engine::search_by_tags(&current_nodes, &buffer);
                }
            }
            Action::Backspace => {
                buffer.pop();
                if input_mode == InputState::TextSearching {
                    filtered_nodes = engine::search_by_text(&current_nodes, &buffer);
                } else if input_mode == InputState::TagSearching {
                    filtered_nodes = engine::search_by_tags(&current_nodes, &buffer);
                }
            }
            Action::Cancel => {
                if input_mode == InputState::SettingsPathInput {
                    input_mode = InputState::SettingsMenu;
                } else {
                    input_mode = InputState::Normal;
                }
                buffer.clear();
            }
            Action::Submit => {
                match input_mode {
                    InputState::SettingsPathInput => {
                        if !buffer.is_empty() {
                            cfg.db_root_path = buffer.clone();
                            base_dir = PathBuf::from(&cfg.db_root_path);
                            let _ = fs::create_dir_all(&base_dir);
                            
                            workspaces = engine::list_workspaces(&base_dir);
                            if workspaces.is_empty() {
                                let default_db = base_dir.join("Main_DB");
                                let _ = fs::create_dir_all(&default_db);
                                workspaces = engine::list_workspaces(&base_dir);
                            }
                            active_workspace_idx = 0;
                            db_path = workspaces[active_workspace_idx].clone();
                            inbox_path = db_path.join("Inbox.md");
                            history_stack.clear();
                            current_nodes = engine::load_database(&db_path);
                            selected_index = 0;
                        }
                        input_mode = InputState::SettingsMenu;
                    }
                    InputState::WorkspaceCreating => {
                        if !buffer.is_empty() {
                            let new_ws_path = base_dir.join(&buffer);
                            let _ = fs::create_dir_all(&new_ws_path);
                            let new_inbox = new_ws_path.join("Inbox.md");
                            let _ = fs::write(&new_inbox, "Tags: #inbox #new\n\n# Входящие заметки\n");

                            workspaces = engine::list_workspaces(&base_dir);
                            if let Some(pos) = workspaces.iter().position(|p| p == &new_ws_path) {
                                active_workspace_idx = pos;
                            }
                            db_path = new_ws_path;
                            inbox_path = db_path.join("Inbox.md");
                            history_stack.clear();
                            current_nodes = engine::load_database(&db_path);
                            selected_index = 0;
                            text_scroll_index = 0;
                        }
                        input_mode = InputState::Normal;
                    }
                    InputState::Creating => {
                        if !buffer.is_empty() {
                            let new_path = db_path.join(&buffer);
                            if buffer.ends_with(".md") {
                                let clean_title = buffer.trim_end_matches(".md");
                                let template = format!(
                                    "Tags: #change_me #general\n\n# {}\n## Краткое описание совета\n`вставьте_команду_сюда`\n",
                                    clean_title
                                );
                                let _ = fs::write(new_path, template);
                            } else {
                                let _ = fs::create_dir_all(new_path);
                            }
                            current_nodes = engine::load_database(&db_path);
                        }
                        input_mode = InputState::Normal;
                    }
                    InputState::QuickInboxInput => {
                        if !buffer.is_empty() {
                            use std::fs::OpenOptions;
                            use std::io::Write;
                            if let Ok(mut file) = OpenOptions::new().append(true).open(&inbox_path) {
                                let _ = writeln!(file, "\n## Заметка: {}", buffer);
                                let _ = writeln!(file, "`введите детали команды позже через Vim`");
                            }
                            current_nodes = engine::load_database(&db_path);
                        }
                        input_mode = InputState::Normal;
                    }
                    InputState::ConfirmDelete => {
                        if buffer.to_lowercase() == "y" {
                            let node = &current_nodes[selected_index];
                            if node.path.is_dir() {
                                let _ = fs::remove_dir_all(&node.path);
                            } else {
                                let _ = fs::remove_file(&node.path);
                            }
                            current_nodes = engine::load_database(&db_path);
                            selected_index = 0;
                        }
                        input_mode = InputState::Normal;
                    }
                    _ => { input_mode = InputState::Normal; }
                }
                buffer.clear();
            }
            Action::None => {}
        }
    }

    disable_raw_mode()?;
    stdout().execute(LeaveAlternateScreen)?;
    Ok(())
}

fn export_clean_text(src: &Path, dest: &Path) -> std::io::Result<()> {
    if src.is_file() {
        let content = fs::read_to_string(src)?;
        let mut clean_lines = Vec::new();
        for line in content.lines() {
            let clean = line.trim_start_matches('#').replace('`', "").trim().to_string();
            if !clean.is_empty() && !clean.to_lowercase().starts_with("tags:") {
                clean_lines.push(clean);
            }
        }
        fs::write(dest, clean_lines.join("\n"))?;
    }
    Ok(())
}
