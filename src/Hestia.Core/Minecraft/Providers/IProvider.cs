using Hestia.Core.Minecraft.Models;

namespace Hestia.Core.Minecraft.Providers;

public interface IProvider
{
    ServerType Type { get; }
    Task<List<MinecraftVersion>> GetVersionsAsync();
    Task<ResolvedServer> ResolveAsync(Server server);
}
