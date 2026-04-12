namespace Hestia.Core.Server;

public enum ServerType { Vanilla, Paper, Fabric }

public enum ServerState { Stopped, Starting, Running, Stopping, Crashed }

public enum EulaState { Pending, Accepted, Rejected }

public sealed record ServerOptions(
    string ServerDirectory,
    int Port = 25565,
    int MaxPlayers = 20,
    string MotD = "A Minecraft Server",
    int ViewDistance = 10,
    bool OnlineMode = true,
    bool Whitelist = false,
    string LevelName = "world",
    string Difficulty = "easy");

public sealed record RconOptions(
    int Port = 25575,
    string Password = "hestia",
    bool Enabled = true,
    int ConnectTimeoutSeconds = 5);

public sealed record JvmOptions(
    string MinMemory = "512M",
    string MaxMemory = "2G",
    IReadOnlyList<string>? AdditionalFlags = null);

public sealed record MinecraftServer(
    Guid Id,
    string Name,
    string MinecraftVersion,
    ServerType Type,
    ServerState State,
    EulaState EulaState,
    ServerOptions Options,
    RconOptions RconOptions,
    JvmOptions JvmOptions,
    string JavaRuntimeId);

public sealed record CreateServerOptions(
    string Name,
    string MinecraftVersion,
    string ServerDirectory,
    ServerType Type = ServerType.Vanilla,
    ServerOptions? Options = null,
    RconOptions? RconOptions = null,
    JvmOptions? JvmOptions = null,
    string? JavaRuntimeId = null,
    bool AcceptEula = false);
