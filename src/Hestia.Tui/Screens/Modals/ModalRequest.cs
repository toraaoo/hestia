using Hestia.Core.Server;
using Hestia.Tui.Services;

namespace Hestia.Tui.Screens.Modals;

internal abstract record ModalRequest;

internal sealed record CreateModalRequest(
    ServerCreateForm Form,
    IReadOnlyDictionary<ServerType, IReadOnlyList<string>> VersionsByType) : ModalRequest;

internal sealed record ServerMenuModalRequest(Guid ServerId, MinecraftServer Server) : ModalRequest;

internal sealed record DeleteModalRequest(Guid ServerId, string ServerName) : ModalRequest;

internal sealed record ProgressModalRequest(ProgressState State) : ModalRequest;
