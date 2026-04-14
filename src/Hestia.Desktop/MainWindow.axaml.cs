using Avalonia.Controls;
using System.Reflection;

namespace Hestia.Desktop;

public partial class MainWindow : Window
{
    public MainWindow()
    {
        InitializeComponent();

        VersionText.Text = $"Version: 1.0.0"; // appInfo.Version.ToString();
        AppDataText.Text = $"App Data Path: {Environment.GetFolderPath(Environment.SpecialFolder.ApplicationData)}";
    }
}