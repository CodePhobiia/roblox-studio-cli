# rs Bridge Plugin

This plugin registers the current Roblox Studio session with the local `rs` bridge, polls for commands, and executes `exec`, `read`, `serialize`, and `deserialize`.

Build with Rojo:

```powershell
rojo build default.project.json --output rs-bridge-plugin.rbxmx
```

Install the generated `.rbxmx` as a local Studio plugin and restart Studio.

The plugin expects the bridge at `http://127.0.0.1:7878` by default. Change `src/Config.lua` if you run the bridge on another port.
