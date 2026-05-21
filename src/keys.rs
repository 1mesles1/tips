use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use std::time::Duration;

pub enum Action {
    MoveUp,
    MoveDown,
    MoveLeft,
    MoveRight,
    Select,
    GoBack,
    Quit,
    ToggleLanguage,
    ToggleFocus,
    StartTextSearch,
    StartTagSearch,
    EditItem,
    ExportItem,
    CreateItem,
    DeleteItem,
    OpenWorkspaceSwitcher,
    CreateWorkspace,
    StartQuickInbox,
    OpenSettingsMenu,
    OpenHelpMenu,      // Новое: действие для открытия справки
    InputChar(char),
    Backspace,
    Submit,
    Cancel,
    None,
}

pub fn poll_action(is_input_mode: bool) -> anyhow::Result<Action> {
    if event::poll(Duration::from_millis(16))? {
        if let Event::Key(KeyEvent { code, modifiers, .. }) = event::read()? {
            if is_input_mode {
                match code {
                    KeyCode::Up => return Ok(Action::MoveUp),
                    KeyCode::Down => return Ok(Action::MoveDown),
                    KeyCode::Left => return Ok(Action::MoveLeft),
                    KeyCode::Right => return Ok(Action::MoveRight),
                    KeyCode::Enter => return Ok(Action::Submit),
                    KeyCode::Esc => return Ok(Action::Cancel),
                    KeyCode::Backspace => return Ok(Action::Backspace),
                    KeyCode::Char(c) => return Ok(Action::InputChar(c)),
                    _ => {}
                }
            } else {
                // Ctrl + S для настроек
                if modifiers.contains(KeyModifiers::CONTROL) && (code == KeyCode::Char('s') || code == KeyCode::Char('ы')) {
                    return Ok(Action::OpenSettingsMenu);
                }

                match code {
                    KeyCode::Char('q') | KeyCode::Char('й') => return Ok(Action::Quit),
                    KeyCode::Tab => return Ok(Action::ToggleFocus),
                    KeyCode::Up | KeyCode::Char('k') => return Ok(Action::MoveUp),
                    KeyCode::Down | KeyCode::Char('j') => return Ok(Action::MoveDown),
                    KeyCode::Left | KeyCode::Char('h') => return Ok(Action::GoBack),
                    KeyCode::Right | KeyCode::Char('l') => return Ok(Action::Select),
                    KeyCode::Enter => return Ok(Action::Select),
                    KeyCode::Esc => return Ok(Action::GoBack),
                    KeyCode::F(2) => return Ok(Action::ToggleLanguage),
                    KeyCode::Char('/') => return Ok(Action::StartTextSearch),
                    KeyCode::Char('t') | KeyCode::Char('е') => return Ok(Action::StartTagSearch),
                    KeyCode::Char('w') | KeyCode::Char('ц') => return Ok(Action::OpenWorkspaceSwitcher),
                    KeyCode::Char('N') | KeyCode::Char('Т') => return Ok(Action::CreateWorkspace),
                    KeyCode::Char('i') | KeyCode::Char('ш') => return Ok(Action::StartQuickInbox),
                    KeyCode::Char('?') | KeyCode::Char(',') => return Ok(Action::OpenHelpMenu), // Клавиша '?' (или ',' в русской раскладке)
                    KeyCode::F(4) => return Ok(Action::EditItem),
                    KeyCode::F(3) => return Ok(Action::ExportItem),
                    KeyCode::Char('+') | KeyCode::Char('=') => return Ok(Action::CreateItem),
                    KeyCode::Char('-') => return Ok(Action::DeleteItem),
                    _ => {}
                }
            }
        }
    }
    Ok(Action::None)
}
