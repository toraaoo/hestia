namespace Hestia.Tui.Input;

internal readonly record struct KeyBinding(
    ConsoleKey Key,
    ConsoleModifiers Modifiers = ConsoleModifiers.None);
