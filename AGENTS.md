# Project Instructions

## Responses
- Keep responses concise
- Ask clarifying questions when instructions are unclear

## Code standards
- Tool chain is pinned via `rust-toolchain.toml`
- No `dbg!`, `TODO`, `FIXME`, `DEBUG:`, or `FIXTURE:` strings anywhere in `.rs` files — `scripts/test.sh` greps for these and fails the build
- New business logic should have tests

## Tests
- Unit tests live inline at the bottom of each source file in `#[cfg(test)] mod tests`, not in separate files

## Commands
- Run `./scripts/test.sh` to run checks 

## Commits and PRs
- Code comments should describe what and why but not how
- `main` is the base branch when reviewing code
