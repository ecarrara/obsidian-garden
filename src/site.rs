use std::{
    collections::HashMap,
    fs::File,
    path::{Path, PathBuf},
};

use minijinja::{context, path_loader, Environment};
use serde::Serialize;
use thiserror::Error;

use crate::vault::{EmbeddedFile, ItemPath, Vault};

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

    fn render_note_string(&self, path: &ItemPath) -> Result<String, SiteRenderError> {
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
            if wikilink.embedded {
                let (target, fragment) = wikilink
                    .target
                    .split_once('#')
                    .unwrap_or((&wikilink.target, ""));

                if let Some((item_path, embedded_file)) = self.vault.resolve_embedded_link(target) {
                    println!("resolved file: {}", item_path);

                    let embedded_html = embedded_file_html(embedded_file, &item_path, fragment);
                    html = html.replace(&format!("{wikilink}"), &embedded_html);

                    let path: PathBuf = item_path.into();
                    let mut source = self.vault.root.clone();
                    source.push(&path);

                    let mut target = self.output_directory.clone();
                    target.push(&path);

                    if let Some(parent) = target.parent() {
                        if !parent.exists() {
                            std::fs::create_dir_all(parent)?;
                        }
                    }

                    println!("copying {} -> {}", source.display(), target.display());
                    std::fs::copy(source, target)?;
                }
            } else if let Some(note_path) = self.vault.resolve_link(&wikilink.target) {
                let label = wikilink.label.as_ref().unwrap_or(&wikilink.target);
                let href = format!("/{}.html", &note_path);
                let a_tag =
                    format!("<a href=\"{href}\" title=\"{label}\" class=\"wikilink\">{label}</a>",);
                html = html.replace(&format!("{wikilink}"), &a_tag);
            }
        }

        Ok(html)
    }

    pub fn render_note(&self, path: &ItemPath) -> Result<(), SiteRenderError> {
        let html = self.render_note_string(path)?;

        let filename = format!("{}.html", path);
        let output_path = self.output_directory.join(filename);

        std::fs::create_dir_all(output_path.parent().unwrap())?;
        std::fs::write(output_path, html)?;

        Ok(())
    }

    fn build_menu(vault: &Vault) -> Menu {
        let mut paths: Vec<ItemPath> = vault.notes.keys().cloned().collect();
        paths.sort();

        let mut menu: Menu = Menu::new();

        for current_path in paths {
            menu.add_path(&current_path);
        }

        menu
    }
}

#[derive(Serialize, Debug)]
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

    fn add_path(&mut self, path: &ItemPath) {
        if let ItemPath::Absolute(components) = path {
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

#[derive(Serialize, Debug)]
#[serde(untagged)]
enum MenuItem {
    Page(ItemPath),
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

fn embedded_file_html(file: &EmbeddedFile, path: &ItemPath, fragment: &str) -> String {
    match file {
        EmbeddedFile::Image(_) => format!(r#"<img src="{}">"#, path),
        EmbeddedFile::Audio(_) => format!(r#"<audio src="{}" controls></audio>"#, path),
        EmbeddedFile::Video(_) => format!(r#"<video src="{}" controls></video>"#, path),
        EmbeddedFile::Pdf(_) => {
            format!(
                r#"<iframe src="{}#{}" frameborder="0"></iframe>"#,
                path, fragment
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::embedded_file_html;
    use crate::vault::{EmbeddedFile, ItemPath};

    #[test]
    fn embedded_file_image_html() {
        let file = EmbeddedFile::Image("./files/image.webp".into());
        let html = embedded_file_html(&file, &ItemPath::from_path("./files/image.webp"), "");
        assert_eq!(html, r#"<img src="./files/image.webp">"#);
    }

    #[test]
    fn embedded_file_audio_html() {
        let file = EmbeddedFile::Audio("./files/audio.ogg".into());
        let html = embedded_file_html(&file, &ItemPath::from_path("./files/audio.ogg"), "");
        assert_eq!(html, r#"<audio src="./files/audio.ogg" controls></audio>"#);
    }

    #[test]
    fn embedded_file_video_html() {
        let file = EmbeddedFile::Video("./files/video.ogv".into());
        let html = embedded_file_html(&file, &ItemPath::from_path("./files/video.ogv"), "");
        assert_eq!(html, r#"<video src="./files/video.ogv" controls></video>"#);
    }

    #[test]
    fn embedded_file_pdf_html() {
        let file = EmbeddedFile::Pdf("./files/document.pdf".into());
        let html = embedded_file_html(
            &file,
            &ItemPath::from_path("./files/document.pdf"),
            "page=1",
        );
        assert_eq!(
            html,
            r#"<iframe src="./files/document.pdf#page=1" frameborder="0"></iframe>"#
        );
    }
}
