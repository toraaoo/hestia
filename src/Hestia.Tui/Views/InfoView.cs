using System.Net.NetworkInformation;
using System.Net.Sockets;
using Hestia.Core.Monitoring;
using Hestia.Core.Server;
using Hestia.Tui.Screens;
using Spectre.Console;
using Spectre.Console.Rendering;

namespace Hestia.Tui.Views;

internal static class InfoView
{
    public static IRenderable Render(MainViewModel vm)
    {
        var s = vm.LatestStatus;
        var focused = vm.ActivePane == Pane.Info;
        var server = vm.SelectedServer;

        var grid = new Grid()
            .AddColumn(new GridColumn().NoWrap().Width(12))
            .AddColumn(new GridColumn().NoWrap());

        void Row(string label, string val)
            => grid.AddRow(new Markup($"[dim]{label}[/]"), new Markup(val));

        if (server is not null)
        {
            var host = GetLocalIp();
            Row("Name", Markup.Escape(server.Name));
            Row("Type", Markup.Escape(server.Type.ToString()));
            Row("Version", Markup.Escape(server.MinecraftVersion));
            Row("Dir", $"[dim]{Markup.Escape(server.Options.ServerDirectory)}[/]");
            Row("Port", server.Options.Port.ToString());
            Row("Join", $"{host}:{server.Options.Port}");

            if (server.RconOptions.Enabled)
            {
                var pw = vm.ShowRconPassword
                    ? server.RconOptions.Password
                    : new string('*', Math.Clamp(server.RconOptions.Password.Length, 8, 24));
                Row("RCON", "[green]ON[/]");
                Row("RCON cmd", Markup.Escape($"mcrcon -H {host} -P {server.RconOptions.Port} -p {pw}"));
            }
            else
            {
                Row("RCON", "[red]OFF[/]");
            }

            grid.AddEmptyRow();
        }

        if (s is null)
        {
            Row("State", "[dim]---[/]");
            Row("Uptime", "[dim]---[/]");
            Row("Players", "[dim]---[/]");
            Row("TPS", "[dim]---[/]");
            Row("Memory", "[dim]---[/]");
            Row("CPU", "[dim]---[/]");
        }
        else
        {
            var stateColor = s.State switch
            {
                ServerState.Running  => "green",
                ServerState.Crashed  => "red",
                ServerState.Starting => "yellow",
                _                    => "dim",
            };
            var uptime = s.Uptime is { } u
                ? $"{(int)u.TotalHours:D2}:{u.Minutes:D2}:{u.Seconds:D2}"
                : "--:--:--";

            Row("State", $"[{stateColor}]{s.State}[/]");
            Row("Uptime", uptime);
            Row("Players", $"{s.PlayerCount}/{s.MaxPlayers}");
            Row("TPS", s.Tps is { } t ? $"{t:F1}" : "[dim]N/A[/]");
            Row("Memory", s.Resources is { } r
                ? $"{r.MemoryBytes / 1024 / 1024} MB / {r.MemoryLimitBytes / 1024 / 1024} MB"
                : "[dim]N/A[/]");
            Row("CPU", s.Resources is { } rc
                ? $"{rc.CpuPercent:F1}%"
                : "[dim]N/A[/]");

            if (s.OnlinePlayers.Count > 0)
            {
                grid.AddEmptyRow();
                grid.AddRow(
                    new Markup("[dim]Online[/]"),
                    new Markup(Markup.Escape(string.Join(", ", s.OnlinePlayers.Select(p => p.Username)))));
            }
        }

        const string TabBar = "[dim]Logs[/]  [bold underline]Info[/]";
        return new Panel(grid)
        {
            Header = new PanelHeader(TabBar),
            Border = focused ? BoxBorder.Double : BoxBorder.Rounded,
            Expand = true,
        };
    }

    private static string GetLocalIp()
    {
        try
        {
            foreach (var ni in NetworkInterface.GetAllNetworkInterfaces())
            {
                if (ni.OperationalStatus != OperationalStatus.Up) continue;
                if (ni.NetworkInterfaceType == NetworkInterfaceType.Loopback) continue;
                foreach (var addr in ni.GetIPProperties().UnicastAddresses)
                {
                    if (addr.Address.AddressFamily == AddressFamily.InterNetwork)
                        return addr.Address.ToString();
                }
            }
        }
        catch { }
        return "127.0.0.1";
    }
}
