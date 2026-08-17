# BadgeHub Scaffolder

A command line tool for scaffolding BadgeHub apps.

[BadgeHub](https://badgehub.eu) is the app store for electronic badges
handed out at hacker camps such as [Fri3d Camp](https://fri3d.be/en/) and [WHY](https://why2025.org/). 
This tool helps you initialize your app with the necessary files to publish it to BadgeHub.

Comes with a GitHub release workflow that bundles your app and pushes it to BadgeHub automatically when you 
tag a release.

## Installation

```
brew install paulinevos/tap/badgehub
```

Or from source using Cargo:

```
cargo install --git https://github.com/paulinevos/badgehub-scaffolder
```

## Usage

### Scaffolding a new project

```shell
bh new              # Scaffold a new project in the current directory OR
bh new <directory>  # Scaffold a new project in the specified directory
```

The scaffolder will prompt you for the necessary information to create the project. It will create the following structure:

```
.
├── .github/workflows/release.yml   optional, see below
├── .gitignore
├── LICENSE                         a stub naming the licence you chose
├── README.md
└── nl.paulinevos.some-app/         the app directory, named for the slug
    ├── metadata.json               the BadgeHub store listing
    ├── MANIFEST.JSON               the MicroPythonOS launcher manifest
    ├── icon-32x32.png              placeholder icon, generated from the slug
    ├── icon-64x64.png              placeholder icon, generated from the slug
    └── __init__.py
```

| Argument | Meaning |
| --- | --- |
| `[DIR]` | Where to scaffold; created if missing, must be empty. Defaults to the current directory |

The (optional) flags below may be passed in lieu of answering the prompts.

| Flag | Meaning                                            |
| --- |----------------------------------------------------|
| `--slug` | Project slug, e.g. `nl.paulinevos.some-app`               |
| `--name` | Display name shown in the launcher and on BadgeHub |
| `--description` | One-line description                               |
| `--author` | Author name                                        |
| `--app-version` | Semantic version, defaults to `0.1.0`              |
| `--project-type` | One of `app`, `library`, `firmware`, `other`       |
| `--category` | Category; repeat for several                       |
| `--badge` | Badge slug; repeat for several                     |
| `--license` | Licence identifier, e.g. `MIT`                     |
| `--release-action`, `--no-release-action` | Write the release workflow, or skip it             |
| `--git-url` | URL of a repository that already exists |
| `--create-repo` | Create the repository on GitHub instead |
| `--repo-name` | Name for the created repository; defaults to the slug |
| `--visibility` | `public` or `private`, defaults to `public` |

Creating a repository needs a configured GitHub token with `Administration: read and
write`.

### Editing existing project metadata

```
bh set --name "ROM Installer" --author "Pauline Vos"
```

Sets fields in `metadata.json` and `MANIFEST.JSON`. Run it from the project root
or from inside the app directory. Passing no flags walks the fields with their
current values prefilled.

| Flag | Meaning                                            |
| --- |----------------------------------------------------|
| `--name` | Display name shown in the launcher and on BadgeHub |
| `--author` | Author name                                        |
| `--description` | One-line description                               |

The slug, the version and the licence are not editable here.

### Bundling the app

```shell
bh bundle
```

Packs the app into `<fullname>_<version>.mpk`, the archive MicroPythonOS
installs. The version comes from `MANIFEST.JSON`. `*.mpk` is added to
`.gitignore` if it is not there yet.

| Flag | Meaning |
| --- | --- |
| `--app-directory` | The project to bundle; defaults to looking in the current directory |
| `--output-directory` | Where to write the `.mpk`; defaults to the project root, created if missing |
| `--no-gitignore` | Leave `.gitignore` alone; for CI |

### Setting up GitHub release workflow on an existing project

```shell
bh release-action
```

Writes `.github/workflows/release.yml`, so publishing a GitHub release builds the
`.mpk` and pushes it to BadgeHub through
[badgehub-release-action](https://github.com/paulinevos/badgehub-release-action).
Publishing needs a `BADGEHUB_API_TOKEN` secret on the repository.

| Flag | Meaning |
| --- | --- |
| `--app-directory` | The project to add the workflow to; defaults to the current directory |
| `--force` | Replace an existing workflow |

### Configuring user defaults

```shell
bh config
```

Sets the author, licence and GitHub token that new projects start from, kept in
`~/.config/badgehub/config.json`.

## Environment variables

| Variable | Effect |
| --- | --- |
| `GITHUB_TOKEN`, `GH_TOKEN` | Token used to create repositories. Either one wins over the saved config, so a shell that already carries a token needs no setup. |
| `XDG_CONFIG_HOME` | Where the config file lives, in place of `~/.config`. |
| `GITHUB_API_URL` | GitHub API base, for GitHub Enterprise or for tests. |
| `BADGEHUB_API_URL` | BadgeHub API base. |

## Development

To run tests:
```
cargo test
```

## Licence

MIT. See [LICENSE](LICENSE).
