pub mod metadata;
pub mod note;
mod site;
pub mod vault;
pub mod wikilink;

use clap::{command, Parser, Subcommand};
use rust_embed::RustEmbed;
use site::Site;
use std::path::{Path, PathBuf};
use vault::VaultBuilder;

fn main() {
    let args = Args::parse();

    match args.command {
        Commands::Init { vault } => {
            let mut config_dir = PathBuf::from(&vault);
            config_dir.push(".garden");
            if config_dir.exists() {
                eprintln!("{} already exists.", config_dir.display());
                std::process::exit(-1);
            }

            if let Err(err) = initialize_config(&config_dir) {
                eprintln!(
                    "failed to create {}/site.yaml: {}",
                    config_dir.display(),
                    err
                );
                std::process::exit(-1);
            }

            if let Err(err) = initialize_default_template(&config_dir) {
                eprintln!("failed to copy default template to filesystem: {err}");
                std::process::exit(-1);
            }

            println!(
                "Obsidian Garden initialized. 🌱🌺\n\
                Run `obsidian-garden build` to generate a static site from your notes."
            )
        }
        Commands::Build {
            vault,
            output_directory,
            template,
            tag,
            config: context,
        } => {
            let mut vault_builder = VaultBuilder::new(&vault);
            if let Some(tags) = tag {
                vault_builder.filter_tags(tags);
            }

            let vault = vault_builder.build();
            match Site::new(&vault, &template, &output_directory, &context) {
                Ok(site) => {
                    println!("Generating pages...");
                    for path in vault.notes.keys() {
                        println!("  {}", path);
                        site.render_note(path).unwrap();
                    }

                    let mut source_static_dir = PathBuf::from(&template);
                    source_static_dir.push("_static");
                    let mut target_static_dir = PathBuf::from(&output_directory);
                    target_static_dir.push("_static");

                    if let Err(err) = fsync::sync(source_static_dir, target_static_dir) {
                        eprintln!("failed to copy _static directory: {err:?}")
                    }

                    println!("\nOutput directory: {}", &output_directory);
                }
                Err(err) => eprintln!("build failed: {err:?}"),
            }
        }
    }
}

fn initialize_config<P: AsRef<Path>>(config_dir: P) -> Result<(), std::io::Error> {
    let default_config = r#"---
title: Site name
topnav:
  links:
    - text: Link 1
      href: https://example.com/link-1
    - text: Link 2
      href: https://example.com/link-2
"#;

    std::fs::create_dir_all(config_dir.as_ref())?;

    let mut config_filepath = PathBuf::from(config_dir.as_ref());
    config_filepath.push("site.yaml");
    std::fs::write(config_filepath, default_config)?;

    Ok(())
}

fn initialize_default_template<P: AsRef<Path>>(config_dir: P) -> Result<(), std::io::Error> {
    let mut default_template_dir = PathBuf::from(config_dir.as_ref());
    default_template_dir.push("templates");
    default_template_dir.push("default");

    std::fs::create_dir_all(&default_template_dir)?;

    for filename in DefaultTemplateAsset::iter() {
        let mut filepath = PathBuf::from(&default_template_dir);
        filepath.push(filename.as_ref());
        if let Some(parent_dir) = filepath.parent() {
            std::fs::create_dir_all(parent_dir)?;
        }

        let file = DefaultTemplateAsset::get(&filename).unwrap();
        std::fs::write(&filepath, file.data)?;
    }

    Ok(())
}

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize Vault directory.
    Init {
        /// Vault directory.
        #[arg(default_value = ".")]
        vault: String,
    },

    /// Build static site.
    Build {
        /// Vault directory.
        #[arg(default_value = ".")]
        vault: String,

        /// Output directory.
        #[arg(default_value = "./dist")]
        output_directory: String,

        /// Template directory.
        #[arg(long, default_value = "templates/default")]
        template: String,

        /// Only select notes with this tag (can be used multiple times).
        #[arg(short, long)]
        tag: Option<Vec<String>>,

        #[arg(long, default_value = ".garden/site.yaml")]
        config: String,
    },
}

#[derive(RustEmbed)]
#[folder = "templates/default"]
struct DefaultTemplateAsset;
