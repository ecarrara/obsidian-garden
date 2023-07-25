pub mod metadata;
pub mod note;
mod site;
pub mod vault;
pub mod wikilink;

use site::Site;
use vault::VaultBuilder;

fn main() {
    let vault = VaultBuilder::new("./notes").build();
    let site = Site::new(&vault, "./templates/default", "./_build");

    for (path, item) in &vault.notes {
        println!(
            "{}\n   Title: {}\n   Links: {:?}",
            path, &item.note.title, &item.note.links
        );

        site.render_note(path).unwrap();
    }

    vault.dot_graph();
}
