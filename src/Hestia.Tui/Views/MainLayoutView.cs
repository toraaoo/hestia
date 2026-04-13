using Hestia.Tui.Screens;
using Spectre.Console;
using Spectre.Console.Rendering;
using static Hestia.Tui.Utilities.TerminalUtils;

namespace Hestia.Tui.Views;

internal sealed class MainLayoutView
{
    private Layout? _rightLogsCmdLayout;

    public Layout BuildLayout(MainViewModel vm)
    {
        var root = new Layout("Root")
            .SplitRows(
                new Layout("Content"),
                new Layout("Status").Size(1));

        root["Content"].SplitColumns(
            new Layout("Left").Size(ComputeLeftWidth(Console.WindowWidth)),
            new Layout("Right"));

        root["Left"].SplitRows(
            new Layout("Header").Size(HeaderH),
            new Layout("Servers"),
            new Layout("JRE").Size(JreH));

        root["Header"].Update(HeaderView.Render(vm));
        root["Servers"].Update(ServerListView.Render(vm));
        root["JRE"].Update(JreListView.Render(vm));
        UpdateRight(root["Right"], vm);
        root["Status"].Update(StatusView.Render(vm));

        return root;
    }

    public void UpdateLayout(Layout layout, MainViewModel vm)
    {
        layout["Left"].Size(ComputeLeftWidth(Console.WindowWidth));
        layout["Header"].Update(HeaderView.Render(vm));
        layout["Servers"].Update(ServerListView.Render(vm));
        layout["JRE"].Update(JreListView.Render(vm));
        UpdateRight(layout["Right"], vm);
        layout["Status"].Update(StatusView.Render(vm));
    }

    private void UpdateRight(Layout right, MainViewModel vm)
    {
        if (vm.SelectedServerId is null)
        {
            right.Update(new Panel(new Markup("[dim]Press [bold]Enter[/] to select a server.[/]"))
            {
                Header = new PanelHeader("No selection"),
                Border = BoxBorder.Rounded,
                Expand = true,
            });
            return;
        }

        if (vm.ActiveTab == Tab.Info)
        {
            right.Update(InfoView.Render(vm));
            return;
        }

        _rightLogsCmdLayout ??= new Layout("RightLogsCmd")
            .SplitRows(
                new Layout("Logs"),
                new Layout("Command").Size(3));

        _rightLogsCmdLayout["Logs"].Update(LogsView.Render(vm));
        _rightLogsCmdLayout["Command"].Update(CommandView.Render(vm));
        right.Update(_rightLogsCmdLayout);
    }
}
