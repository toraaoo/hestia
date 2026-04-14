namespace Hestia.Core.Utils;

public interface IProgressCallback
{
    void OnStart() { }
    /// <param name="progress">Normalized value in range [0.0, 1.0].</param>
    void OnProgress(double progress);
    void OnCompleted() { }
    void OnError(Exception ex) { }
}
