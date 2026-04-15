using Hestia.Core.Minecraft;
using Hestia.Core.Minecraft.Models;
using Hestia.Core.Utils;

namespace Hestia.Tui;

public static class Program
{
    private static async Task Main()
    {
        var fs = new AppDataFileSystem();
        var javaManager = new Core.Java.Manager();
        var mcManager = new Manager(javaManager, fs);

        var progress = new ConsoleProgress();

        Console.WriteLine("=== Creating servers ===");

        var server1 = await mcManager.CreateAsync(new Server
        {
            Name = "Alpha",
            Version = "1.21.4",
            Port = 25565,
            RconPort = 25575,
            RconPassword = "alpha-secret",
        }, progress);
        Console.WriteLine($"Created: {server1.Name} ({server1.Version}) [{server1.Id}]");

        var server2 = await mcManager.CreateAsync(new Server
        {
            Name = "Beta",
            Version = "1.21.4",
            Port = 25566,
            RconPort = 25576,
            RconPassword = "beta-secret",
        }, progress);
        Console.WriteLine($"Created: {server2.Name} ({server2.Version}) [{server2.Id}]");

        Console.WriteLine("\n=== Starting servers ===");

        var instance1 = await mcManager.StartAsync(server1.Id);
        Console.WriteLine($"Started: {server1.Name}");

        var instance2 = await mcManager.StartAsync(server2.Id);
        Console.WriteLine($"Started: {server2.Name}");

        // Subscribe to output from both servers
        instance1.Output.Subscribe(line => Console.WriteLine($"[{server1.Name}] {line}"));
        instance2.Output.Subscribe(line => Console.WriteLine($"[{server2.Name}] {line}"));

        // Give servers a moment to start accepting RCON
        Console.WriteLine("\nWaiting for servers to initialize...");
        await Task.Delay(TimeSpan.FromSeconds(15));
        
        var metrics = await instance1.GetMetricsAsync();
        var metrics2 = await instance2.GetMetricsAsync();
        
        
        

        Console.WriteLine("\n=== Sending commands ===");

        var response1 = await instance1.SendCommandAsync("list");
        Console.WriteLine($"[{server1.Name}] list → {response1}");

        var response2 = await instance2.SendCommandAsync("list");
        Console.WriteLine($"[{server2.Name}] list → {response2}");

        Console.WriteLine("\n=== Stopping servers ===");

        await mcManager.StopAsync(server1.Id);
        Console.WriteLine($"Stopped: {server1.Name}");

        await mcManager.StopAsync(server2.Id);
        Console.WriteLine($"Stopped: {server2.Name}");

        Console.WriteLine("\n=== Cleaning up ===");

        await mcManager.DeleteAsync(server1.Id);
        Console.WriteLine($"Deleted: {server1.Name}");

        await mcManager.DeleteAsync(server2.Id);
        Console.WriteLine($"Deleted: {server2.Name}");
    }

    private sealed class ConsoleProgress : IProgressCallback
    {
        public void OnProgress(double progress) =>
            Console.Write($"\r  {progress:P0}   ");

        public void OnCompleted() =>
            Console.WriteLine();
    }
}
