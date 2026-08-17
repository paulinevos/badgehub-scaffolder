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

Sets fields in `metadata.json` and `MANIFEST.JSON` to new values, under whichever
name each file uses — an author is `author` in one and `publisher` in the other.
Anything else in those files is left where it is, including fields this tool
knows nothing about.

| Flag | Meaning                                            |
| --- |----------------------------------------------------|
| `--name` | Display name shown in the launcher and on BadgeHub |
| `--author` | Author name                                        |
| `--description` | One-line description                               |

Run it from the project root or from inside the app directory. Passing no flags
walks the fields with their current values prefilled.

Nothing else is editable here. The slug is the directory name, the manifest's
`fullname`, the BadgeHub project identity and the release workflow's
`app-directory` all at once; the version is written by the release action from
the git tag; and the licence is a file to edit as much as a field to set, so
`license_type` and `LICENSE` are yours to change together.

### Setting up GitHub release workflow on an existing project

```shell
bh release-action
```

Writes `.github/workflows/release.yml` into a project that already exists, so
publishing a GitHub release builds the `.mpk` and pushes it to BadgeHub through
[badgehub-release-action](https://github.com/paulinevos/badgehub-release-action).
An existing workflow is never replaced without `--force`, and `--app-directory`
names a project other than the one you are standing in. Afterwards it prints the
`gh secret set BADGEHUB_API_TOKEN` line still to run; without that secret the
workflow still builds on every release and warns instead of publishing.

### Configuring user defaults

```shell
bh config
```

Edits the per-user defaults every new project starts from — author, licence and
GitHub token — kept in `~/.config/badgehub/config.json`. The file is written
readable only by you, since it may hold a token.

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
