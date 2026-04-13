namespace Hestia.Tui.Utilities;

internal static class ServerUtils
{
    public static bool IsValidPort(int port) => port is >= 1 and <= 65535;

    public static bool TryParsePort(string value, out int port)
    {
        port = 0;
        return int.TryParse(value.Trim(), out port) && IsValidPort(port);
    }

    public static int FindNextFreePort(int start, HashSet<int> used)
    {
        for (var p = Math.Max(1, start); p <= 65535; p++)
            if (!used.Contains(p))
                return p;

        return start;
    }

    public static List<string> SplitJvmFlags(string? raw)
    {
        var s = raw?.Trim();
        if (string.IsNullOrWhiteSpace(s)) return [];
        return s.Split(' ', StringSplitOptions.RemoveEmptyEntries | StringSplitOptions.TrimEntries).ToList();
    }

    public static List<string> FilterVersions(IReadOnlyList<string> all, string query)
    {
        if (string.IsNullOrWhiteSpace(query))
            return [.. all];

        var q = query.Trim();
        var res = new List<string>(all.Count);
        for (var i = 0; i < all.Count; i++)
        {
            var v = all[i];
            if (v.Contains(q, StringComparison.OrdinalIgnoreCase))
                res.Add(v);
        }

        return res;
    }

    public static int FindIndex(IReadOnlyList<string> list, string value)
    {
        for (var i = 0; i < list.Count; i++)
            if (list[i] == value)
                return i;
        return -1;
    }
}
