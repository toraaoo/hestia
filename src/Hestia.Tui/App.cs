using Hestia.Core.Minecraft;
using Hestia.Core.Utils;
using Hestia.Tui.Features.Dashboard;
using Hestia.Tui.Features.Samples;
using Hestia.Tui.Features.Splash;
using Hestia.Tui.Input;
using Hestia.Tui.Navigation;
using Microsoft.Extensions.DependencyInjection;
using Spectre.Console;
using Spectre.Console.Cli;

namespace Hestia.Tui;

public class App : AsyncCommand
{
    protected override async Task<int> ExecuteAsync(CommandContext context, CancellationToken cancellationToken)
    {
        try
        {
            var services = new ServiceCollection();

            // Infrastructure
            services.AddSingleton<AppDataFileSystem>();
            services.AddSingleton<Core.Java.Manager>();
            services.AddSingleton<Manager>();
            services.AddSingleton<KeyMap>();

            // Navigation
            services.AddSingleton<Navigator>();
            services.AddSingleton<INavigator>(sp => sp.GetRequiredService<Navigator>());
            services.AddSingleton<ScreenStack>();

            // Screens
            services.AddTransient<DashboardScreen>();
            services.AddTransient<SampleScreen>();
            services.AddTransient<SplashScreen>();

            // Factory for runtime-param screens
            services.AddTransient<Func<string, SampleDetailScreen>>(sp =>
                item => new SampleDetailScreen(item, sp.GetRequiredService<INavigator>()));

            await using var sp = services.BuildServiceProvider();

            var keyMap = sp.GetRequiredService<KeyMap>();
            await keyMap.LoadAsync();

            var stack = sp.GetRequiredService<ScreenStack>();
            var splash = sp.GetRequiredService<SplashScreen>();
            await stack.RunAsync(splash, cancellationToken);

            return 0;
        }
        catch (Exception ex)
        {
            AnsiConsole.WriteException(ex);
            return 1;
        }
    }
}