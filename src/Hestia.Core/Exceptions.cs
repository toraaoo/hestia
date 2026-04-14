namespace Hestia.Core;

public class HestiaException : Exception
{
    public HestiaException(string message) : base(message) { }
    public HestiaException(string message, Exception innerException) : base(message, innerException) { }
}

public class DownloadException : HestiaException
{
    public DownloadException(string message) : base(message) { }
    public DownloadException(string message, Exception innerException) : base(message, innerException) { }
}