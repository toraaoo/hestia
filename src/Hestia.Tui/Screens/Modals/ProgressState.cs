namespace Hestia.Tui.Screens.Modals;

internal sealed class ProgressState
{
    public string ServerName { get; set; } = "";
    public string Version { get; set; } = "";
    public string Type { get; set; } = "";
    public double Progress { get; set; }
    public string StatusMsg { get; set; } = "";
    public bool IsComplete { get; set; }
    public bool HasError { get; set; }
}
