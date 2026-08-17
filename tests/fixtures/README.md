# Recorded GitHub responses

Real response bodies, not hand-written ones, so the parsing here is checked
against shapes GitHub actually sends.

Both come from [octokit/fixtures](https://github.com/octokit/fixtures), which
records against api.github.com and is maintained by GitHub's own Octokit org.
There is no official hosted GitHub mock, and the fixtures ship as JSON, so they
are copied in rather than run through their Node mock server.

- `created_repository.json` — the repository object, taken from the
  `rename-repository` scenario. `POST /user/repos` answers 201 with this same
  schema; only `clone_url` is read here.
- `validation_failed.json` — the 422 body GitHub returns when a resource
  already exists or is malformed, from the `errors` scenario.

Refresh them from
`scenarios/api.github.com/{rename-repository,errors}/normalized-fixture.json`.
