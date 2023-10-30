# TODO

- deduplication algorithm for link items:

  maintain a map targetUnreferencedPlatforms: target -> platforms

  for each component:
    for each platform:
      for each target:
        if targetUnreferencedPlatforms[target] doesn't exist {
          insert targetUnreferencedPlatforms[target] = { all platforms }
        }
        targetUnreferencedPlatforms[target] -= platform
  for each component:
    for each target:
      gate each target as follows:
        targetReferencedPlatforms[target] = { allPlatforms } - targetUnreferencedPlatforms[target]
        #[cfg(all(feature = <feature-name>, any(platform = ..targetReferencedPlatforms[target])))]
        // #[cfg(all(feature = <feature-name>, not(any(platform = ..targetUnreferencedPlatforms[target]))))]
      where "platform =" is expanded out to "target_os", "target_arch", "target_env" as needed

- modify download functionality to allow fetching component tarballs or just standalone manifests
- modify analysis and generation phases to only fetch and process only standalone manifests
- modify feature to process all platforms for each component
  - this should work fine since external (system-specific) dependencies will not be included in features anyway
- work on error ergonomics
  - from/into chains
  - figure out which functions should output higher level errors (probably the public facing API?)
- allow fetching tarballs from a specified directory
- revisit visibility of all items
- define build configurator for rust cc crate Build struct based on `llvmup.json` manifest contents
