# Homebrew tap

`brew install paulinevos/tap/badgehub` resolves to a repository named
`paulinevos/homebrew-tap`. That repository does not exist yet; this directory
holds what goes into it.

## Creating the tap

Make a public GitHub repository called `homebrew-tap` under `paulinevos`. The
`homebrew-` prefix is what lets `paulinevos/tap` be written as shorthand, so
the name is not free choice.

Then copy:

- `badgehub.rb` to `Formula/badgehub.rb` in the tap
- `tap-ci.yml` to `.github/workflows/ci.yml` in the tap

The formula lands with placeholder checksums. The first real one comes from the
release workflow, which opens a pull request against the tap on every pushed
`v*` tag.

## The TAP_TOKEN secret

The release workflow writes to a second repository, which `GITHUB_TOKEN` cannot
reach, so it needs a token of its own. Pauline has to create this herself — a
token is a credential and nobody else should hold it, least of all an agent.

Make a fine-grained personal access token at
<https://github.com/settings/personal-access-tokens/new>:

- Repository access: only `paulinevos/homebrew-tap`
- Permissions: Contents — read and write; Pull requests — read and write

Nothing else. Then add it to `paulinevos/badgehub-scaffolder` as a repository
secret named `TAP_TOKEN`.

For the tap to accept the pull requests the workflow opens, Settings → Actions →
General → "Allow GitHub Actions to create and approve pull requests" has to be
on in the tap repository.

## Testing the formula locally

From a clone of the tap, with real checksums in place:

```
brew install --build-from-source ./Formula/badgehub.rb
brew test badgehub
brew audit --strict --new paulinevos/tap/badgehub
```

`--new` applies the extra rules Homebrew holds new formulae to; drop it once the
formula has shipped once. A formula with placeholder checksums fails at the
download step, which is the intended behaviour rather than a problem to work
around.
