namespace Hestia.Tui.Input;

internal sealed class KeyMap
{
    private readonly Dictionary<KeyBinding, InputAction> _bindings = new();

    public void Bind(ConsoleKey key, InputAction action) =>
        _bindings[new KeyBinding(key)] = action;

    public void Bind(ConsoleKey key, ConsoleModifiers modifiers, InputAction action) =>
        _bindings[new KeyBinding(key, modifiers)] = action;

    public void Unbind(ConsoleKey key, ConsoleModifiers modifiers = ConsoleModifiers.None) =>
        _bindings.Remove(new KeyBinding(key, modifiers));

    public InputAction? Translate(ConsoleKeyInfo k)
    {
        if (k.KeyChar == '/')
            return InputAction.OpenCommand;

        if (_bindings.TryGetValue(new KeyBinding(k.Key, k.Modifiers), out var exact))
            return exact;

        if (k.Modifiers != ConsoleModifiers.None
            && _bindings.TryGetValue(new KeyBinding(k.Key), out var bare))
            return bare;

        if (!char.IsControl(k.KeyChar))
            return InputAction.TextInput;

        return null;
    }

    public static KeyMap Default()
    {
        var m = new KeyMap();

        m.Bind(ConsoleKey.Q, InputAction.Quit);
        m.Bind(ConsoleKey.Escape, InputAction.Escape);
        m.Bind(ConsoleKey.Enter, InputAction.Confirm);
        m.Bind(ConsoleKey.Tab, InputAction.CycleFocusNext);
        m.Bind(ConsoleKey.Tab, ConsoleModifiers.Shift, InputAction.CycleFocusPrev);

        m.Bind(ConsoleKey.UpArrow, InputAction.CursorUp);
        m.Bind(ConsoleKey.DownArrow, InputAction.CursorDown);
        m.Bind(ConsoleKey.PageUp, InputAction.PageUp);
        m.Bind(ConsoleKey.PageDown, InputAction.PageDown);
        m.Bind(ConsoleKey.LeftArrow, InputAction.TabLeft);
        m.Bind(ConsoleKey.RightArrow, InputAction.TabRight);

        m.Bind(ConsoleKey.M, InputAction.ServerMenu);
        m.Bind(ConsoleKey.C, InputAction.ServerCreate);
        m.Bind(ConsoleKey.F, InputAction.ToggleFollow);

        m.Bind(ConsoleKey.Oem2, InputAction.OpenCommand);
        m.Bind(ConsoleKey.Divide, InputAction.OpenCommand);

        m.Bind(ConsoleKey.P, InputAction.TogglePassword);

        m.Bind(ConsoleKey.Backspace, InputAction.TextBackspace);

        return m;
    }
}