namespace Hestia.Tui.Input;

internal enum InputAction
{
    Quit,
    Escape,
    Confirm,
    CycleFocusNext,
    CycleFocusPrev,

    CursorUp,
    CursorDown,
    PageUp,
    PageDown,
    TabLeft,
    TabRight,

    ServerStart,
    ServerStop,
    ServerRestart,
    ServerToggle,
    ServerDelete,
    ServerCreate,
    ServerMenu,
    ToggleFollow,

    OpenCommand,

    TogglePassword,

    TextBackspace,
    TextInput,
}