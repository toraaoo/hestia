using Hestia.Core.Minecraft.Models;
using Hestia.Core.Utils;

namespace Hestia.Core.Minecraft;

public class WorldManager(AppDataFileSystem fs, Manager serverManager)
{
    private const string LevelNameKey = "level-name";
    private const string DefaultWorldName = "world";

    public IReadOnlyList<WorldInfo> ListWorlds(Guid serverId)
    {
        var serverDir = fs.Servers.GetServerDir(serverId);
        var active = GetActiveWorldName(serverId);

        var diskWorlds = Directory.Exists(serverDir)
            ? Directory.GetDirectories(serverDir)
                .Where(IsWorldDirectory)
                .Select(Path.GetFileName)
                .Where(n => n is not null)
                .Select(n => n!)
                .ToHashSet()
            : [];

        var result = diskWorlds
            .Select(name => new WorldInfo(name, IsActive: name == active, ExistsOnDisk: true))
            .ToList();

        // Active world from server.properties may not exist on disk yet (never started)
        if (!diskWorlds.Contains(active))
            result.Add(new WorldInfo(active, IsActive: true, ExistsOnDisk: false));

        return result;
    }

    public string GetActiveWorldName(Guid serverId)
    {
        var propsPath = fs.Servers.GetPropertiesPath(serverId);
        var props = ServerPropertiesFile.Read(propsPath);
        return props.TryGetValue(LevelNameKey, out var name) && !string.IsNullOrWhiteSpace(name)
            ? name
            : DefaultWorldName;
    }

    public void SwitchWorld(Guid serverId, string worldName)
    {
        ThrowIfRunning(serverId);

        var serverDir = fs.Servers.GetServerDir(serverId);
        var targetDir = Path.Combine(serverDir, worldName);
        if (!IsWorldDirectory(targetDir))
            throw new HestiaException($"World '{worldName}' does not exist on disk for server '{serverId}'.");

        var propsPath = fs.Servers.GetPropertiesPath(serverId);
        ServerPropertiesFile.Update(propsPath, new Dictionary<string, string>
        {
            [LevelNameKey] = worldName,
        });
    }

    public void CreateWorld(Guid serverId, WorldConfig config)
    {
        ThrowIfRunning(serverId);

        var propsPath = fs.Servers.GetPropertiesPath(serverId);
        ServerPropertiesFile.Update(propsPath, new Dictionary<string, string>
        {
            ["level-name"]         = config.Name,
            ["level-seed"]         = config.Seed ?? "",
            ["default-game-mode"]  = WorldPropertyValues.Of(config.GameMode),
            ["difficulty"]         = WorldPropertyValues.Of(config.Difficulty),
        });
    }

    public void DeleteWorld(Guid serverId, string worldName)
    {
        ThrowIfRunning(serverId);

        var worldDir = Path.Combine(fs.Servers.GetServerDir(serverId), worldName);
        if (!Directory.Exists(worldDir))
            throw new HestiaException($"World '{worldName}' not found for server '{serverId}'.");

        Directory.Delete(worldDir, recursive: true);
    }

    private void ThrowIfRunning(Guid serverId)
    {
        if (serverManager.IsRunning(serverId))
            throw new HestiaException($"Cannot modify worlds for server '{serverId}' while it is running.");
    }

    private static bool IsWorldDirectory(string path) =>
        Directory.Exists(path) && File.Exists(Path.Combine(path, "level.dat"));
}
