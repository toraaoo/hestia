using Hestia.Core.Abstractions;
using Hestia.Core.Monitoring;
using Hestia.Tui.Services;

namespace Hestia.Tui.ViewModels;

internal sealed class ServerSessionVm : IAsyncDisposable
{
    private readonly IHestiaService _service;
    private readonly UiDispatcher _ui;
    private CancellationTokenSource? _cts;

    public Guid ServerId { get; }
    public RingBuffer<string> LogBuffer { get; } = new(5_000);
    public ServerStatus? LatestStatus { get; private set; }

    public event Action? Changed;

    public ServerSessionVm(IHestiaService service, UiDispatcher ui, Guid serverId)
    {
        _service = service;
        _ui = ui;
        ServerId = serverId;
    }

    public void Start()
    {
        _cts = new CancellationTokenSource();
        var ct = _cts.Token;
        _ = RunLogsAsync(ct);
        _ = RunStatsAsync(ct);
    }

    private async Task RunLogsAsync(CancellationToken ct)
    {
        try
        {
            await foreach (var line in _service.StreamLogsAsync(ServerId, ct).WithCancellation(ct))
            {
                LogBuffer.Add(line);
                _ui.Post(() => Changed?.Invoke());
            }
        }
        catch (OperationCanceledException) { }
        catch (Exception ex)
        {
            LogBuffer.Add($"[stream error] {ex.Message}");
            _ui.Post(() => Changed?.Invoke());
        }
    }

    private async Task RunStatsAsync(CancellationToken ct)
    {
        try
        {
            await foreach (var status in _service
                .WatchStatusAsync(ServerId, TimeSpan.FromSeconds(2), ct)
                .WithCancellation(ct))
            {
                LatestStatus = status;
                _ui.Post(() => Changed?.Invoke());
            }
        }
        catch (OperationCanceledException) { }
        catch
        {
            LatestStatus = null;
            _ui.Post(() => Changed?.Invoke());
        }
    }

    public async ValueTask DisposeAsync()
    {
        if (_cts is not null)
        {
            await _cts.CancelAsync();
            _cts.Dispose();
            _cts = null;
        }
    }
}
