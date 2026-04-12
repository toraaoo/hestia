using Hestia.Core.Server;

namespace Hestia.Tui.Services;

internal sealed class ServerCreateForm
{
    private readonly string _appDataDir;
    private readonly ServerType[] _types = [ServerType.Vanilla, ServerType.Paper, ServerType.Fabric];
    private readonly IReadOnlyList<string> _availableVersions;

    public enum Field { Name, Type, Version, Directory, Eula }

    public string Name { get; set; } = string.Empty;
    public ServerType Type { get; set; } = ServerType.Vanilla;
    public string Version { get; set; } = "1.21.4";
    public string Directory { get; set; }
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

    public void ToggleEula() => AcceptEula = !AcceptEula;

    public ServerType[] GetTypes() => _types;

    public IReadOnlyList<string> GetVersions() => _availableVersions;

    private void UpdateDirectoryFromName()
    {
        if (!string.IsNullOrWhiteSpace(Name))
            Directory = Path.Combine(_appDataDir, "servers", Name.ToLower().Replace(' ', '-'));
    }
}
