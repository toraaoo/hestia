using Hestia.Tui.Input;
using Hestia.Tui.Services;

namespace Hestia.Tui.Screens.Modals;

internal abstract record ModalResult;

internal sealed record CreateModalResult(ServerCreateForm? Form) : ModalResult;

internal sealed record ServerMenuModalResult(InputAction? Action) : ModalResult;

internal sealed record DeleteModalResult(bool Confirmed) : ModalResult;

internal sealed record ProgressModalResult : ModalResult;
