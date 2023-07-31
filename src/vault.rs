use petgraph::prelude::{NodeIndex, StableGraph};
use serde::Serialize;
use std::{
    collections::{HashMap, HashSet, VecDeque},
    fmt::Display,
    os::unix::prelude::OsStrExt,
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

    pub fn build(&self) -> Vault {
        let mut notes: HashMap<NotePath, NoteItem> = HashMap::new();
        let mut graph = StableGraph::new();

        for result in WalkDir::new(&self.directory) {
            match result {
                Ok(entry) => {
                    if !entry.file_type().is_file() {
                        continue;
                    }

                    if !entry.file_name().as_bytes().ends_with(b".md") {
                        continue;
                    }

                    match Note::from_file(&entry.path()) {
                        Ok(note) => {
                            if let Some(tags) = &self.tags {
                                if !note.tags.iter().any(|t| tags.contains(t)) {
                                    continue;
                                }
                            }

                            let note_path =
                                NotePath::from(entry.path().strip_prefix(&self.directory).unwrap());
                            let index = graph.add_node(note_path.clone());
                            notes.insert(note_path, NoteItem { index, note });
                        }
                        Err(err) => {
                            eprintln!("Unable to parse {}: {}", entry.path().display(), err)
                        }
                    }
                }
                Err(err) => eprintln!("{}", err),
            }
        }

        for item in notes.values() {
            for link in item.note.links.iter() {
                if let Some(found) = resolve_link(&notes, &link.target) {
                    graph.add_edge(item.index, notes[&found].index, ());
                }
            }
        }

        Vault { notes, graph }
    }

    pub(crate) fn filter_tags(&mut self, tags: Vec<String>) -> &mut Self {
        self.tags = Some(tags);
        self
    }
}

pub(crate) struct Vault {
    pub notes: HashMap<NotePath, NoteItem>,
    graph: StableGraph<NotePath, ()>,
}

impl Vault {
    pub(crate) fn get_note(&self, note_path: &NotePath) -> Option<&Note> {
        self.notes.get(note_path).map(|item| &item.note)
    }

    pub(crate) fn local_graph(
        &self,
        path: &NotePath,
        max_depth: usize,
    ) -> Option<StableGraph<NotePath, ()>> {
        let mut depth = 0;
        let mut stack = VecDeque::new();
        let mut discovered: HashSet<NotePath> = HashSet::new();
        let mut path_indexes: HashMap<NotePath, NodeIndex> = HashMap::new();

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

    pub(crate) fn resolve_link<S: Into<String>>(&self, target: S) -> Option<NotePath> {
        resolve_link(&self.notes, target)
    }
}

/// A `Note` in a `Vault`.
pub(crate) struct NoteItem {
    pub note: Note,
    index: NodeIndex,
}

#[derive(Hash, Eq, PartialEq, PartialOrd, Ord, Debug, Clone)]
pub(crate) enum NotePath {
    Absolute(Vec<String>),
    FileName(String),
}

impl Serialize for NotePath {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            NotePath::Absolute(components) => serializer.serialize_str(&components.join("/")),
            NotePath::FileName(filename) => serializer.serialize_str(filename),
        }
    }
}

impl Display for NotePath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NotePath::Absolute(components) => f.write_str(&components.join("/")),
            NotePath::FileName(filename) => f.write_str(filename),
        }
    }
}

impl From<&Path> for NotePath {
    fn from(value: &Path) -> Self {
        let mut path = value
            .parent()
            .expect("note path parent")
            .components()
            .map(|component| component.as_os_str().to_string_lossy().to_string())
            .collect::<Vec<String>>();

        let title = value.file_stem().unwrap().to_string_lossy().to_string();
        path.push(title);

        NotePath::Absolute(path)
    }
}

impl From<String> for NotePath {
    fn from(value: String) -> Self {
        if value.contains('/') {
            NotePath::Absolute(value.split('/').map(|v| v.to_string()).collect())
        } else {
            NotePath::FileName(value)
        }
    }
}

pub(crate) fn resolve_link<S: Into<String>>(
    notes: &HashMap<NotePath, NoteItem>,
    target: S,
) -> Option<NotePath> {
    let target = NotePath::from(target.into());
    match target {
        NotePath::Absolute(_) => {
            if notes.contains_key(&target) {
                Some(target)
            } else {
                None
            }
        }
        NotePath::FileName(filename) => {
            for path in notes.keys() {
                if let NotePath::Absolute(components) = path {
                    if let Some(item_filename) = components.last() {
                        if *item_filename == filename {
                            return Some(path.clone());
                        }
                    }
                }
            }
            None
        }
    }
}
