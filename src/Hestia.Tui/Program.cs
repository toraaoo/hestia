using Hestia.Tui;
using Spectre.Console.Cli;

var app = new CommandApp<App>();

return await app.RunAsync(args);