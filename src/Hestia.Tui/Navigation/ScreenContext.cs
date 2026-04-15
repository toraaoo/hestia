using Hestia.Tui.Input;

namespace Hestia.Tui.Navigation;

public static class ScreenContext
{
    private static readonly AsyncLocal<IScreenHost?> _host = new();
    private static readonly AsyncLocal<KeyMap?> _keyMap = new();

    public static IScreenHost Host =>
        _host.Value ?? throw new InvalidOperationException("No ScreenContext is active.");

    public static KeyMap KeyMap =>
        _keyMap.Value ?? throw new InvalidOperationException("No ScreenContext is active.");

    internal static void Set(IScreenHost host, KeyMap keyMap)
    {
        _host.Value = host;
        _keyMap.Value = keyMap;
    }
}
