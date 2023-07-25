use petgraph::{
    dot::Dot,
    prelude::{NodeIndex, StableGraph},
};
use serde::Serialize;
use std::{
    collections::HashMap,
    fmt::Display,
    os::unix::prelude::OsStrExt,
    path::{Path, PathBuf},
};
use walkdir::WalkDir;

use crate::note::Note;

pub(crate) struct VaultBuilder {
    pub directory: PathBuf,
}

impl VaultBuilder {
    pub fn new<P: AsRef<Path>>(directory: P) -> Self {
        Self {
            directory: directory.as_ref().to_path_buf(),
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
                let note_path: NotePath = link.target.clone().into();

                if let Some(found_item) = notes.get(&note_path) {
                    graph.add_edge(item.index, found_item.index, ());
                }
            }
        }

        Vault { notes, graph }
    }
}

pub(crate) struct Vault {
    pub notes: HashMap<NotePath, NoteItem>,
    graph: StableGraph<NotePath, ()>,
}

impl Vault {
    pub(crate) fn dot_graph(&self) {
        let dot = Dot::new(&self.graph);
        println!("{:?}", dot);
    }

    pub(crate) fn get_note(&self, note_path: &NotePath) -> Option<&Note> {
        self.notes.get(note_path).map(|item| &item.note)
    }
}

/// A `Note` in a `Vault`.
pub(crate) struct NoteItem {
    pub note: Note,
    index: NodeIndex,
}

#[derive(Hash, Eq, PartialEq, PartialOrd, Ord, Debug, Clone)]
pub(crate) struct NotePath {
    pub path: Vec<String>,
}

impl Serialize for NotePath {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.path.join("/"))
    }
}

impl Display for NotePath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.path.join("/"))
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

        NotePath { path }
    }
}

impl From<String> for NotePath {
    fn from(value: String) -> Self {
        Self {
            path: value.split('/').map(|v| v.to_string()).collect(),
        }
    }
}
