using Hestia.Tui.Features.Dashboard;
using Hestia.Tui.Input;
using Hestia.Tui.Navigation;
using Hestia.Tui.Utils;
using Spectre.Console;
using Spectre.Console.Rendering;

namespace Hestia.Tui.Features.Splash;

public sealed class SplashScreen(INavigator navigator, DashboardScreen dashboard) : ScreenBase
{
    private bool _ready;

    public override Task LoadAsync(CancellationToken ct)
    {
        _ = Task.Delay(1500, ct).ContinueWith(_ => _ready = true, TaskScheduler.Default);
        return Task.CompletedTask;
    }

    public override IRenderable Render()
    {
        if (_ready)
            navigator.Push(dashboard);

        var splash = new Align(
            new Rows(
                new Markup($"[bold green]{Ascii.Header}[/]"),
                new Markup(""),
                new Markup("[dim] Minecraft Server Manager[/]"),
                new Markup(""),
                new Markup("[dim] Loading…[/]")
            ),
            HorizontalAlignment.Center,
            VerticalAlignment.Middle
        );

        var root = new Layout("Root").SplitRows(
            new Layout("Splash"),
            new Layout("Footer").Size(1)
        );

        root["Splash"].Update(splash);
        root["Footer"].Update(Align.Center(new Markup("[dim] Press any key to continue[/]")));

        return root;
    }

    public override void OnInput(InputAction action)
    {
        _ready = true;
    }
}