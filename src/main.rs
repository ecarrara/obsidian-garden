pub mod metadata;
pub mod note;
mod site;
pub mod vault;
pub mod wikilink;

use clap::{command, Parser};
use site::Site;
use vault::VaultBuilder;

fn main() {
    let args = Args::parse();

    let mut vault_builder = VaultBuilder::new(&args.vault);
    if let Some(tags) = args.tags {
        vault_builder.filter_tags(tags);
    }

    let vault = vault_builder.build();
    let site = Site::new(&vault, &args.template, &args.output_directory);

    for path in vault.notes.keys() {
        println!("{}", path);
        site.render_note(path).unwrap();
    }
}

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Vault directory.
    vault: String,

    /// Output directory.
    output_directory: String,

    /// Template directory.
    #[arg(long, default_value = "templates/default")]
    template: String,

    #[arg(short, long)]
    tags: Option<Vec<String>>,
}
