namespace Hestia.Core.Rcon;

public sealed record RconCredentials(string Host, int Port, string Password);

public sealed record RconConnection(
    Guid Id,
    Guid ServerId,
    RconCredentials Credentials,
    bool IsConnected);

public sealed record RconResponse(string RequestId, string Payload, bool IsError);
