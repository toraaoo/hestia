using Hestia.Core;
using Hestia.Core.Abstractions;
using Hestia.Tui.Input;
using Hestia.Tui.Screens;
using Hestia.Tui.Screens.Modals;
using Hestia.Tui.Services;
using Hestia.Tui.ViewModels;
using Hestia.Tui.Views;
using Spectre.Console;
using static Hestia.Tui.Utilities.TerminalUtils;

namespace Hestia.Tui.App;

internal sealed class TuiApp
{
    private readonly IHestiaService _service;
    private readonly AppInfo _appInfo;
    private readonly string _stamp;
    private readonly UiDispatcher _ui = new();
    private readonly KeyMap _keyMap = KeyMap.Default();
    private readonly MainLayoutView _mainLayoutView = new();

    private MainPresenter _presenter = null!;

    private ServerListVm _serverListVm = null!;
    private JreListVm _jreListVm = null!;
    private CommandVm _commandVm = null!;

    private bool _quit;

    private readonly CancellationTokenSource _appCts = new();

    public TuiApp(IHestiaService service, AppInfo appInfo, string stamp)
    {
        _service = service;
        _appInfo = appInfo;
        _stamp = stamp;
    }

    public async Task RunAsync()
    {
        _serverListVm = new ServerListVm(_service);
        _jreListVm = new JreListVm(_service);
        _commandVm = new CommandVm(_service);

        _presenter = new MainPresenter(
            _service,
            _appInfo,
            _stamp,
            _ui,
            _keyMap,
            _serverListVm,
            _jreListVm,
            _commandVm);

        _commandVm.LineAppended += line => _presenter.Ui.Post(() => _presenter.AppendRconOutputToLogs(line));
        _commandVm.StatusChanged += msg => _presenter.Ui.Post(() => _presenter.SetStatus(msg, true));

        Console.CursorVisible = false;
        await _presenter.StartAsync(_appCts.Token);

        while (!_quit)
        {
            await RunLiveAsync();

            var modal = _presenter.TryDequeueModal();
            if (modal is null)
                continue;

            await RunModalAsync(modal);
        }

        Console.CursorVisible = true;
        _appCts.Cancel();
        await _presenter.DisposeAsync();
    }

    private async Task RunLiveAsync()
    {
        while (!_quit && !_presenter.HasPendingModal && TooSmall())
        {
            AnsiConsole.Clear();
            Console.WriteLine(
                $"Terminal too small. Resize to at least {MinWidth}×{MinHeight} (now {Console.WindowWidth}×{Console.WindowHeight}).");
            await Task.Delay(300);
        }

        if (_quit || _presenter.HasPendingModal) return;

        var layout = _mainLayoutView.BuildLayout(_presenter.Snapshot());
        try
        {
            await AnsiConsole.Live(layout)
                .AutoClear(false)
                .Overflow(VerticalOverflow.Ellipsis)
                .Cropping(VerticalOverflowCropping.Bottom)
                .StartAsync(async ctx =>
                {
                    while (!_quit && !_presenter.HasPendingModal)
                    {
                        if (TooSmall()) return;
                        _ui.Drain();
                        HandleInput();
                        try
                        {
                            _mainLayoutView.UpdateLayout(layout, _presenter.Snapshot());
                            ctx.Refresh();
                        }
                        catch
                        {
                            return;
                        }

                        await Task.Delay(50);
                    }
                });
        }
        catch (OperationCanceledException) { }
        catch
        {
            // ignored
        }
    }

    private async Task RunModalAsync(ModalRequest req)
    {
        switch (req)
        {
            case ServerMenuModalRequest m:
            {
                var result = await ServerMenuModal.RunAsync(m.Server, _keyMap, _appCts.Token);
                await _presenter.HandleModalResultAsync(result, _appCts.Token);
                break;
            }
            case DeleteModalRequest d:
            {
                var result = await DeleteConfirmModal.RunAsync(d.ServerName, _appCts.Token);
                await _presenter.HandleModalResultAsync(result, _appCts.Token);
                break;
            }
            case CreateModalRequest c:
            {
                var result = await CreateWizardModal.RunAsync(c.Form, c.VersionsByType, _keyMap, _appCts.Token);
                await _presenter.HandleModalResultAsync(result, _appCts.Token);
                break;
            }
            case ProgressModalRequest p:
            {
                await ProgressModal.RunAsync(p.State, _appCts.Token);
                await _presenter.HandleModalResultAsync(new ProgressModalResult(), _appCts.Token);
                break;
            }
        }
    }

    private void HandleInput()
    {
        while (Console.KeyAvailable)
        {
            var key = Console.ReadKey(intercept: true);
            if (key.Key == ConsoleKey.Q)
            {
                _quit = true;
                continue;
            }

            _presenter.OnKey(key);
        }
    }

}
