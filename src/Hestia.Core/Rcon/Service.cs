using System.Collections.Concurrent;
using System.Net.Sockets;
using System.Text;
using Hestia.Core.Abstractions;

namespace Hestia.Core.Rcon;

public sealed class Service : IRconService
{
    private const int PacketTypeLogin = 3;
    private const int PacketTypeCommand = 2;
    private const int PacketTypeResponse = 0;
    private const int AuthFailureId = -1;

    private readonly ConcurrentDictionary<Guid, ActiveConnection> _connections = new();

    public async ValueTask<RconConnection> ConnectAsync(
        Guid serverId,
        RconCredentials credentials,
        CancellationToken ct = default)
    {
        var tcp = new TcpClient();
        var timeout = TimeSpan.FromSeconds(5);

        await tcp.ConnectAsync(credentials.Host, credentials.Port, ct).ConfigureAwait(false);
        var stream = tcp.GetStream();

        var requestId = Random.Shared.Next(1, int.MaxValue);
        await SendPacketAsync(stream, requestId, PacketTypeLogin, credentials.Password, ct)
            .ConfigureAwait(false);

        var (respId, respType, _) = await ReceivePacketAsync(stream, ct).ConfigureAwait(false);

        if (respId == AuthFailureId)
        {
            tcp.Dispose();
            throw new UnauthorizedAccessException(
                $"RCON authentication failed for server at {credentials.Host}:{credentials.Port}.");
        }

        var connectionId = Guid.NewGuid();
        var active = new ActiveConnection(tcp, stream, credentials, serverId, requestId);
        _connections[connectionId] = active;

        return new RconConnection(connectionId, serverId, credentials, IsConnected: true);
    }

    public ValueTask DisconnectAsync(Guid connectionId, CancellationToken ct = default)
    {
        if (_connections.TryRemove(connectionId, out var conn))
            conn.Dispose();

        return ValueTask.CompletedTask;
    }

    public async ValueTask<RconResponse> SendCommandAsync(
        Guid connectionId,
        string command,
        CancellationToken ct = default)
    {
        if (!_connections.TryGetValue(connectionId, out var conn))
            throw new InvalidOperationException($"RCON connection '{connectionId}' is not open.");

        var requestId = Interlocked.Increment(ref conn.NextRequestId);

        await conn.Lock.WaitAsync(ct).ConfigureAwait(false);
        try
        {
            await SendPacketAsync(conn.Stream, requestId, PacketTypeCommand, command, ct)
                .ConfigureAwait(false);

            var terminatorId = requestId + 1;
            await SendPacketAsync(conn.Stream, terminatorId, PacketTypeCommand, "", ct)
                .ConfigureAwait(false);

            var payload = new StringBuilder();
            while (true)
            {
                var (respId, _, body) = await ReceivePacketAsync(conn.Stream, ct)
                    .ConfigureAwait(false);

                if (respId == terminatorId)
                    break;

                payload.Append(body);
            }

            return new RconResponse(requestId.ToString(), payload.ToString(), IsError: false);
        }
        finally
        {
            conn.Lock.Release();
        }
    }

    public bool IsConnected(Guid connectionId)
        => _connections.TryGetValue(connectionId, out var conn) && conn.Tcp.Connected;

    private static async Task SendPacketAsync(
        NetworkStream stream,
        int requestId,
        int type,
        string payload,
        CancellationToken ct)
    {
        var payloadBytes = Encoding.UTF8.GetBytes(payload);
        var length = 4 + 4 + payloadBytes.Length + 2;
        var packet = new byte[4 + length];

        WriteInt32LE(packet, 0, length);
        WriteInt32LE(packet, 4, requestId);
        WriteInt32LE(packet, 8, type);
        payloadBytes.CopyTo(packet, 12);

        await stream.WriteAsync(packet, ct).ConfigureAwait(false);
    }

    private static async Task<(int RequestId, int Type, string Payload)> ReceivePacketAsync(
        NetworkStream stream,
        CancellationToken ct)
    {
        var header = new byte[12];
        await ReadExactAsync(stream, header, ct).ConfigureAwait(false);

        var length = ReadInt32LE(header, 0);
        var requestId = ReadInt32LE(header, 4);
        var type = ReadInt32LE(header, 8);

        var bodyLength = length - 8;
        var body = new byte[bodyLength];
        await ReadExactAsync(stream, body, ct).ConfigureAwait(false);

        var payloadLength = Math.Max(0, bodyLength - 2);
        var payload = Encoding.UTF8.GetString(body, 0, payloadLength);

        return (requestId, type, payload);
    }

    private static async Task ReadExactAsync(
        NetworkStream stream,
        byte[] buffer,
        CancellationToken ct)
    {
        var offset = 0;
        while (offset < buffer.Length)
        {
            var read = await stream.ReadAsync(buffer.AsMemory(offset), ct).ConfigureAwait(false);
            if (read == 0)
                throw new IOException("RCON connection closed by remote.");
            offset += read;
        }
    }

    private static void WriteInt32LE(byte[] buf, int offset, int value)
    {
        buf[offset] = (byte)value;
        buf[offset + 1] = (byte)(value >> 8);
        buf[offset + 2] = (byte)(value >> 16);
        buf[offset + 3] = (byte)(value >> 24);
    }

    private static int ReadInt32LE(byte[] buf, int offset)
        => buf[offset] | (buf[offset + 1] << 8) | (buf[offset + 2] << 16) | (buf[offset + 3] << 24);

    private sealed class ActiveConnection(
        TcpClient tcp,
        NetworkStream stream,
        RconCredentials credentials,
        Guid serverId,
        int initialRequestId) : IDisposable
    {
        public TcpClient Tcp => tcp;
        public NetworkStream Stream => stream;
        public RconCredentials Credentials => credentials;
        public Guid ServerId => serverId;
        public SemaphoreSlim Lock { get; } = new(1, 1);
        public int NextRequestId = initialRequestId;

        public void Dispose()
        {
            Lock.Dispose();
            stream.Dispose();
            tcp.Dispose();
        }
    }
}
