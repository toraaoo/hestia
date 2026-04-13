using Hestia.Tui.Formatting;
using Hestia.Tui.Screens;
using Spectre.Console;
using Spectre.Console.Rendering;

namespace Hestia.Tui.Views;

internal static class HeaderView
{
    public static IRenderable Render(MainViewModel vm)
    {
        var content = new Markup(
            $"[bold green]{AsciiArt.Header}[/]\n[dim]{Markup.Escape(AsciiArt.Stamp(vm.AppVersion, vm.Stamp))}[/]");
        return new Panel(content) { Border = BoxBorder.None, Expand = true };
    }
}
