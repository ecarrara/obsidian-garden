use std::path::{Path, PathBuf};

use minijinja::{context, path_loader, Environment};
use thiserror::Error;

use crate::vault::{NotePath, Vault};

pub(crate) struct Site<'a> {
    vault: &'a Vault,
    env: Environment<'a>,
    output_directory: PathBuf,
    menu: Vec<(String, String)>,
}

impl<'a> Site<'a> {
    pub fn new<P: AsRef<Path>>(vault: &'a Vault, template_dir: P, output_directory: P) -> Self {
        let mut env = Environment::new();
        env.set_loader(path_loader(template_dir));

        let mut menu: Vec<(String, String)> = vault
            .notes
            .keys()
            .map(|path| (format!("/{}.html", path), format!("{}", path)))
            .collect();
        menu.sort();

        Self {
            vault,
            env,
            output_directory: output_directory.as_ref().to_path_buf(),
            menu,
        }
    }

    fn render_note_string(&self, path: &NotePath) -> Result<String, SiteRenderError> {
        let note = self
            .vault
            .get_note(path)
            .ok_or(SiteRenderError::NoteNotFound)?;

        let page_tmpl = self.env.get_template("page.html")?;

        let mut html = page_tmpl
            .render(
                context! { note => note, path => path, note_html => note.render_html(), menu => self.menu }
            )
            .unwrap();

        for wikilink in note.links.iter() {
            let label = wikilink.label.as_ref().unwrap_or(&wikilink.target);
            let href = format!("/{}.html", &wikilink.target);
            let a_tag =
                format!("<a href=\"{href}\" title=\"{label}\" class=\"wikilink\">{label}</a>",);
            html = html.replace(&format!("{wikilink}"), &a_tag);
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
