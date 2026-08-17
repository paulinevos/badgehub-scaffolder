# badgehub

A command line tool for scaffolding and managing BadgeHub apps.

[BadgeHub](https://badgehub.eu) is the app store for the electronic badges
handed out at events such as Fri3d Camp and MCH. An app published there is a
directory with a `metadata.json` describing it, a `MANIFEST.JSON` the badge
firmware reads, and the code itself. This tool writes that layout for you and
keeps it consistent afterwards, so you can get on with the app.

The binary is called `bh`.

## Install

```
brew install paulinevos/tap/badgehub
```

Or, with a Rust toolchain to hand:

```
cargo install --git https://github.com/paulinevos/badgehub-scaffolder
```

## Usage

### `bh new`

Scaffolds a project into a new directory beside the current one: a
`metadata.json`, a `MANIFEST.JSON`, a README, a licence stub, and a fresh git
repository. Anything you do not pass as a flag is asked for interactively; if
there is no terminal to ask on, a missing answer is an error rather than a
guess.

| Flag | Meaning |
| --- | --- |
| `--slug` | Project slug, e.g. `org.fri3d.hwtest` |
| `--name` | Display name shown in the launcher and on BadgeHub |
| `--description` | One-line description |
| `--author` | Author name |
| `--app-version` | Semantic version, defaults to `0.1.0` |
| `--project-type` | One of `app`, `library`, `firmware`, `other` |
| `--category` | Category; repeat for several |
| `--badge` | Badge slug; repeat for several |
| `--license` | Licence identifier, e.g. `MIT` |
| `--release-action`, `--no-release-action` | Write the release workflow, or skip it |

The repository is settled by a second group of flags. Passing `--git-url`
records a repository that already exists; passing `--create-repo` creates one on
GitHub through the API and sets it as the `origin` remote. The two conflict.
With neither, and a terminal available, you are asked which you want.

| Flag | Meaning |
| --- | --- |
| `--git-url` | URL of a repository that already exists |
| `--create-repo` | Create the repository on GitHub instead |
| `--repo-name` | Name for the created repository; defaults to the slug |
| `--visibility` | `public` or `private`, defaults to `public` |

Creating a repository needs a GitHub token with `Administration: read and
write`. If none is configured, `bh` explains how to make one, asks for it, and
offers to save it.

A third pair settles the release workflow: `--release-action` writes it,
`--no-release-action` skips it, and with neither you are asked. Without a
terminal, nothing is written.

### `bh set`

Changes what a scaffolded project can still change, keeping `metadata.json` and
`MANIFEST.JSON` in step — the same answer is `author` in one and `publisher` in
the other. Anything else in those files, including fields this tool knows
nothing about, is left exactly where it was.

```
bh set --name "Hardware Test" --author "Pauline Vos"
```

Run it from the project root or from inside the app directory; `--app-directory`
names one explicitly. Bare `bh set` walks the fields with the current values
prefilled. The flags are `--name`, `--author`, `--description` and `--license`.

The slug is not among them: it is the directory name, the manifest's `fullname`,
the BadgeHub project identity and the release workflow's `app-directory` all at
once. Nor is the version, which the release action writes from the git tag.

Changing the licence rewrites `LICENSE` only while it is still the placeholder
`bh new` wrote. Once you have put real licence text there, it is left alone and
`bh` says so.

### `bh release-action`

Writes `.github/workflows/release.yml` into a project that already exists, so
publishing a GitHub release builds the `.mpk` and pushes it to BadgeHub through
[badgehub-release-action](https://github.com/paulinevos/badgehub-release-action).
An existing workflow is never replaced without `--force`. Afterwards it prints
the `gh secret set BADGEHUB_API_TOKEN` line still to run; without that secret the
workflow still builds on every release and warns instead of publishing.

### `bh config`

Edits the per-user defaults every new project starts from — author, licence and
GitHub token — kept in `~/.config/badgehub/config.json`. The file is written
readable only by you, since it may hold a token.

## Environment

| Variable | Effect |
| --- | --- |
| `GITHUB_TOKEN`, `GH_TOKEN` | Token used to create repositories. Either one wins over the saved config, so a shell that already carries a token needs no setup. |
| `XDG_CONFIG_HOME` | Where the config file lives, in place of `~/.config`. |
| `GITHUB_API_URL` | GitHub API base, for GitHub Enterprise or for tests. |
| `BADGEHUB_API_URL` | BadgeHub API base. |

## Development

```
cargo test
```

## Licence

MIT. See [LICENSE](LICENSE).
