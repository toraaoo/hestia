using Hestia.Core.Server;

namespace Hestia.Core.Abstractions;

public interface IServerProvider
{
    ServerType ServerType { get; }

    ValueTask<IReadOnlyList<string>> GetAvailableVersionsAsync(CancellationToken ct = default);

    ValueTask DownloadServerJarAsync(
        string minecraftVersion,
        string destPath,
        IProgress<double>? progress = null,
        CancellationToken ct = default);
}
