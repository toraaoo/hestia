using Avalonia.Controls;
using System.Reflection;
using Hestia.Core;

namespace Hestia.Desktop;

public partial class MainWindow : Window
{
    public MainWindow()
    {
        InitializeComponent();

        var hostAssembly = Assembly.GetEntryAssembly() ?? Assembly.GetExecutingAssembly();
        var appInfo = new AppInfoService(hostAssembly).GetAppInfo();

        VersionText.Text = $"Version: {appInfo.Version}";
        AppDataText.Text = appInfo.AppDataDirectory;
    }
}
