using Hestia.Core.Abstractions;
using Hestia.Core.Server;
using Hestia.Tui.Formatting;

namespace Hestia.Tui.ViewModels;

internal sealed class ServerListVm
{
    private readonly IHestiaService _service;
    private List<MinecraftServer> _servers = [];

    public IReadOnlyList<MinecraftServer> Servers => _servers;
    public List<string> Rows { get; private set; } = [];

    public event Action? Changed;

    public ServerListVm(IHestiaService service) => _service = service;

    public async Task RefreshAsync(CancellationToken ct = default)
    {
        var list = await _service.GetServersAsync(ct);
        _servers = [.. list];
        Rows = _servers.Select(RowFormatters.ServerRow).ToList();
        Changed?.Invoke();
    }

    public MinecraftServer? GetAt(int index) =>
        index >= 0 && index < _servers.Count ? _servers[index] : null;

    public async Task StartAsync(Guid serverId, CancellationToken ct = default)
    {
        await _service.StartServerAsync(serverId, ct);
        await RefreshAsync(ct);
    }

    public async Task StopAsync(Guid serverId, CancellationToken ct = default)
    {
        await _service.StopServerAsync(serverId, gracePeriod: default, ct: ct);
        await RefreshAsync(ct);
    }

    public async Task RestartAsync(Guid serverId, CancellationToken ct = default)
    {
        await _service.RestartServerAsync(serverId, gracePeriod: default, ct: ct);
        await RefreshAsync(ct);
    }

    public async Task DeleteAsync(Guid serverId, CancellationToken ct = default)
    {
        await _service.DeleteServerAsync(serverId, ct);
        await RefreshAsync(ct);
    }
}
