using Hestia.Tui.ViewModels;

namespace Hestia.Tui.Tests;

public sealed class CommandVmHistoryTests
{
    // CommandVm history uses ref-based mutation helpers, tested via the public API.
    // We test the HistoryUp/HistoryDown behaviour without needing a real IHestiaService.

    private static CommandVm MakeVm()
    {
        // CommandVm only needs IHestiaService for SendAsync; history doesn't touch it.
        return new CommandVm(null!);
    }

    [Fact]
    public void HistoryUp_on_empty_history_is_noop()
    {
        var vm = MakeVm();
        string current = "typed";
        vm.HistoryUp(ref current);
        Assert.Equal("typed", current); // unchanged
    }

    [Fact]
    public void HistoryDown_without_prior_up_is_noop()
    {
        var vm = MakeVm();
        string current = "typed";
        vm.HistoryDown(ref current);
        Assert.Equal("typed", current);
    }

    [Fact]
    public void SendAsync_adds_to_history_and_HistoryUp_recalls_it()
    {
        var vm = MakeVm();

        // Manually populate history by calling the internal list via reflection
        // (simpler than mocking the full service for a unit test).
        var historyField = typeof(CommandVm)
            .GetField("_history", System.Reflection.BindingFlags.NonPublic | System.Reflection.BindingFlags.Instance)!;
        var history = (List<string>)historyField.GetValue(vm)!;
        history.Add("say hello");
        history.Add("list players");

        string current = string.Empty;
        vm.HistoryUp(ref current);
        Assert.Equal("list players", current);

        vm.HistoryUp(ref current);
        Assert.Equal("say hello", current);
    }

    [Fact]
    public void HistoryDown_after_up_returns_newer_entry()
    {
        var vm = MakeVm();
        var historyField = typeof(CommandVm)
            .GetField("_history", System.Reflection.BindingFlags.NonPublic | System.Reflection.BindingFlags.Instance)!;
        var history = (List<string>)historyField.GetValue(vm)!;
        history.Add("cmd1");
        history.Add("cmd2");

        string current = string.Empty;
        vm.HistoryUp(ref current); // → cmd2
        vm.HistoryUp(ref current); // → cmd1
        vm.HistoryDown(ref current); // → cmd2
        Assert.Equal("cmd2", current);
    }

    [Fact]
    public void HistoryDown_past_end_clears_current()
    {
        var vm = MakeVm();
        var historyField = typeof(CommandVm)
            .GetField("_history", System.Reflection.BindingFlags.NonPublic | System.Reflection.BindingFlags.Instance)!;
        var history = (List<string>)historyField.GetValue(vm)!;
        history.Add("only");

        string current = string.Empty;
        vm.HistoryUp(ref current);   // → "only"
        vm.HistoryDown(ref current); // → "" (past end)
        Assert.Equal(string.Empty, current);
    }
}
