# Hestia

[![.NET](https://img.shields.io/badge/.NET-8.0-512BD4?logo=dotnet&logoColor=white)](https://dotnet.microsoft.com/)
[![Avalonia](https://img.shields.io/badge/Avalonia-12.0-335CFF?logo=avalonia&logoColor=white)](https://avaloniaui.net/)
[![Spectre.Console](https://img.shields.io/badge/Spectre.Console-0.49-111827?logo=windowsterminal&logoColor=white)](https://spectreconsole.net/)

Hestia is a .NET 8 tool for managing local Minecraft servers (Vanilla, Paper, Fabric) with:

- a terminal UI (`Hestia.Tui`, Spectre.Console)
- a desktop UI (`Hestia.Desktop`, Avalonia)
- shared core logic (`Hestia.Core`) for server/JRE/RCON/monitoring

## Prerequisites

- .NET SDK 8.x
- Network access to download:
    - server jars (Mojang, PaperMC, Fabric)
    - Java runtimes (Adoptium Temurin)

## Build / Test

```bash
dotnet build Hestia.sln -c Release
dotnet test  Hestia.sln -c Release
```

Run a single test project:

```bash
dotnet test tests/Hestia.Core.Tests/Hestia.Core.Tests.csproj -c Release
```

## Run

Terminal UI:

```bash
dotnet run --project src/Hestia.Tui/Hestia.Tui.csproj -c Release
```

Desktop UI:

```bash
dotnet run --project src/Hestia.Desktop/Hestia.Desktop.csproj -c Release
```

## Data Location / Side Effects

- App state is stored in an OS-specific app-data directory:
    - Windows: `%APPDATA%/Hestia`
    - Linux: `~/.hestia`
- Servers are persisted to `<appDataDir>/servers.json`.
- Creating a server creates a server directory and downloads `server.jar` into it.
- Deleting a server deletes its server directory recursively.
- Starting a server spawns a `java` process in the server directory and appends logs under `<serverDir>/logs/`.

## Security Notes

- RCON is supported; treat the RCON password as a secret.
- The create-server flow defaults the RCON password to `hestia`; change it.
