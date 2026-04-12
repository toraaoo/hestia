using Hestia.Core.Server;

namespace Hestia.Tui.Services;

internal sealed class ServerCreateForm
{
    private readonly string _appDataDir;
    private readonly ServerType[] _types = [ServerType.Vanilla, ServerType.Paper, ServerType.Fabric];
    private readonly IReadOnlyList<string> _availableVersions;

    public enum Field
    {
        Name,
        Type,
        Version,
        Directory,
        ServerPort,
        MaxPlayers,
        MotD,
        ViewDistance,
        OnlineMode,
        Whitelist,
        LevelName,
        Difficulty,
        RconEnabled,
        RconPort,
        RconPassword,
        RconTimeoutSeconds,
        JvmMinMemory,
        JvmMaxMemory,
        JvmAdditionalFlags,
        Eula
    }

    public string Name { get; set; } = string.Empty;
    public ServerType Type { get; set; } = ServerType.Vanilla;
    public string Version { get; set; } = "1.21.4";
    public string Directory { get; set; }
    public int ServerPort { get; private set; } = 25565;
    public int MaxPlayers { get; private set; } = 20;
    public string MotD { get; private set; } = "A Minecraft Server";
    public int ViewDistance { get; private set; } = 10;
    public bool OnlineMode { get; private set; } = true;
    public bool Whitelist { get; private set; }
    public string LevelName { get; private set; } = "world";
    public string Difficulty { get; private set; } = "easy";

    public int RconPort { get; private set; } = 25575;
    public string RconPassword { get; private set; } = "hestia";
    public bool RconEnabled { get; private set; } = true;
    public int RconTimeoutSeconds { get; private set; } = 5;

    public string JvmMinMemory { get; private set; } = "512M";
    public string JvmMaxMemory { get; private set; } = "2G";
    public string JvmAdditionalFlags { get; private set; } = string.Empty;

    public bool AcceptEula { get; set; }
    public Field SelectedField { get; set; } = Field.Name;

    public ServerCreateForm(string appDataDir, IReadOnlyList<string> availableVersions)
    {
        _appDataDir = appDataDir;
        _availableVersions = availableVersions;
        Directory = Path.Combine(appDataDir, "servers", "new-server");
    }

    public void MoveUp()
    {
        SelectedField = SelectedField == Field.Name ? Field.Eula : (Field)(SelectedField - 1);
    }

    public void MoveDown()
    {
        SelectedField = SelectedField == Field.Eula ? Field.Name : (Field)(SelectedField + 1);
    }

    public Field GetFieldToEdit() => SelectedField;

    public void SetName(string value)
    {
        if (!string.IsNullOrWhiteSpace(value))
        {
            Name = value;
            UpdateDirectoryFromName();
        }
    }

    public void SetType(ServerType value) => Type = value;

    public void SetVersion(string value)
    {
        if (!string.IsNullOrWhiteSpace(value)) Version = value;
    }

    public void SetDirectory(string value)
    {
        if (!string.IsNullOrWhiteSpace(value)) Directory = value;
    }

    public void SetServerPort(int value) => ServerPort = value;

    public void SetMaxPlayers(int value) => MaxPlayers = value;

    public void SetMotD(string value)
    {
        if (!string.IsNullOrWhiteSpace(value)) MotD = value;
    }

    public void SetViewDistance(int value) => ViewDistance = value;

    public void ToggleOnlineMode() => OnlineMode = !OnlineMode;

    public void ToggleWhitelist() => Whitelist = !Whitelist;

    public void SetLevelName(string value)
    {
        if (!string.IsNullOrWhiteSpace(value)) LevelName = value;
    }

    public void SetDifficulty(string value)
    {
        if (!string.IsNullOrWhiteSpace(value)) Difficulty = value;
    }

    public void SetRconPort(int value) => RconPort = value;

    public void SetRconPassword(string value)
    {
        if (!string.IsNullOrWhiteSpace(value)) RconPassword = value;
    }

    public void ToggleRconEnabled() => RconEnabled = !RconEnabled;

    public void SetRconTimeoutSeconds(int value) => RconTimeoutSeconds = value;

    public void SetJvmMinMemory(string value)
    {
        if (!string.IsNullOrWhiteSpace(value)) JvmMinMemory = value;
    }

    public void SetJvmMaxMemory(string value)
    {
        if (!string.IsNullOrWhiteSpace(value)) JvmMaxMemory = value;
    }

    public void SetJvmAdditionalFlags(string value) => JvmAdditionalFlags = value ?? string.Empty;

    public void ToggleEula() => AcceptEula = !AcceptEula;

    public ServerType[] GetTypes() => _types;

    public IReadOnlyList<string> GetVersions() => _availableVersions;

    private void UpdateDirectoryFromName()
    {
        if (!string.IsNullOrWhiteSpace(Name))
            Directory = Path.Combine(_appDataDir, "servers", Name.ToLower().Replace(' ', '-'));
    }
}
