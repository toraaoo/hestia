namespace Hestia.Tui.Input;

public record KeyBinding(ConsoleKey Key, ConsoleModifiers Modifiers = ConsoleModifiers.None)
{
    public bool Matches(ConsoleKeyInfo key) =>
        key.Key == Key && key.Modifiers == Modifiers;
}
