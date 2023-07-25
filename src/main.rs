pub mod metadata;
pub mod note;
pub mod vault;
pub mod wikilink;

use vault::VaultBuilder;

fn main() {
    let vault = VaultBuilder::new("./notes").build();

    for (path, item) in &vault.notes {
        println!(
            "{}\n   Title: {}\n   Links: {:?}",
            path, &item.note.title, &item.note.links
        );
    }

    vault.dot_graph();
}
