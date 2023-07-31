use std::{
    collections::HashMap,
    fs::File,
    path::{Path, PathBuf},
};

use minijinja::{context, path_loader, Environment};
use serde::Serialize;
use thiserror::Error;

use crate::vault::{NotePath, Vault};

pub(crate) struct Site<'a> {
    vault: &'a Vault,
    env: Environment<'a>,
    output_directory: PathBuf,
    menu: Menu,
    context: Option<serde_yaml::Value>,
}

impl<'a> Site<'a> {
    pub fn new<P: AsRef<Path>>(
        vault: &'a Vault,
        template_dir: P,
        output_directory: P,
        context_filepath: P,
    ) -> Result<Self, SiteError> {
        let mut env = Environment::new();
        env.set_loader(path_loader(template_dir));

        let context = {
            if let Ok(file) = File::open(&context_filepath) {
                Some(serde_yaml::from_reader(file)?)
            } else {
                eprintln!("failed to open {}", &context_filepath.as_ref().display());
                None
            }
        };

        let menu = Site::build_menu(vault);

        Ok(Self {
            vault,
            env,
            output_directory: output_directory.as_ref().to_path_buf(),
            context,
            menu,
        })
    }

    fn render_note_string(&self, path: &NotePath) -> Result<String, SiteRenderError> {
        let note = self
            .vault
            .get_note(path)
            .ok_or(SiteRenderError::NoteNotFound)?;

        let page_tmpl = self.env.get_template("page.html")?;

        let mut html = page_tmpl
            .render(context! {
                note => note,
                path => path,
                note_html => note.render_html(),
                menu => self.menu,
                graph => self.vault.local_graph(path, 2),
                site => self.context,
            })
            .unwrap();

        for wikilink in note.links.iter() {
            if let Some(note_path) = self.vault.resolve_link(&wikilink.target) {
                let label = wikilink.label.as_ref().unwrap_or(&wikilink.target);
                let href = format!("/{}.html", &note_path);
                let a_tag =
                    format!("<a href=\"{href}\" title=\"{label}\" class=\"wikilink\">{label}</a>",);
                html = html.replace(&format!("{wikilink}"), &a_tag);
            }
        }

        Ok(html)
    }

    pub fn render_note(&self, path: &NotePath) -> Result<(), SiteRenderError> {
        let html = self.render_note_string(path)?;

        let filename = format!("{}.html", path);
        let output_path = self.output_directory.join(filename);

        std::fs::create_dir_all(output_path.parent().unwrap())?;
        std::fs::write(output_path, html)?;

        Ok(())
    }

    fn build_menu(vault: &Vault) -> Menu {
        let mut paths: Vec<NotePath> = vault.notes.keys().cloned().collect();
        paths.sort();

        let mut menu: Menu = Menu::new();

        for current_path in paths {
            menu.add_path(&current_path);
        }

        menu
    }
}

#[derive(Serialize)]
struct Menu {
    #[serde(flatten)]
    items: HashMap<String, MenuItem>,
}

impl Menu {
    fn new() -> Self {
        Self {
            items: HashMap::new(),
        }
    }

    fn add_path(&mut self, path: &NotePath) {
        if let NotePath::Absolute(components) = path {
            let mut current_menu = self;
            for component in &components[..components.len() - 1] {
                let menu = current_menu
                    .items
                    .entry(component.clone())
                    .or_insert(MenuItem::Folder(Menu::new()));
                match menu {
                    MenuItem::Page(_) => todo!(),
                    MenuItem::Folder(menu) => current_menu = menu,
                }
            }

            let filename = &components[components.len() - 1];
            current_menu
                .items
                .entry(filename.clone())
                .or_insert(MenuItem::Page(path.clone()));
        }
    }
}

#[derive(Serialize)]
#[serde(untagged)]
enum MenuItem {
    Page(NotePath),
    Folder(Menu),
}

#[derive(Error, Debug)]
pub(crate) enum SiteRenderError {
    #[error("note not found")]
    NoteNotFound,

    #[error("template error")]
    TemplateError(#[from] minijinja::Error),

    #[error("io error")]
    IOError(#[from] std::io::Error),
}

#[derive(Error, Debug)]
pub(crate) enum SiteError {
    #[error("context error")]
    InvalidContext(#[from] serde_yaml::Error),
}
