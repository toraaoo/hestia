using Hestia.Core.Monitoring;
using Hestia.Core.Server;

namespace Hestia.Tui.Screens;

internal sealed record MainViewModel(
    IReadOnlyList<MinecraftServer> Servers,
    IReadOnlyList<string>          JreRows,
    Guid?                          SelectedServerId,
    MinecraftServer?               SelectedServer,
    int                            ServerCursor,
    Pane                           ActivePane,
    Tab                            ActiveTab,
    int                            LogScroll,
    bool                           LogFollow,
    string                         InputBuffer,
    string                         StatusMsg,
    bool                           StatusIsError,
    bool                           ShowRconPassword,
    IReadOnlyList<string>          LogLines,
    string                         AppVersion,
    string                         Stamp,
    ServerStatus?                  LatestStatus
);
