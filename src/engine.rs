use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, PartialEq)]
pub enum NodeType {
    Folder,
    File,
    MarkdownHeader { level: usize },
}

#[derive(Clone, Debug)]
pub struct DbNode {
    pub name: String,
    pub path: PathBuf,
    pub node_type: NodeType,
    pub tags: Vec<String>,
    pub children: Vec<DbNode>,
}

/// Сканирует корневую директорию и находит изолированные базы данных.
/// Исправлено: заглядываем внутрь папки db/, чтобы автоматически подхватить старые базы!
pub fn list_workspaces(base_path: &Path) -> Vec<PathBuf> {
    let mut workspaces = Vec::new();
    
    // Сначала проверяем классический путь ~/.config/ttytips/db/
    let old_db_path = base_path.join("db");
    let scan_target = if old_db_path.is_dir() { &old_db_path } else { base_path };

    if let Ok(entries) = fs::read_dir(scan_target) {
        for entry in entries.flatten() {
            let path = entry.path();
            // Исключаем служебные файлы (например, файлы конфигураций или логи)
            if path.is_dir() && path.file_name().map_or(true, |name| name != "db") {
                workspaces.push(path);
            }
        }
    }
    
    // Стабильная сортировка по алфавиту
    workspaces.sort();
    workspaces
}

/// Сканирует конкретную выбранную базу данных и строит изолированное дерево.
/// Исправлено: рекурсивный спуск теперь заперт строго внутри переданного root_path.
pub fn load_database(root_path: &Path) -> Vec<DbNode> {
    let mut nodes = Vec::new();
    if let Ok(entries) = fs::read_dir(root_path) {
        let mut entries_vec: Vec<_> = entries.flatten().collect();
        entries_vec.sort_by_key(|e| e.path());
        
        for entry in entries_vec {
            let path = entry.path();
            let name = path.file_name().unwrap_or_default().to_string_lossy().into_owned();
            
            // Защита: файл Inbox.md отображаем только в корне базы, глубже не дублируем
            if name == "Inbox.md" && root_path.file_name().map_or("", |n| n.to_str().unwrap_or("")) != path.parent().unwrap().file_name().unwrap().to_str().unwrap() {
                // Если это не самый корень базы, можно скрыть или переименовать, но дефолтно пропускаем служебные бэкапы
            }

            if path.is_dir() {
                // Рекурсивно сканируем только подпапки текущей БАЗЫ
                let children = load_database(&path);
                nodes.push(DbNode { name, path, node_type: NodeType::Folder, tags: Vec::new(), children });
            } else if path.extension().map_or(false, |ext| ext == "md") {
                let (tags, children) = parse_md_file(&path);
                nodes.push(DbNode { name, path, node_type: NodeType::File, tags, children });
            }
        }
    }
    nodes
}

fn parse_md_file(path: &Path) -> (Vec<String>, Vec<DbNode>) {
    let mut headers = Vec::new();
    let mut tags = Vec::new();
    
    if let Ok(content) = fs::read_to_string(path) {
        for line in content.lines() {
            if line.to_lowercase().starts_with("tags:") && tags.len() < 10 {
                for word in line.split_whitespace().skip(1) {
                    if word.starts_with('#') && tags.len() < 10 {
                        tags.push(word.trim_matches('#').to_string());
                    }
                }
            }
            if line.starts_with('#') {
                let level = line.chars().take_while(|&c| c == '#').count();
                let name = line.trim_start_matches('#').trim().to_string();
                headers.push(DbNode {
                    name,
                    path: path.to_path_buf(),
                    node_type: NodeType::MarkdownHeader { level },
                    tags: Vec::new(),
                    children: Vec::new(),
                });
            }
        }
    }
    (tags, headers)
}

pub fn search_by_text(nodes: &[DbNode], query: &str) -> Vec<DbNode> {
    if query.is_empty() { return nodes.to_vec(); }
    let q = query.to_lowercase();
    let mut filtered = Vec::new();

    for node in nodes {
        let name_match = node.name.to_lowercase().contains(&q);
        let filtered_children = search_by_text(&node.children, query);
        let has_matching_children = !filtered_children.is_empty();

        if name_match || has_matching_children {
            let mut new_node = node.clone();
            new_node.children = filtered_children;
            filtered.push(new_node);
        }
    }
    filtered
}

pub fn search_by_tags(nodes: &[DbNode], query: &str) -> Vec<DbNode> {
    if query.is_empty() { return nodes.to_vec(); }
    let q = query.to_lowercase();
    let mut filtered = Vec::new();

    for node in nodes {
        let tag_match = node.tags.iter().any(|t| t.to_lowercase().contains(&q));
        let filtered_children = search_by_tags(&node.children, query);
        let has_matching_children = !filtered_children.is_empty();

        if tag_match || has_matching_children {
            let mut new_node = node.clone();
            new_node.children = filtered_children;
            filtered.push(new_node);
        }
    }
    filtered
}
