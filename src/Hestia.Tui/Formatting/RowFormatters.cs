using Hestia.Core.Jre;
using Hestia.Core.Monitoring;
using Hestia.Core.Server;

namespace Hestia.Tui.Formatting;

internal static class RowFormatters
{
    public static string ServerRow(MinecraftServer s)
    {
        var badge = s.State switch
        {
            ServerState.Running  => "[RUN]",
            ServerState.Starting => "[STR]",
            ServerState.Stopping => "[STP]",
            ServerState.Crashed  => "[CRH]",
            _                    => "[---]",
        };
        var name = Truncate(s.Name, 22);
        return $"{badge} {name,-22}  {s.MinecraftVersion,-8}  {s.Type}";
    }

    public static string JreRow(JavaRuntime r)
    {
        var dist = r.Distribution.ToString();
        var vendor = r.VendorString is { Length: > 0 } v ? $" ({Truncate(v, 16)})" : "";
        return $"  Java {r.MajorVersion,-3}  {dist,-8}{vendor}";
    }

    public static string FormatStatus(ServerStatus st)
    {
        var uptime = st.Uptime is { } u
            ? $"{(int)u.TotalHours:D2}:{u.Minutes:D2}:{u.Seconds:D2}"
            : "--:--:--";
        var tps = st.Tps is { } t ? $"{t:F1}" : "N/A";
        var mem = st.Resources is { } r
            ? $"{r.MemoryBytes / 1024 / 1024} MB / {r.MemoryLimitBytes / 1024 / 1024} MB"
            : "N/A";
        var cpu = st.Resources is { } rc ? $"{rc.CpuPercent:F1}%" : "N/A";
        return $"Players: {st.PlayerCount}/{st.MaxPlayers}  TPS: {tps}  Uptime: {uptime}  Mem: {mem}  CPU: {cpu}";
    }

    private static string Truncate(string s, int max) =>
        s.Length <= max ? s : s[..(max - 1)] + "…";
}
