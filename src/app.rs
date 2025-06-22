use ratatui::widgets::ListState;
// use rayon::prelude::*;
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, Mutex},
    thread,
    time::{Duration, Instant},
};
use walkdir::WalkDir;

const MAX_RESULTS: usize = 100;
const DEBOUNCE_MS: u64 = 100; // Reduced for better responsiveness
const MAX_FILES: usize = 50_000;

#[derive(Clone, Debug)]
pub struct FileEntry {
    pub path: String,
    pub filename: String,
    pub score: u32,
}

pub struct AppState {
    pub all_files: Vec<FileEntry>,
    pub matches: Vec<FileEntry>,
    pub input: String,
    pub selected: usize,
    pub loading: bool,
    pub last_update: Instant,
    pub search_cache: HashMap<String, Vec<FileEntry>>,
    pub input_changed: bool,
    pub list_state: ListState,
}

impl AppState {
    pub fn new() -> Arc<Mutex<Self>> {
        let app_state = Arc::new(Mutex::new(AppState {
            all_files: Vec::new(),
            matches: Vec::new(),
            input: String::new(),
            selected: 0,
            loading: true,
            last_update: Instant::now(),
            search_cache: HashMap::new(),
            input_changed: true, // Set to true to trigger initial search
            list_state: ListState::default(),
        }));

        let app_clone = Arc::clone(&app_state);
        thread::spawn(move || {
            let files = AppState::load_files();
            let mut app = app_clone.lock().unwrap();
            app.all_files = files;
            app.loading = false;
            app.update_matches(); // Perform initial match
        });

        app_state
    }

    fn load_files() -> Vec<FileEntry> {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/home/charlie"));
        let ignore_dirs = [
            ".git",
            "node_modules",
            "target",
            ".cargo",
            ".rustup",
            "__pycache__",
            ".vscode",
            ".idea",
            "build",
            "dist",
            ".cache",
            ".local/share",
            ".steam",
        ];

        WalkDir::new(home)
            .max_depth(5)
            .into_iter()
            .filter_entry(|e| {
                let path_str = e.path().to_string_lossy();
                !ignore_dirs.iter().any(|&dir| path_str.contains(dir))
            })
            .filter_map(Result::ok)
            .filter(|e| e.file_type().is_file())
            .take(MAX_FILES)
            .map(|entry| {
                let path = entry.path().display().to_string();
                let filename = entry.file_name().to_string_lossy().to_string();
                FileEntry {
                    path,
                    filename,
                    score: 0,
                }
            })
            .collect()
    }

    pub fn update_matches(&mut self) {
        if self.input.is_empty() {
            self.matches = self.all_files.iter().take(MAX_RESULTS).cloned().collect();
            self.selected = 0;
            return;
        }

        if let Some(cached) = self.search_cache.get(&self.input) {
            self.matches = cached.clone();
            self.selected = self.selected.min(cached.len().saturating_sub(1));
            return;
        }

        let input_lower = self.input.to_lowercase();
        let mut scored_matches: Vec<FileEntry> = self
            .all_files
            .iter() // Using standard iterator for simplicity
            .filter_map(|file| {
                let score = calculate_fuzzy_score(&file.filename.to_lowercase(), &input_lower);
                if score > 0 {
                    Some(FileEntry {
                        path: file.path.clone(),
                        filename: file.filename.clone(),
                        score,
                    })
                } else {
                    None
                }
            })
            .collect();

        scored_matches.sort_unstable_by(|a, b| b.score.cmp(&a.score));
        scored_matches.truncate(MAX_RESULTS);

        if self.search_cache.len() > 100 {
            // Cache pruning
            self.search_cache.clear();
        }
        self.search_cache
            .insert(self.input.clone(), scored_matches.clone());

        self.matches = scored_matches;
        self.selected = 0;
    }

    pub fn should_update(&self) -> bool {
        self.input_changed && self.last_update.elapsed() > Duration::from_millis(DEBOUNCE_MS)
    }

    pub fn mark_updated(&mut self) {
        self.last_update = Instant::now();
        self.input_changed = false;
    }

    pub fn mark_input_changed(&mut self) {
        self.input_changed = true;
        self.last_update = Instant::now();
    }

    pub fn move_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    pub fn move_down(&mut self) {
        if !self.matches.is_empty() && self.selected + 1 < self.matches.len() {
            self.selected += 1;
        }
    }

    pub fn get_selected_path(&self) -> Option<&str> {
        self.matches.get(self.selected).map(|f| f.path.as_str())
    }
}

// Optimized fuzzy matching algorithm
fn calculate_fuzzy_score(text: &str, pattern: &str) -> u32 {
    if pattern.is_empty() {
        return 1;
    }
    if text.is_empty() {
        return 0;
    }

    if let Some(pos) = text.find(pattern) {
        return (1000 - pos as u32).saturating_sub((text.len() - pattern.len()) as u32);
    }

    let mut score = 0;
    let mut pattern_idx = 0;
    let pattern_chars: Vec<char> = pattern.chars().collect();
    let text_chars: Vec<char> = text.chars().collect();
    let mut last_match_idx = 0;
    let mut consecutive = 0;

    for (text_idx, &text_char) in text_chars.iter().enumerate() {
        if pattern_idx < pattern_chars.len()
            && text_char
                .to_lowercase()
                .eq(pattern_chars[pattern_idx].to_lowercase())
        {
            score += 1;
            if text_idx == last_match_idx + 1 {
                consecutive += 1;
                score += consecutive * 5;
            } else {
                consecutive = 0;
            }
            if text_idx == 0
                || matches!(
                    text_chars.get(text_idx - 1),
                    Some(&'-') | Some(&'_') | Some(&' ') | Some(&'.')
                )
            {
                score += 10;
            }
            last_match_idx = text_idx;
            pattern_idx += 1;
        }
    }

    if pattern_idx == pattern_chars.len() {
        score
    } else {
        0
    }
}
