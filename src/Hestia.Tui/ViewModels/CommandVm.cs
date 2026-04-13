using Hestia.Core.Abstractions;

namespace Hestia.Tui.ViewModels;

internal sealed class CommandVm
{
    private readonly IHestiaService _service;
    private readonly List<string> _history = [];
    private int _historyIndex = -1;

    public event Action<string>? LineAppended;
    public event Action<string>? StatusChanged;

    public CommandVm(IHestiaService service) => _service = service;

    public void HistoryUp(ref string current)
    {
        if (_history.Count == 0) return;
        if (_historyIndex < 0) _historyIndex = _history.Count;
        _historyIndex = Math.Max(0, _historyIndex - 1);
        current = _history[_historyIndex];
    }

    public void HistoryDown(ref string current)
    {
        if (_historyIndex < 0) return;
        _historyIndex++;
        if (_historyIndex >= _history.Count)
        {
            _historyIndex = -1;
            current = string.Empty;
        }
        else
        {
            current = _history[_historyIndex];
        }
    }

    public async Task SendAsync(Guid serverId, string command, CancellationToken ct = default)
    {
        if (string.IsNullOrWhiteSpace(command)) return;

        _history.Add(command);
        _historyIndex = -1;

        try
        {
            var response = await _service.SendCommandAsync(serverId, command, ct);
            if (response.IsError)
            {
                LineAppended?.Invoke($"[rcon] error: {response.Payload}");
                StatusChanged?.Invoke($"RCON error: {response.Payload}");
            }
            else
            {
                LineAppended?.Invoke($"[rcon] > {command}");
                if (!string.IsNullOrWhiteSpace(response.Payload))
                    LineAppended?.Invoke($"[rcon] {response.Payload}");
            }
        }
        catch (Exception ex)
        {
            var msg = ex.Message;
            LineAppended?.Invoke($"[rcon] error: {msg}");
            StatusChanged?.Invoke($"RCON failed: {msg}");
        }
    }
}
