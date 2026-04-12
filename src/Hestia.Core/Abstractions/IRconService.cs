using Hestia.Core.Rcon;

namespace Hestia.Core.Abstractions;

public interface IRconService
{
    ValueTask<RconConnection> ConnectAsync(
        Guid serverId,
        RconCredentials credentials,
        CancellationToken ct = default);

    ValueTask DisconnectAsync(Guid connectionId, CancellationToken ct = default);

    ValueTask<RconResponse> SendCommandAsync(
        Guid connectionId,
        string command,
        CancellationToken ct = default);

    bool IsConnected(Guid connectionId);
}
