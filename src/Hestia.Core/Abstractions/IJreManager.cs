using Hestia.Core.Jre;

namespace Hestia.Core.Abstractions;

public interface IJreManager
{
    ValueTask<IReadOnlyList<JavaRuntime>> GetInstalledRuntimesAsync(CancellationToken ct = default);

    IAsyncEnumerable<JavaRuntime> DetectSystemRuntimesAsync(CancellationToken ct = default);

    ValueTask<JavaRuntime> InstallRuntimeAsync(
        JreInstallOptions options,
        IProgress<double>? progress = null,
        CancellationToken ct = default);

    ValueTask RemoveRuntimeAsync(string runtimeId, CancellationToken ct = default);

    ValueTask<JavaRuntime?> ResolveRuntimeForVersionAsync(
        string minecraftVersion,
        CancellationToken ct = default);
}
