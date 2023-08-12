# Obsidian Garden

Obsidian Garden is a program to transform Obsidian Vault's notes into web
pages. It converts your markdown notes, created in Obsidian, into fully
functional site, ready for deployment.

## Installation

If you are on OS X or Linux, you can use the installation script to fetch the
latest release:

```bash
curl https://raw.githubusercontent.com/ecarrara/obsidian-garden/main/install.toml | sh
```

## Getting Started

1. Navigate to you Vault folder and run `obsidian-garden init`

```bash
cd my-notes/
obsidian-garden init
```

2. Customize your site settings by editing the `.garden/site.yaml` file

```yaml
title: Site name
pagefind: false
topnav:
  links:
    - text: Link 1
      href: https://example.com/link-1
    - text: Link 2
      href: https://example.com/link-2
```

3. Generate a static site from your notes.

```bash
obsidian-garden build
```

4. Optional - Enable pagefind on `.garden/site.yaml` and run
[pagefind](https://pagefind.app) to index your site

```bash
pagefind --source dist
```