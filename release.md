# Cutting a release of crux

## Dependencies between crates

The Crux crate depend on one another, and need to be released in the right order

1. `crux_macros`
2. `crux_core`
2. Capability crates (`crux_http crux_kv crux_platform crux_time`)

There are scripts to help with this.

## Steps

The easiest way of releasing is using `release-plz`

1. `release-plz update` - this should move the versions of all the crates according to the changes
   to their APIs. Note that this isn't bulletproof, because it doesn't consider all the changes
   which are in fact breaking as breaking.

   Things it misses: (may not be the complete list):
   * changes to crux_macros that generate incompatible code
   * changes to capability operation types
2. Optionally - open a PR with the changes to the versions for review
3. use `scripts/cargo_publish.fish` to publish the crates in the right order OR
   - publish `crux_macros` if it changed
   - publish `crux_core` if it changed
   - publish any capability crates that changed
4. tag the commit used for the release with [crate_name]-vX.Y.Z
5. for `crux_core` create a GitHub release with release notes, especially breaking changes