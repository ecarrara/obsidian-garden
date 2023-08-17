use petgraph::prelude::{NodeIndex, StableGraph};
use serde::Serialize;
use std::{
    collections::{HashMap, HashSet, VecDeque},
    fmt::Display,
    path::{Path, PathBuf},
};
use walkdir::WalkDir;

use crate::note::Note;

pub(crate) struct VaultBuilder {
    pub directory: PathBuf,
    tags: Option<Vec<String>>,
}

impl VaultBuilder {
    pub fn new<P: AsRef<Path>>(directory: P) -> Self {
        Self {
            directory: directory.as_ref().to_path_buf(),
            tags: None,
        }
    }

    pub fn build(self) -> Vault {
        let mut notes: HashMap<ItemPath, NoteItem> = HashMap::new();
        let mut graph = StableGraph::new();
        let mut files: HashMap<ItemPath, EmbeddedFile> = HashMap::new();

        const MARKDOWN_FILE_EXTENSIONS: &[&str] = &[".md"];
        const IMAGE_FILE_EXTENSIONS: &[&str] =
            &[".png", ".webp", ".jpg", ".jpeg", ".gif", ".bmp", ".svg"];
        const AUDIO_FILE_EXTENSIONS: &[&str] =
            &[".mp3", ".webm", ".wav", ".m4a", ".ogg", ".3gp", ".flac"];
        const VIDEO_FILE_EXTENSIONS: &[&str] = &[".mp4", ".webm", ".ogv", ".mov", ".mkv"];
        const PDF_FILE_EXTENSIONS: &[&str] = &[".pdf"];

        for result in WalkDir::new(&self.directory) {
            match result {
                Ok(entry) => {
                    if !entry.file_type().is_file() {
                        continue;
                    }

                    let filename = entry.file_name();

                    // path relative to vault root directory
                    let relative_path = entry.path().strip_prefix(&self.directory).unwrap();

                    if MARKDOWN_FILE_EXTENSIONS
                        .iter()
                        .any(|ext| filename.to_string_lossy().ends_with(ext))
                    {
                        match Note::from_file(&entry.path()) {
                            Ok(note) => {
                                if let Some(tags) = &self.tags {
                                    if !note.tags.iter().any(|t| tags.contains(t)) {
                                        continue;
                                    }
                                }
                                let note_path = ItemPath::from_path_without_ext(relative_path);
                                let index = graph.add_node(note_path.clone());
                                notes.insert(note_path, NoteItem { index, note });
                            }
                            Err(err) => {
                                eprintln!("Unable to parse {}: {}", entry.path().display(), err)
                            }
                        }
                    } else if IMAGE_FILE_EXTENSIONS
                        .iter()
                        .any(|ext| filename.to_string_lossy().ends_with(ext))
                    {
                        let item_path = ItemPath::from_path(relative_path);
                        files.insert(item_path, EmbeddedFile::Image(entry.path().to_path_buf()));
                    } else if AUDIO_FILE_EXTENSIONS
                        .iter()
                        .any(|ext| filename.to_string_lossy().ends_with(ext))
                    {
                        let item_path = ItemPath::from_path(relative_path);
                        files.insert(item_path, EmbeddedFile::Audio(entry.path().to_path_buf()));
                    } else if VIDEO_FILE_EXTENSIONS
                        .iter()
                        .any(|ext| filename.to_string_lossy().ends_with(ext))
                    {
                        let item_path = ItemPath::from_path(relative_path);
                        files.insert(item_path, EmbeddedFile::Video(entry.path().to_path_buf()));
                    } else if PDF_FILE_EXTENSIONS
                        .iter()
                        .any(|ext| filename.to_string_lossy().ends_with(ext))
                    {
                        let item_path = ItemPath::from_path(relative_path);
                        files.insert(item_path, EmbeddedFile::Pdf(entry.path().to_path_buf()));
                    }
                }
                Err(err) => eprintln!("{}", err),
            }
        }

        for item in notes.values() {
            for link in item.note.links.iter() {
                if let Some((found, _)) = resolve_link(&notes, &link.target) {
                    graph.add_edge(item.index, notes[&found].index, ());
                }
            }
        }

        Vault {
            notes,
            graph,
            files,
            root: self.directory,
        }
    }

    pub(crate) fn filter_tags(&mut self, tags: Vec<String>) -> &mut Self {
        self.tags = Some(tags);
        self
    }
}

pub(crate) struct Vault {
    pub notes: HashMap<ItemPath, NoteItem>,
    graph: StableGraph<ItemPath, ()>,
    pub(crate) root: PathBuf,
    pub(crate) files: HashMap<ItemPath, EmbeddedFile>,
}

impl Vault {
    pub(crate) fn get_note(&self, note_path: &ItemPath) -> Option<&Note> {
        self.notes.get(note_path).map(|item| &item.note)
    }

    pub(crate) fn local_graph(
        &self,
        path: &ItemPath,
        max_depth: usize,
    ) -> Option<StableGraph<ItemPath, ()>> {
        let mut depth = 0;
        let mut stack = VecDeque::new();
        let mut discovered: HashSet<ItemPath> = HashSet::new();
        let mut path_indexes: HashMap<ItemPath, NodeIndex> = HashMap::new();

        stack.push_back(path.clone());

        let mut g = StableGraph::new();

        while let Some(node) = stack.pop_front() {
            if depth > max_depth || discovered.contains(&node) {
                continue;
            }

            discovered.insert(node.clone());

            let origin = *path_indexes
                .entry(node.clone())
                .or_insert_with(|| g.add_node(node.clone()));

            for succ in self.graph.neighbors(self.notes[&node].index) {
                let succ_path = &self.graph[succ];
                stack.push_back(succ_path.clone());

                let target = *path_indexes
                    .entry(succ_path.clone())
                    .or_insert_with(|| g.add_node(succ_path.clone()));

                g.update_edge(origin, target, ());
            }

            depth += 1;
        }

        Some(g)
    }

    pub(crate) fn resolve_link<S: Into<String>>(&self, target: S) -> Option<ItemPath> {
        let (item_path, _) = resolve_link(&self.notes, target)?;
        Some(item_path)
    }

    pub(crate) fn resolve_embedded_link<S: Into<String>>(
        &self,
        target: S,
    ) -> Option<(ItemPath, &EmbeddedFile)> {
        resolve_link(&self.files, target)
    }
}

/// A `Note` in a `Vault`.
pub(crate) struct NoteItem {
    pub note: Note,
    index: NodeIndex,
}

#[derive(Hash, Eq, PartialEq, PartialOrd, Ord, Debug, Clone)]
pub(crate) enum ItemPath {
    Absolute(Vec<String>),
    FileName(String),
}

impl ItemPath {
    pub(crate) fn from_path<P: AsRef<Path>>(path: P) -> Self {
        let path: &Path = path.as_ref();
        let parts = path
            .components()
            .map(|component| component.as_os_str().to_string_lossy().to_string())
            .collect::<Vec<String>>();
        ItemPath::Absolute(parts)
    }

    pub(crate) fn from_path_without_ext<P: AsRef<Path>>(path: P) -> Self {
        let path: &Path = path.as_ref();
        let mut parts = path
            .parent()
            .expect("note path parent")
            .components()
            .map(|component| component.as_os_str().to_string_lossy().to_string())
            .collect::<Vec<String>>();

        let title = path.file_stem().unwrap().to_string_lossy().to_string();
        parts.push(title);

        ItemPath::Absolute(parts)
    }
}

impl From<ItemPath> for PathBuf {
    fn from(val: ItemPath) -> Self {
        match val {
            ItemPath::Absolute(components) => PathBuf::from(&components.join("/")),
            ItemPath::FileName(filename) => PathBuf::from(filename.clone()),
        }
    }
}

impl Serialize for ItemPath {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            ItemPath::Absolute(components) => serializer.serialize_str(&components.join("/")),
            ItemPath::FileName(filename) => serializer.serialize_str(filename),
        }
    }
}

impl Display for ItemPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ItemPath::Absolute(components) => f.write_str(&components.join("/")),
            ItemPath::FileName(filename) => f.write_str(filename),
        }
    }
}

impl From<String> for ItemPath {
    fn from(value: String) -> Self {
        if value.contains('/') {
            ItemPath::Absolute(value.split('/').map(|v| v.to_string()).collect())
        } else {
            ItemPath::FileName(value)
        }
    }
}

pub(crate) fn resolve_link<S: Into<String>, V>(
    paths: &HashMap<ItemPath, V>,
    target: S,
) -> Option<(ItemPath, &V)> {
    let target = ItemPath::from(target.into());
    match target {
        ItemPath::Absolute(_) => paths.get(&target).map(|value| (target, value)),
        ItemPath::FileName(filename) => {
            for (path, value) in paths.iter() {
                if let ItemPath::Absolute(components) = path {
                    if let Some(item_filename) = components.last() {
                        if *item_filename == filename {
                            return Some((path.clone(), value));
                        }
                    }
                }
            }
            None
        }
    }
}

pub(crate) enum EmbeddedFile {
    Image(PathBuf),
    Audio(PathBuf),
    Video(PathBuf),
    Pdf(PathBuf),
}
