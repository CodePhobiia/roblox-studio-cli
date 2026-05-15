# Architecture

`rs` has three runtime pieces:

1. `rs` in CLI mode: short-lived commands such as `list`, `exec`, `read`, and `transfer`.
2. `rs bridge serve`: a local HTTP daemon on `127.0.0.1:7878`.
3. `rs-bridge-plugin`: a Roblox Studio plugin that registers, heartbeats, long-polls for commands, and posts results.

CLI commands probe `/healthz`. If the bridge is missing, they spawn `rs bridge serve --port <port>` and wait up to three seconds.

Studio sessions are selected by UUID, exact name, or unique case-insensitive substring. If more than one Studio matches, the bridge returns an ambiguity error and the CLI asks the user to disambiguate by UUID.

The bridge stores one queue per Studio session. `exec`, `read`, `export`, and `import-asset` enqueue one plugin command and wait for its result. `export` returns file payloads that the CLI writes to disk after validating every relative path. `import-asset` parses local OBJ/STL/glTF/GLB geometry in the CLI, or asks headless Blender to convert other supported DCC formats to GLB, then sends triangle mesh payloads to Studio. The plugin turns those payloads into welded MeshParts through `EditableMesh`. `transfer` enqueues a `serialize` command on the source Studio, then a `deserialize` command on the target Studio with the returned blob.
