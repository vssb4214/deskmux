# Commit Convention

DeskMux uses [Conventional Commits](https://www.conventionalcommits.org/). The format keeps history scannable and makes changelog generation trivial.

## Format

```
<type>(<optional scope>): <description>

<optional body>

<optional footer>
```

- **description**: imperative mood, lowercase, no trailing period. "add config validation", not "Added config validation."
- Keep the subject line under ~72 characters.
- Use the body to explain *why*, not *what* — the diff shows what.

## Types

| Type       | Use for                                                        |
|------------|----------------------------------------------------------------|
| `feat`     | a new feature                                                  |
| `fix`      | a bug fix                                                      |
| `docs`     | documentation only                                            |
| `refactor` | code change that neither fixes a bug nor adds a feature        |
| `test`     | adding or correcting tests                                     |
| `chore`    | build process, tooling, dependencies                          |
| `ci`       | CI configuration                                              |
| `perf`     | a performance improvement                                     |
| `style`    | formatting, whitespace — no logic change                       |

## Scopes (suggested)

`config`, `executor`, `api`, `ui`, `peer` — match the part of the app you touched.

## Examples

```
feat(executor): apply preset layout sequentially with per-monitor logs
fix(config): report missing monitor input instead of panicking
docs(config): add macOS BetterDisplay command examples
ci: run build and clippy on windows and macos
refactor(api): extract preset resolution out of the request handler
```

## Breaking changes

Add a `!` after the type/scope, and explain in the footer:

```
feat(config)!: rename `inputs.pc` to `inputs.primary`

BREAKING CHANGE: existing config files must rename the `pc`/`mac`
input keys. See docs/CONFIG.md for the migration.
```
