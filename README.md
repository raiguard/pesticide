# Pesticide

A command-line UI for the [Debug Adapter Protocol](https://microsoft.github.io/debug-adapter-protocol/).

## Installation

Install [Go](https://golang.org) and run

```
go install github.com/raiguard/pesticide
```

This will install the `pesticide` executable into your `go/bin` directory.

## Usage / configuration

Place a `pesticide.json` file in your project directory and specify your adapter configurations. The file is JSON formatted:

```json
{
  "adapters": {
    "fmtk": {
      "cmd": "fmtk debug /home/rai/dev/factorio/1.1/bin/x64/factorio",
      "args": {
        "hookControl": [ "UltimateResearchQueue" ],
        "modsPath": "/home/rai/dev/factorio/1.1/mods"
      }
    },
    "mock": {
      "cmd": "mockserver",
      "addr": ":54321"
    }
  }
}
```

- `cmd`: A command-line command to execute.
- `addr`: An IP address to connect to. This can be used in combination with `cmd`.
- `args`: Any adapter-specific arguments.

Launch the `pesticide` executable and it will source the configuration file. You can now run commands.

### Current commands

**NOTE:** A heavy rewrite is underway. This list represents all currently functioning commands at the time of writing.

- `launch <adapter name>`
- `pause`
- `continue`
- `quit`
