using Hestia.Core.Server;

namespace Hestia.Tui.Services;

internal sealed class ServerCreateForm
{
    private readonly string _appDataDir;

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
        Advanced,
        Submit
    }

    public string Name { get; set; } = string.Empty;
    public ServerType Type { get; set; } = ServerType.Vanilla;
    public string Version { get; set; } = "1.21.4";
    public string Directory { get; set; }
    public int ServerPort { get; set; } = 25565;
    public int MaxPlayers { get; set; } = 20;
    public string MotD { get; private set; } = "A Minecraft Server";
    public int ViewDistance { get; set; } = 10;
    public bool OnlineMode { get; private set; } = true;
    public bool Whitelist { get; private set; }
    public string LevelName { get; private set; } = "world";
    public string Difficulty { get; private set; } = "easy";

    public int RconPort { get; set; } = 25575;
    public string RconPassword { get; private set; } = "hestia";
    public bool RconEnabled { get; private set; } = true;
    public int RconTimeoutSeconds { get; set; } = 5;

    public string JvmMinMemory { get; private set; } = "512M";
    public string JvmMaxMemory { get; private set; } = "2G";
    public string JvmAdditionalFlags { get; set; } = string.Empty;

    public bool AcceptEula { get; set; }
    public Field SelectedField { get; set; } = Field.Name;

    public ServerType[] Types { get; } = [ServerType.Vanilla, ServerType.Paper, ServerType.Fabric];

    public ServerCreateForm(string appDataDir, IReadOnlyList<string> availableVersions)
    {
        _appDataDir = appDataDir;
        Directory = Path.Combine(appDataDir, "servers", "new-server");
    }

    public void MoveUp(IReadOnlyList<Field> visibleFields)
    {
        if (visibleFields.Count == 0) return;
        var i = visibleFields.IndexOf(SelectedField);
        if (i < 0) i = 0;
        i = i == 0 ? visibleFields.Count - 1 : i - 1;
        SelectedField = visibleFields[i];
    }

    public void MoveDown(IReadOnlyList<Field> visibleFields)
    {
        if (visibleFields.Count == 0) return;
        var i = visibleFields.IndexOf(SelectedField);
        if (i < 0) i = 0;
        i = i == visibleFields.Count - 1 ? 0 : i + 1;
        SelectedField = visibleFields[i];
    }

    public void SetName(string value)
    {
        if (!string.IsNullOrWhiteSpace(value))
        {
            Name = value;
            UpdateDirectoryFromName();
        }
    }

    public void SetDirectory(string value)
    {
        if (!string.IsNullOrWhiteSpace(value)) Directory = value;
    }

    public void SetMotD(string value)
    {
        if (!string.IsNullOrWhiteSpace(value)) MotD = value;
    }

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

    public void SetRconPassword(string value)
    {
        if (!string.IsNullOrWhiteSpace(value)) RconPassword = value;
    }

    public void ToggleRconEnabled() => RconEnabled = !RconEnabled;

    public void SetJvmMinMemory(string value)
    {
        if (!string.IsNullOrWhiteSpace(value)) JvmMinMemory = value;
    }

    public void SetJvmMaxMemory(string value)
    {
        if (!string.IsNullOrWhiteSpace(value)) JvmMaxMemory = value;
    }

    public bool IsTextEditable(Field field) => field is
        Field.Name or Field.Directory or Field.ServerPort or
        Field.MaxPlayers or Field.MotD or Field.ViewDistance or
        Field.LevelName or Field.Difficulty or Field.RconPort or
        Field.RconPassword or Field.RconTimeoutSeconds or
        Field.JvmMinMemory or Field.JvmMaxMemory or Field.JvmAdditionalFlags;

    public string GetTextValue(Field field) => field switch
    {
        Field.Name => Name,
        Field.Directory => Directory,
        Field.ServerPort => ServerPort.ToString(),
        Field.MaxPlayers => MaxPlayers.ToString(),
        Field.MotD => MotD,
        Field.ViewDistance => ViewDistance.ToString(),
        Field.LevelName => LevelName,
        Field.Difficulty => Difficulty,
        Field.RconPort => RconPort.ToString(),
        Field.RconPassword => RconPassword,
        Field.RconTimeoutSeconds => RconTimeoutSeconds.ToString(),
        Field.JvmMinMemory => JvmMinMemory,
        Field.JvmMaxMemory => JvmMaxMemory,
        Field.JvmAdditionalFlags => JvmAdditionalFlags,
        _ => string.Empty
    };

    private void UpdateDirectoryFromName()
    {
        if (!string.IsNullOrWhiteSpace(Name))
            Directory = Path.Combine(_appDataDir, "servers", Name.ToLower().Replace(' ', '-'));
    }
}

internal static class ServerCreateFormFieldListExtensions
{
    public static int IndexOf(this IReadOnlyList<ServerCreateForm.Field> list, ServerCreateForm.Field value)
    {
        for (var i = 0; i < list.Count; i++)
            if (list[i] == value)
                return i;
        return -1;
    }
}
