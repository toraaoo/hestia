using System.Reflection;
using Hestia.Core;
using Hestia.Core.Abstractions;
using Hestia.Core.Events;
using Hestia.Core.Rcon;
using Hestia.Core.Server;
using Microsoft.Extensions.DependencyInjection;

var appInfo = new AppInfoService(
    Assembly.GetEntryAssembly() ?? Assembly.GetExecutingAssembly()).GetAppInfo();

// Include an easy-to-spot stamp so it's obvious which binary is running.
var entry = Assembly.GetEntryAssembly() ?? Assembly.GetExecutingAssembly();
var stamp = "unknown";
try
{
    if (!string.IsNullOrWhiteSpace(entry.Location) && File.Exists(entry.Location))
        stamp = File.GetLastWriteTimeUtc(entry.Location).ToString("yyyyMMdd-HHmmss'Z'");
}
catch { }

var appDataDir = appInfo.AppDataDirectory;
Directory.CreateDirectory(appDataDir);

var services = new ServiceCollection();

services.AddSingleton<IEventBus, EventBus>();
services.AddSingleton<HttpClient>(_ => new HttpClient
{
    DefaultRequestHeaders = { { "User-Agent", $"Hestia/{appInfo.Version}" } }
});
services.AddSingleton<IJreManager>(sp =>
    new global::Hestia.Core.Jre.Manager(appDataDir, sp.GetRequiredService<HttpClient>(),
        sp.GetRequiredService<IEventBus>()));
services.AddSingleton<IServerProvider, global::Hestia.Core.Server.Providers.Vanilla>();
services.AddSingleton<IServerManager>(sp =>
    new global::Hestia.Core.Server.Manager(appDataDir, sp.GetRequiredService<IJreManager>(),
        sp.GetServices<IServerProvider>(), sp.GetRequiredService<IEventBus>()));
services.AddSingleton<IRconService, global::Hestia.Core.Rcon.Service>();
services.AddSingleton<IServerMonitor>(sp =>
    new global::Hestia.Core.Monitoring.Monitor(sp.GetRequiredService<IServerManager>(),
        sp.GetRequiredService<IRconService>(),
        sp.GetRequiredService<IEventBus>()));
services.AddSingleton<IHestiaService>(sp => new HestiaService(
    sp.GetRequiredService<IJreManager>(),
    sp.GetRequiredService<IServerManager>(),
    sp.GetRequiredService<IRconService>(),
    sp.GetRequiredService<IServerMonitor>(),
    sp.GetRequiredService<IEventBus>(),
    sp.GetServices<IServerProvider>()));

await using var provider = services.BuildServiceProvider();


