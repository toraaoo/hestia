using System.Net.Sockets;
using System.Text;

namespace Hestia.Core.Minecraft.Rcon;

public sealed class RconClient : IDisposable
{
    private const int AuthPacketType = 3;
    private const int CommandPacketType = 2;
    private const int AuthFailedId = -1;

    private TcpClient? _tcp;
    private NetworkStream? _stream;
    private int _requestId;

    public async Task ConnectAsync(string host, int port, string password)
    {
        _tcp = new TcpClient();
        await _tcp.ConnectAsync(host, port);
        _stream = _tcp.GetStream();

        var authId = NextId();
        await SendPacketAsync(authId, AuthPacketType, password);
        var response = await ReadPacketAsync();

        if (response.RequestId == AuthFailedId)
            throw new HestiaException("RCON authentication failed — check the RCON password.");
    }

    public async Task<string> SendCommandAsync(string command)
    {
        EnsureConnected();

        var id = NextId();
        await SendPacketAsync(id, CommandPacketType, command);
        var response = await ReadPacketAsync();
        return response.Payload;
    }

    public void Dispose()
    {
        _stream?.Dispose();
        _tcp?.Dispose();
    }

    private void EnsureConnected()
    {
        if (_stream is null || _tcp is null || !_tcp.Connected)
            throw new InvalidOperationException("RCON client is not connected.");
    }

    private int NextId() => ++_requestId;

    private async Task SendPacketAsync(int id, int type, string payload)
    {
        var body = Encoding.UTF8.GetBytes(payload);

        var length = 4 + 4 + body.Length + 2;

        using var ms = new MemoryStream();
        ms.Write(BitConverter.GetBytes(length));
        ms.Write(BitConverter.GetBytes(id));
        ms.Write(BitConverter.GetBytes(type));
        ms.Write(body);
        ms.Write([0, 0]);

        var data = ms.ToArray();
        await _stream!.WriteAsync(data);
    }

    private async Task<RconPacket> ReadPacketAsync()
    {
        var lengthBuf = new byte[4];
        await ReadExactAsync(lengthBuf);
        var length = BitConverter.ToInt32(lengthBuf);

        var data = new byte[length];
        await ReadExactAsync(data);

        var requestId = BitConverter.ToInt32(data, 0);
        var type = BitConverter.ToInt32(data, 4);

        var payload = Encoding.UTF8.GetString(data, 8, length - 8 - 2);

        return new RconPacket(requestId, type, payload);
    }

    private async Task ReadExactAsync(byte[] buffer)
    {
        var offset = 0;
        while (offset < buffer.Length)
        {
            var read = await _stream!.ReadAsync(buffer.AsMemory(offset));
            if (read == 0)
                throw new HestiaException("RCON connection closed unexpectedly.");
            offset += read;
        }
    }

    private record RconPacket(int RequestId, int Type, string Payload);
}
