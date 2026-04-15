namespace Hestia.Core.Minecraft.Models;

/// <param name="Name">World directory name (matches level-name in server.properties).</param>
/// <param name="IsActive">True if this world is the current level-name.</param>
/// <param name="ExistsOnDisk">True if the directory with level.dat exists on disk.</param>
public record WorldInfo(string Name, bool IsActive, bool ExistsOnDisk);
