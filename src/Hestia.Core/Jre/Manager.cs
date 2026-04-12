using System.Collections.Concurrent;
using System.Diagnostics;
using System.Net.Http.Json;
using System.Runtime.CompilerServices;
using System.Text.Json;
using System.Text.Json.Serialization;
using Hestia.Core.Abstractions;

namespace Hestia.Core.Jre;

public sealed class Manager : IJreManager
{
    private readonly string _appDataDir;
    private readonly HttpClient _http;
    private readonly IEventBus _eventBus;
    private readonly string _runtimesFile;
    private readonly string _managedJreDir;
    private readonly SemaphoreSlim _persistLock = new(1, 1);
    private readonly ConcurrentDictionary<string, JavaRuntime> _runtimes = new();

    private static readonly JsonSerializerOptions JsonOptions = new()
    {
        WriteIndented = true,
        Converters = { new JsonStringEnumConverter() }
    };

    public Manager(string appDataDir, HttpClient http, IEventBus eventBus)
    {
        _appDataDir = appDataDir;
        _http = http;
        _eventBus = eventBus;
        _runtimesFile = Path.Combine(appDataDir, "jres.json");
        _managedJreDir = Path.Combine(appDataDir, "jres");
        LoadPersistedRuntimes();
    }

    public ValueTask<IReadOnlyList<JavaRuntime>> GetInstalledRuntimesAsync(
        CancellationToken ct = default)
    {
        IReadOnlyList<JavaRuntime> list = _runtimes.Values.ToList().AsReadOnly();
        return ValueTask.FromResult(list);
    }

    public async IAsyncEnumerable<JavaRuntime> DetectSystemRuntimesAsync(
        [EnumeratorCancellation] CancellationToken ct = default)
    {
        var candidates = EnumerateJavaCandidates();

        foreach (var path in candidates)
        {
            ct.ThrowIfCancellationRequested();
            var runtime = await TryProbeJavaAsync(path, ct).ConfigureAwait(false);
            if (runtime is null) continue;

            _runtimes[runtime.Id] = runtime;
            await PersistAsync(ct).ConfigureAwait(false);
            await _eventBus.PublishAsync(new JreDetectedEvent(runtime), ct).ConfigureAwait(false);
            yield return runtime;
        }
    }

    public async ValueTask<JavaRuntime> InstallRuntimeAsync(
        JreInstallOptions options,
        IProgress<double>? progress = null,
        CancellationToken ct = default)
    {
        var (downloadUrl, archiveName) = await ResolveAdoptiumDownloadAsync(options, ct)
            .ConfigureAwait(false);

        var destDir = Path.Combine(_managedJreDir, $"temurin-{options.MajorVersion}");
        Directory.CreateDirectory(destDir);

        var archivePath = Path.Combine(destDir, archiveName);
        await DownloadWithProgressAsync(downloadUrl, archivePath, progress, ct).ConfigureAwait(false);

        await ExtractArchiveAsync(archivePath, destDir, ct).ConfigureAwait(false);
        File.Delete(archivePath);

        var javaBinary = FindJavaBinaryInDirectory(destDir)
            ?? throw new InvalidOperationException(
                $"Could not locate java binary after extracting to '{destDir}'.");

        var runtime = await TryProbeJavaAsync(javaBinary, ct).ConfigureAwait(false)
            ?? throw new InvalidOperationException(
                $"Extracted JRE at '{javaBinary}' failed version probe.");

        _runtimes[runtime.Id] = runtime;
        await PersistAsync(ct).ConfigureAwait(false);
        await _eventBus.PublishAsync(new JreInstalledEvent(runtime), ct).ConfigureAwait(false);

        return runtime;
    }

    public async ValueTask RemoveRuntimeAsync(string runtimeId, CancellationToken ct = default)
    {
        if (!_runtimes.TryRemove(runtimeId, out var runtime))
            return;

        var managedDir = Path.Combine(_managedJreDir, runtimeId);
        if (Directory.Exists(managedDir))
            Directory.Delete(managedDir, recursive: true);

        await PersistAsync(ct).ConfigureAwait(false);
        await _eventBus.PublishAsync(new JreRemovedEvent(runtimeId), ct).ConfigureAwait(false);
    }

    public async ValueTask<JavaRuntime?> ResolveRuntimeForVersionAsync(
        string minecraftVersion,
        CancellationToken ct = default)
    {
        var required = RequiredJavaMajorVersion(minecraftVersion);
        var runtimes = await GetInstalledRuntimesAsync(ct).ConfigureAwait(false);

        return runtimes
            .Where(r => r.MajorVersion >= required)
            .OrderBy(r => r.MajorVersion)
            .FirstOrDefault();
    }

    private static int RequiredJavaMajorVersion(string minecraftVersion)
    {
        if (!TryParseMinecraftVersion(minecraftVersion, out var major, out var minor))
            return 17;

        return (major, minor) switch
        {
            (1, >= 20) when minor >= 5 => 21,
            (1, >= 17) => 17,
            _ => 8
        };
    }

    private static bool TryParseMinecraftVersion(string version, out int major, out int minor)
    {
        major = 0;
        minor = 0;
        var parts = version.Split('.');
        if (parts.Length < 2) return false;
        return int.TryParse(parts[0], out major) && int.TryParse(parts[1], out minor);
    }

    private static IEnumerable<string> EnumerateJavaCandidates()
    {
        var javaHome = Environment.GetEnvironmentVariable("JAVA_HOME");
        if (!string.IsNullOrEmpty(javaHome))
        {
            var candidate = Path.Combine(javaHome, "bin",
                OperatingSystem.IsWindows() ? "java.exe" : "java");
            if (File.Exists(candidate))
                yield return candidate;
        }

        var pathVar = Environment.GetEnvironmentVariable("PATH") ?? string.Empty;
        var separator = OperatingSystem.IsWindows() ? ';' : ':';
        var javaBin = OperatingSystem.IsWindows() ? "java.exe" : "java";

        foreach (var dir in pathVar.Split(separator, StringSplitOptions.RemoveEmptyEntries))
        {
            var candidate = Path.Combine(dir.Trim(), javaBin);
            if (File.Exists(candidate))
                yield return candidate;
        }

        if (OperatingSystem.IsLinux())
        {
            var sdkmanDir = Path.Combine(
                Environment.GetFolderPath(Environment.SpecialFolder.UserProfile),
                ".sdkman", "candidates", "java");
            if (Directory.Exists(sdkmanDir))
                foreach (var dir in Directory.EnumerateDirectories(sdkmanDir))
                {
                    var candidate = Path.Combine(dir, "bin", "java");
                    if (File.Exists(candidate))
                        yield return candidate;
                }

            foreach (var dir in new[]
            {
                "/usr/lib/jvm",
                "/usr/local/lib/jvm",
                "/opt/java",
                "/opt/jdk"
            })
            {
                if (!Directory.Exists(dir)) continue;
                foreach (var sub in Directory.EnumerateDirectories(dir))
                {
                    var candidate = Path.Combine(sub, "bin", "java");
                    if (File.Exists(candidate))
                        yield return candidate;
                }
            }
        }

        if (OperatingSystem.IsMacOS())
        {
            var libJvmDir = "/Library/Java/JavaVirtualMachines";
            if (Directory.Exists(libJvmDir))
                foreach (var jvm in Directory.EnumerateDirectories(libJvmDir))
                {
                    var candidate = Path.Combine(jvm, "Contents", "Home", "bin", "java");
                    if (File.Exists(candidate))
                        yield return candidate;
                }
        }
    }

    private static async Task<JavaRuntime?> TryProbeJavaAsync(string javaPath, CancellationToken ct)
    {
        try
        {
            using var proc = new Process
            {
                StartInfo = new ProcessStartInfo
                {
                    FileName = javaPath,
                    Arguments = "-version",
                    RedirectStandardError = true,
                    RedirectStandardOutput = true,
                    UseShellExecute = false,
                    CreateNoWindow = true
                }
            };

            proc.Start();

            var vendor = await proc.StandardError.ReadToEndAsync(ct).ConfigureAwait(false);
            await proc.WaitForExitAsync(ct).ConfigureAwait(false);

            if (proc.ExitCode != 0) return null;

            var majorVersion = ParseJavaMajorVersion(vendor);
            if (majorVersion == 0) return null;

            var id = $"custom-{majorVersion}-{Path.GetDirectoryName(javaPath)?.GetHashCode():x8}";
            return new JavaRuntime(id, majorVersion, javaPath, JavaDistribution.Custom,
                vendor.Trim());
        }
        catch
        {
            return null;
        }
    }

    private static int ParseJavaMajorVersion(string versionOutput)
    {
        var match = System.Text.RegularExpressions.Regex.Match(
            versionOutput, @"""(\d+)(?:\.(\d+))?");

        if (!match.Success) return 0;

        var first = int.Parse(match.Groups[1].Value);
        if (first == 1 && match.Groups[2].Success)
            return int.Parse(match.Groups[2].Value);

        return first;
    }

    private async Task<(string Url, string FileName)> ResolveAdoptiumDownloadAsync(
        JreInstallOptions options,
        CancellationToken ct)
    {
        var os = OperatingSystem.IsWindows() ? "windows"
            : OperatingSystem.IsMacOS() ? "mac"
            : "linux";

        var arch = System.Runtime.InteropServices.RuntimeInformation.OSArchitecture switch
        {
            System.Runtime.InteropServices.Architecture.Arm64 => "aarch64",
            _ => "x64"
        };

        var ext = OperatingSystem.IsWindows() ? "zip" : "tar.gz";
        var apiUrl =
            $"https://api.adoptium.net/v3/assets/latest/{options.MajorVersion}/hotspot" +
            $"?architecture={arch}&image_type=jre&jvm_impl=hotspot&os={os}&vendor=eclipse";

        var assets = await _http
            .GetFromJsonAsync<List<AdoptiumAsset>>(apiUrl, ct)
            .ConfigureAwait(false)
            ?? throw new InvalidOperationException(
                $"Adoptium API returned no assets for Java {options.MajorVersion}.");

        var asset = assets.FirstOrDefault()
            ?? throw new InvalidOperationException(
                $"No Adoptium JRE found for Java {options.MajorVersion} ({os}/{arch}).");

        var pkg = asset.Binary.Package;
        return (pkg.Link, Path.GetFileName(new Uri(pkg.Link).LocalPath));
    }

    private async Task DownloadWithProgressAsync(
        string url,
        string destPath,
        IProgress<double>? progress,
        CancellationToken ct)
    {
        using var response = await _http
            .GetAsync(url, HttpCompletionOption.ResponseHeadersRead, ct)
            .ConfigureAwait(false);
        response.EnsureSuccessStatusCode();

        var totalBytes = response.Content.Headers.ContentLength;
        await using var source = await response.Content.ReadAsStreamAsync(ct).ConfigureAwait(false);
        await using var dest = new FileStream(destPath, FileMode.Create, FileAccess.Write,
            FileShare.None, bufferSize: 81920, useAsync: true);

        var buffer = new byte[81920];
        long downloaded = 0;
        int bytesRead;
        while ((bytesRead = await source.ReadAsync(buffer, ct).ConfigureAwait(false)) > 0)
        {
            await dest.WriteAsync(buffer.AsMemory(0, bytesRead), ct).ConfigureAwait(false);
            downloaded += bytesRead;
            if (totalBytes.HasValue)
                progress?.Report((double)downloaded / totalBytes.Value);
        }
        progress?.Report(1.0);
    }

    private static async Task ExtractArchiveAsync(
        string archivePath,
        string destDir,
        CancellationToken ct)
    {
        if (archivePath.EndsWith(".zip", StringComparison.OrdinalIgnoreCase))
        {
            System.IO.Compression.ZipFile.ExtractToDirectory(archivePath, destDir,
                overwriteFiles: true);
        }
        else
        {
            using var proc = new Process
            {
                StartInfo = new ProcessStartInfo
                {
                    FileName = "tar",
                    Arguments = $"-xzf \"{archivePath}\" -C \"{destDir}\" --strip-components=1",
                    UseShellExecute = false,
                    CreateNoWindow = true
                }
            };
            proc.Start();
            await proc.WaitForExitAsync(ct).ConfigureAwait(false);
            if (proc.ExitCode != 0)
                throw new InvalidOperationException(
                    $"tar extraction failed with exit code {proc.ExitCode}.");
        }
    }

    private static string? FindJavaBinaryInDirectory(string dir)
    {
        var javaBin = OperatingSystem.IsWindows() ? "java.exe" : "java";
        return Directory.EnumerateFiles(dir, javaBin, SearchOption.AllDirectories)
            .FirstOrDefault();
    }

    private void LoadPersistedRuntimes()
    {
        if (!File.Exists(_runtimesFile)) return;
        try
        {
            var json = File.ReadAllText(_runtimesFile);
            var list = JsonSerializer.Deserialize<List<JavaRuntime>>(json, JsonOptions);
            if (list is null) return;
            foreach (var r in list)
                _runtimes[r.Id] = r;
        }
        catch
        {
        }
    }

    private async Task PersistAsync(CancellationToken ct)
    {
        await _persistLock.WaitAsync(ct).ConfigureAwait(false);
        try
        {
            Directory.CreateDirectory(_appDataDir);
            var json = JsonSerializer.Serialize(_runtimes.Values.ToList(), JsonOptions);
            await File.WriteAllTextAsync(_runtimesFile, json, ct).ConfigureAwait(false);
        }
        finally
        {
            _persistLock.Release();
        }
    }

    private sealed record AdoptiumAsset(
        [property: JsonPropertyName("binary")] AdoptiumBinary Binary);

    private sealed record AdoptiumBinary(
        [property: JsonPropertyName("package")] AdoptiumPackage Package);

    private sealed record AdoptiumPackage(
        [property: JsonPropertyName("link")] string Link,
        [property: JsonPropertyName("checksum")] string Checksum,
        [property: JsonPropertyName("size")] long Size);
}
