namespace Hestia.Core.Utils;

internal static class ServerPropertiesFile
{
    internal static Dictionary<string, string> Read(string path)
    {
        if (!File.Exists(path))
            return [];

        var result = new Dictionary<string, string>();
        foreach (var line in File.ReadAllLines(path))
        {
            if (line.StartsWith('#') || line.StartsWith('!') || string.IsNullOrWhiteSpace(line))
                continue;

            var sep = line.IndexOf('=');
            if (sep < 0)
                continue;

            var key = line[..sep].Trim();
            var value = line[(sep + 1)..];
            result[key] = value;
        }
        return result;
    }

    internal static void Update(string path, Dictionary<string, string> updates)
    {
        var props = Read(path);
        foreach (var (key, value) in updates)
            props[key] = value;

        var lines = props.Select(kv => $"{kv.Key}={kv.Value}");
        File.WriteAllLines(path, lines);
    }
}
