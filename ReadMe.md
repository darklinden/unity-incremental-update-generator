# Unity Hot Update Incremental Patch Tool

## Versions

-   Loader Version

    -   Position `Assets/Resources/Version.txt`
    -   The `apk`/`ipa`/`exe` version, will not be changed until the next package release.
    -   The Addressable Assets Catalog will be generated based on this version.
    -   Different Loader Version **`CAN NOT`** use the same incremental patch.

-   Game Patch Version

    -   Position `ServerData/Build Target/Version.txt`
    -   The Patch version, will be changed every time the game patch is released.

## Workflow

-   Using Loader Version to find older versions in git tags, and generate incremental Version
    -   The Tag name should be `LoaderVersion-GamePatchVersion`
-   Build full Patch Package For Configured Platforms
    -   ServerData/Build Target/Version.txt
-   Is It a new Loader Version?

    -   Yes => Create Full Patch Package => Done
    -   No => Continue

-   List git tags and select the base tag to build the incremental patches
-   For each tag, generate the incremental patch package

-   Done, Create Tag for the new Game Patch Version
