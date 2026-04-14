using Hestia.Tui.Screens;
using Spectre.Console;
using Spectre.Console.Rendering;

namespace Hestia.Tui.Views;

internal static class StatusView
{
    public static IRenderable Render(MainViewModel vm)
    {
        var hints = vm.ActivePane switch
        {
            Pane.Command => "[dim]Enter:send  ↑↓:history  Esc:cancel[/]",
            Pane.Logs    => "[dim]←:servers  →:info  ↑↓/PgUp/PgDn:scroll  f:follow  /:command  x:actions  Esc:servers[/]",
            Pane.Info    => "[dim]←:logs  p:pw  x:actions  Esc:servers[/]",
            _            => "[dim]q:quit  ↑↓:nav  Enter:select  s:start/stop  r:restart  x:menu  c:create[/]",
        };

        if (!string.IsNullOrEmpty(vm.StatusMsg))
        {
            var style = vm.StatusIsError ? "bold red" : "bold green";
            return new Markup($"{hints}  [{style}]{Markup.Escape(vm.StatusMsg)}[/]");
        }

        return new Markup(hints);
    }
}
