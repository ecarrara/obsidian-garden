pub mod metadata;
pub mod note;
mod site;
pub mod vault;
pub mod wikilink;

use clap::{command, Parser};
use site::Site;
use std::path::PathBuf;
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

    let mut source_static_dir = PathBuf::from(&args.template);
    source_static_dir.push("_static");
    let mut target_static_dir = PathBuf::from(&args.output_directory);
    target_static_dir.push("_static");

    if let Err(err) = fsync::sync(source_static_dir, target_static_dir) {
        eprintln!("failed to copy _static directory: {err}")
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
