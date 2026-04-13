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
    ServerDelete,
    ServerCreate,
    ToggleFollow,

    OpenCommand,
    CommandHistoryUp,
    CommandHistoryDown,

    TogglePassword,

    TextBackspace,
    TextInput,
}