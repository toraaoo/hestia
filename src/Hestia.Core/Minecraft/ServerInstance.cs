using System.Diagnostics;
using System.Globalization;
using System.Text.RegularExpressions;
using Hestia.Core.Minecraft.Models;
using Hestia.Core.Minecraft.Rcon;

namespace Hestia.Core.Minecraft;

public sealed class ServerInstance : IAsyncDisposable
{
    private static readonly Regex ListPattern = new(
        @"There are (?<current>\d+) of a max of (?<max>\d+) players online",
        RegexOptions.Compiled);

    private static readonly Regex TpsPattern = new(
        @"TPS from last 1m, 5m, 15m: (?<tps1>[\d.]+),",
        RegexOptions.Compiled);

    private readonly Process _process;
    private readonly RconClient _rcon = new();
    private bool _rconConnected;

    public Server Server { get; }
    public IObservable<string> Output { get; }
    public bool IsRunning => !_process.HasExited;
    public DateTime StartedAt { get; }

    internal ServerInstance(Server server, Process process, IObservable<string> output)
    {
        Server = server;
        _process = process;
        Output = output;
        StartedAt = DateTime.UtcNow;
    }

    public async Task<string> SendCommandAsync(string command)
    {
        await EnsureRconAsync();
        return await _rcon.SendCommandAsync(command);
    }

    public async Task<ServerMetrics> GetMetricsAsync()
    {
        var listResponse = await SendCommandAsync("list");
        var listMatch = ListPattern.Match(listResponse);
        if (!listMatch.Success)
            throw new HestiaException($"Unexpected response to 'list' command: {listResponse}");

        var currentPlayers = int.Parse(listMatch.Groups["current"].Value);
        var maxPlayers = int.Parse(listMatch.Groups["max"].Value);

        double? tps = null;
        var tpsResponse = await SendCommandAsync("tps");
        if (!tpsResponse.Contains("Unknown command", StringComparison.OrdinalIgnoreCase))
        {
            var tpsMatch = TpsPattern.Match(tpsResponse);
            if (tpsMatch.Success)
                tps = double.Parse(tpsMatch.Groups["tps1"].Value, CultureInfo.InvariantCulture);
        }

        var connectUrl = Server.Port == 25565
            ? Server.Host
            : $"{Server.Host}:{Server.Port}";

        return new ServerMetrics
        {
            CurrentPlayers = currentPlayers,
            MaxPlayers = maxPlayers,
            Tps = tps,
            Uptime = DateTime.UtcNow - StartedAt,
            ConnectUrl = connectUrl,
        };
    }

    public async Task StopAsync()
    {
        if (!IsRunning)
            return;

        try
        {
            await SendCommandAsync("stop");
        }
        catch
        {
            // RCON may not be ready yet — fall through to kill
        }

        if (!_process.WaitForExit(TimeSpan.FromSeconds(15)))
            _process.Kill(entireProcessTree: true);
    }

    public async ValueTask DisposeAsync()
    {
        if (IsRunning)
            await StopAsync();

        _rcon.Dispose();
        _process.Dispose();
    }

    private async Task EnsureRconAsync()
    {
        if (_rconConnected)
            return;

        await _rcon.ConnectAsync(Server.Host, Server.RconPort, Server.RconPassword);
        _rconConnected = true;
    }
}
